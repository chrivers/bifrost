use std::net::Ipv4Addr;

use config::{Config, ConfigError};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub name: String,
    pub mac: MacAddress,
    pub ipaddress: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub timezone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub ha_discovery_topic: String,
    pub topics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub bridge: BridgeConfig,
    pub mqtt: MqttConfig,
}

pub fn parse(filename: &str) -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name(filename))
        .build()?;

    settings.try_deserialize()
}
