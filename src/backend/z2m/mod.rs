mod backend_event;
mod bridge_event;
mod bridge_import;
pub mod entertainment;
pub mod learn;
pub mod websocket;
pub mod zclcommand;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use native_tls::TlsConnector;
use svc::error::SvcError;
use svc::template::ServiceTemplate;
use svc::traits::{BoxDynService, Service};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
};

use bifrost_api::backend::BackendRequest;
use hue::api::ResourceLink;
use z2m::update::DeviceUpdate;

use crate::backend::z2m::entertainment::EntStream;
use crate::backend::z2m::learn::SceneLearn;
use crate::backend::z2m::websocket::Z2mWebSocket;
use crate::config::{AppConfig, Z2mServer};
use crate::error::{ApiError, ApiResult};
use crate::model::throttle::Throttle;
use crate::resource::Resources;
use crate::server::appstate::AppState;

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("No config found for z2m server {0:?}")]
    NotFound(String),
}

pub struct Z2mServiceTemplate {
    state: AppState,
}

impl Z2mServiceTemplate {
    #[must_use]
    pub const fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl ServiceTemplate for Z2mServiceTemplate {
    fn generate(&self, name: String) -> Result<BoxDynService, SvcError> {
        let config = self.state.config();
        let Some(server) = config.z2m.servers.get(&name) else {
            return Err(SvcError::generation(TemplateError::NotFound(name)));
        };
        let svc = Z2mBackend::new(name, server.clone(), config, self.state.res.clone())
            .map_err(SvcError::generation)?;

        Ok(svc.boxed())
    }
}

pub struct Z2mBackend {
    name: String,
    server: Z2mServer,
    config: Arc<AppConfig>,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, ResourceLink>,
    rmap: HashMap<ResourceLink, String>,
    learner: SceneLearn,
    ignore: HashSet<String>,
    network: HashMap<String, z2m::api::Device>,
    entstream: Option<EntStream>,
    counter: u32,
    fps: u32,
    throttle: Throttle,
    socket: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,

    // for sending delayed messages over the websocket
    message_rx: mpsc::UnboundedReceiver<(String, DeviceUpdate)>,
    message_tx: mpsc::UnboundedSender<(String, DeviceUpdate)>,
}

impl Z2mBackend {
    const DEFAULT_FPS: u32 = 20;
    const LIGHT_BREATHE_DURATION: Duration = Duration::from_secs(2);

    pub fn new(
        name: String,
        server: Z2mServer,
        config: Arc<AppConfig>,
        state: Arc<Mutex<Resources>>,
    ) -> ApiResult<Self> {
        let fps = server.streaming_fps.map_or(Self::DEFAULT_FPS, u32::from);
        let map = HashMap::new();
        let rmap = HashMap::new();
        let ignore = HashSet::new();
        let learner = SceneLearn::new(name.clone());
        let network = HashMap::new();
        let entstream = None;
        let throttle = Throttle::from_fps(fps);
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        Ok(Self {
            name,
            server,
            config,
            state,
            map,
            rmap,
            learner,
            ignore,
            network,
            entstream,
            throttle,
            fps,
            message_rx,
            message_tx,
            socket: None,
            counter: 0,
        })
    }

    pub async fn event_loop(
        &mut self,
        chan: &mut Receiver<Arc<BackendRequest>>,
        mut socket: Z2mWebSocket,
    ) -> ApiResult<()> {
        loop {
            select! {
                // all backend event handling implemented in backend::z2m::backend_event
                pkt = chan.recv() => {
                    let api_req = pkt?;
                    self.handle_backend_event(&mut socket, api_req).await?;
                    // FIXME: this used to be our "throttle" feature, but it breaks entertainment mode
                    /* tokio::time::sleep(std::time::Duration::from_millis(100)).await; */
                },

                // all bridge event handling implemented in backend::z2m::bridge_event
                pkt = socket.next() => {
                    self.handle_bridge_event(pkt.ok_or(ApiError::UnexpectedZ2mEof)??).await?;
                },

                Some((topic, upd)) = self.message_rx.recv() => {
                    socket.send_update(&topic, &upd).await?;
                }
            };
        }
    }
}

#[async_trait]
impl Service for Z2mBackend {
    type Error = ApiError;

    async fn start(&mut self) -> ApiResult<()> {
        // let's not include auth tokens in log output
        let sanitized_url = self.server.get_sanitized_url();
        let url = self.server.get_url();

        if url != self.server.url {
            log::info!(
                "[{}] Rewrote url for compatibility with z2m 2.x.",
                self.name
            );
            log::info!(
                "[{}] Consider updating websocket url to {}",
                self.name,
                sanitized_url
            );
        }
        Ok(())
    }

    async fn run(&mut self) -> ApiResult<()> {
        let sanitized_url = self.server.get_sanitized_url();
        let url = self.server.get_url();
        // if tls verification is disabled, build a TlsConnector that explicitly
        // does not check certificate validity. This is obviously neither safe
        // nor recommended.
        let connector = if self.server.disable_tls_verify.unwrap_or_default() {
            log::warn!(
                "[{}] TLS verification disabled; will accept any certificate!",
                self.name
            );
            Some(Connector::NativeTls(
                TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()?,
            ))
        } else {
            None
        };

        log::info!("[{}] Connecting to {}", self.name, &sanitized_url);
        let socket =
            match connect_async_tls_with_config(url.as_str(), None, false, connector.clone()).await
            {
                Ok((socket, _)) => socket,
                Err(err) => {
                    log::error!("[{}] Connect failed: {err:?}", self.name);
                    return Err(err.into());
                }
            };

        let z2m_socket = Z2mWebSocket::new(self.name.clone(), socket);
        let mut chan = self.state.lock().await.backend_event_stream();
        self.event_loop(&mut chan, z2m_socket).await
    }

    async fn stop(&mut self) -> ApiResult<()> {
        self.socket.take();
        Ok(())
    }
}
