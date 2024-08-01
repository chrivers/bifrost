use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

use chrono::Utc;
use mac_address::MacAddress;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::{AppConfig, MqttConfig};
use crate::error::ApiResult;
use crate::hue::event::EventBlock;
use crate::hue::v1::{ApiConfig, ApiShortConfig, Whitelist};
use crate::hue::v2::{ResourceRecord, ResourceType};
use crate::resource::Resources;

#[derive(Clone)]
pub struct AppState {
    conf: AppConfig,
    pub res: Arc<Mutex<Resources>>,
}

impl AppState {
    pub fn new(conf: AppConfig) -> Self {
        Self {
            conf,
            res: Arc::new(Mutex::new(Resources::new())),
        }
    }

    pub const fn mac(&self) -> MacAddress {
        self.conf.bridge.mac
    }

    pub const fn ip(&self) -> Ipv4Addr {
        self.conf.bridge.ipaddress
    }

    pub const fn mqtt_config(&self) -> &MqttConfig {
        &self.conf.mqtt
    }

    pub async fn channel(&self) -> Receiver<EventBlock> {
        self.res.lock().await.chan.subscribe()
    }

    pub fn bridge_id(&self) -> String {
        let mac = self.mac().bytes();
        format!(
            "{:02X}{:02X}{:02X}FFFE{:02X}{:02X}{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
        )
    }

    pub fn api_short_config(&self) -> ApiShortConfig {
        ApiShortConfig {
            bridgeid: self.bridge_id(),
            mac: self.mac(),
            ..Default::default()
        }
    }

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

    pub async fn get_resources(&self) -> Vec<ResourceRecord> {
        self.res.lock().await.get_resources()
    }

    pub async fn get_resources_by_type(&self, ty: ResourceType) -> Vec<ResourceRecord> {
        self.res.lock().await.get_resources_by_type(ty)
    }

    pub async fn get_resource(&self, ty: ResourceType, id: Uuid) -> ApiResult<ResourceRecord> {
        self.res.lock().await.get_resource(ty, id)
    }
}
