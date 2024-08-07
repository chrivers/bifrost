use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

use chrono::Utc;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use mac_address::MacAddress;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::config::{AppConfig, MqttConfig, Z2mConfig};
use crate::error::ApiResult;
use crate::hue::v1::{ApiConfig, ApiShortConfig, Whitelist};
use crate::resource::Resources;

#[derive(Clone)]
pub struct AppState {
    conf: AppConfig,
    pub res: Arc<Mutex<Resources>>,
    pub ws: Arc<
        Mutex<
            SplitSink<
                WebSocketStream<MaybeTlsStream<TcpStream>>,
                tokio_tungstenite::tungstenite::Message,
            >,
        >,
    >,
}

impl AppState {
    pub async fn new(conf: AppConfig) -> ApiResult<Self> {
        /* FIXME: just for proof of concept */
        let first_z2m_server = &conf.z2m.servers.values().next().unwrap().url.clone();

        let ws = connect_async(first_z2m_server).await?.0;
        let (ws_sink, mut ws_stream) = ws.split();
        tokio::spawn(async move {
            loop {
                let _ = ws_stream.next().await;
            }
        });

        let res = Arc::new(Mutex::new(Resources::new()));
        let ws = Arc::new(Mutex::new(ws_sink));

        Ok(Self { conf, res, ws })
    }

    #[must_use]
    pub const fn mac(&self) -> MacAddress {
        self.conf.bridge.mac
    }

    #[must_use]
    pub const fn ip(&self) -> Ipv4Addr {
        self.conf.bridge.ipaddress
    }

    #[must_use]
    pub const fn mqtt_config(&self) -> &MqttConfig {
        &self.conf.mqtt
    }

    #[must_use]
    pub const fn z2m_config(&self) -> &Z2mConfig {
        &self.conf.z2m
    }

    #[must_use]
    pub fn bridge_id(&self) -> String {
        let mac = self.mac().bytes();
        format!(
            "{:02X}{:02X}{:02X}FFFE{:02X}{:02X}{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        )
    }

    #[must_use]
    pub fn api_short_config(&self) -> ApiShortConfig {
        ApiShortConfig {
            bridgeid: self.bridge_id(),
            mac: self.mac(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn api_config(&self, username: Uuid) -> ApiConfig {
        ApiConfig {
            short_config: self.api_short_config(),
            ipaddress: self.conf.bridge.ipaddress,
            netmask: self.conf.bridge.netmask,
            gateway: self.conf.bridge.gateway,
            timezone: self.conf.bridge.timezone.clone(),
            whitelist: HashMap::from([(
                username,
                Whitelist {
                    create_date: Utc::now(),
                    last_use_date: Utc::now(),
                    name: "User#foo".to_string(),
                },
            )]),
            ..ApiConfig::default()
        }
    }

    pub async fn send<T: Serialize + Send>(&self, topic: String, payload: T) -> ApiResult<()> {
        let api_req = crate::z2m::api::Other {
            topic,
            payload: serde_json::to_value(payload)?,
        };

        self.ws
            .lock()
            .await
            .send(Message::Text(serde_json::to_string(&api_req)?))
            .await?;

        log::info!("{api_req:#?}");

        Ok(())
    }

    pub async fn send_set<T: Serialize + Send>(&self, topic: &str, payload: T) -> ApiResult<()> {
        self.send(format!("{topic}/set"), payload).await
    }
}
