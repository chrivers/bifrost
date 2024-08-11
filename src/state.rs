use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

use chrono::Utc;
use mac_address::MacAddress;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::{AppConfig, Z2mConfig};
use crate::error::ApiResult;
use crate::hue::legacy_api::{ApiConfig, ApiShortConfig, Whitelist};
use crate::resource::Resources;

#[derive(Clone)]
pub struct AppState {
    conf: Arc<AppConfig>,
    pub res: Arc<Mutex<Resources>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> ApiResult<Self> {
        let conf = Arc::new(config);
        let res = Arc::new(Mutex::new(Resources::new()));

        Ok(Self { conf, res })
    }

    #[must_use]
    pub fn mac(&self) -> MacAddress {
        self.conf.bridge.mac
    }

    #[must_use]
    pub fn ip(&self) -> Ipv4Addr {
        self.conf.bridge.ipaddress
    }

    #[must_use]
    pub fn z2m_config(&self) -> &Z2mConfig {
        &self.conf.z2m
    }

    #[must_use]
    pub fn config(&self) -> Arc<AppConfig> {
        self.conf.clone()
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
}
