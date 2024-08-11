use std::{collections::HashMap, net::Ipv4Addr};

use config::{Config, ConfigError};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};

use crate::hue::api::RoomArchetype;

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
pub struct Z2mConfig {
    #[serde(flatten)]
    pub servers: HashMap<String, Z2mServer>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Z2mServer {
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoomConfig {
    pub name: Option<String>,
    pub icon: Option<RoomArchetype>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub bridge: BridgeConfig,
    pub z2m: Z2mConfig,
    #[serde(default)]
    pub rooms: HashMap<String, RoomConfig>,
}

pub fn parse(filename: &str) -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name(filename))
        .build()?;

    settings.try_deserialize()
}
