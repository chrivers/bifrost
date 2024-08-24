use std::{collections::HashMap, net::Ipv4Addr};

use camino::{Utf8Path, Utf8PathBuf};
use config::{Config, ConfigError};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};

use crate::hue::api::RoomArchetype;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub name: String,
    pub mac: MacAddress,
    pub ipaddress: Ipv4Addr,
    pub http_port: u16,
    pub https_port: u16,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub timezone: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BifrostConfig {
    pub state_file: Utf8PathBuf,
    pub cert_file: Utf8PathBuf,
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
    pub bifrost: BifrostConfig,
    #[serde(default)]
    pub rooms: HashMap<String, RoomConfig>,
}

pub fn parse(filename: &Utf8Path) -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        .set_default("bifrost.state_file", "state.yaml")?
        .set_default("bifrost.cert_file", "cert.pem")?
        .set_default("bridge.http_port", 80)?
        .set_default("bridge.https_port", 443)?
        .add_source(config::File::with_name(filename.as_str()))
        .build()?;

    settings.try_deserialize()
}
