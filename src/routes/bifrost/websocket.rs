use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use tokio::select;

use bifrost_api::backend::BackendRequest;
use bifrost_api::service::Service;
use bifrost_api::websocket::Update;
use hue::event::EventBlock;
use svc::manager::{ServiceEvent, SvmClient};

use crate::routes::bifrost::BifrostApiResult;
use crate::server::appstate::AppState;
use crate::server::hueevents::HueEventRecord;

struct WebSocketTask {
    state: AppState,
    ws: WebSocket,
    mgr: SvmClient,
}

#[allow(clippy::unnecessary_wraps, clippy::unused_self)]
impl WebSocketTask {
    pub fn new(state: AppState, ws: WebSocket) -> Self {
        let mgr = state.manager();

        Self { state, ws, mgr }
    }

    async fn send(&mut self, value: Update) -> BifrostApiResult<()> {
        let text = serde_json::to_string(&value)?.into();

        self.ws.send(Message::Text(text)).await?;

        Ok(())
    }

    fn handle_websocket_message(&self, msg: &Message) -> BifrostApiResult<Option<Update>> {
        log::trace!("Websocket message: {msg:?}");
        Ok(None)
    }

    fn handle_backend_event(
        &self,
        backend_event: &Arc<BackendRequest>,
    ) -> BifrostApiResult<Option<Update>> {
        log::info!("Backend event: {backend_event:?}");
        Ok(Some(Update::BackendRequest((**backend_event).clone())))
    }

    fn handle_hue_event(&self, hue_event: HueEventRecord) -> BifrostApiResult<Option<Update>> {
        log::info!("Hue event: {hue_event:?}");
        Ok(Some(Update::HueEvent(hue_event.block)))
    }

    async fn handle_service_event(
        &mut self,
        service_event: Option<ServiceEvent>,
    ) -> BifrostApiResult<Option<Update>> {
        let Some(service_event) = service_event else {
            log::error!("service event channel broke");
            panic!();
        };

        log::trace!("service event: {service_event:?}");

        let name = self.mgr.lookup_name(service_event.id()).await?;

        let service = Service {
            id: service_event.id(),
            name,
            state: service_event.state(),
        };

        Ok(Some(Update::ServiceUpdate(service)))
    }

    async fn handle_socket(mut self) -> BifrostApiResult<()> {
        let lock = self.state.res.lock().await;
        let mut backend_events = lock.backend_event_stream();
        let mut hue_events = lock.hue_event_stream().subscribe();
        let hue_state = lock.get_resources();
        drop(lock);

        let mut svc_events = self.mgr.subscribe().await?.1;

        let app_config = self.state.config();
        self.send(Update::AppConfig((*app_config).clone())).await?;

        self.send(Update::HueEvent(EventBlock::add(hue_state)))
            .await?;

        loop {
            let reply = select! {
                msg = self.ws.recv() => match msg {
                    Some(msg) => self.handle_websocket_message(&msg?),
                    None => return Ok(()),
                },
                backend_event = backend_events.recv() => self.handle_backend_event(&backend_event?),
                service_event = svc_events.recv() => self.handle_service_event(service_event).await,
                hue_event = hue_events.recv() => self.handle_hue_event(hue_event?),
            };

            if let Some(reply) = reply? {
                self.send(reply).await?;
            }
        }
    }
}

pub async fn websocket(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|ws| async move {
        let wst = WebSocketTask::new(state, ws);

        match wst.handle_socket().await {
            Ok(()) => {
                log::info!("Websocket closed.");
            }
            Err(err) => {
                log::error!("Websocket error: {err:?}");
            }
        }
    })
}
