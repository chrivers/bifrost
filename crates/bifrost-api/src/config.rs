use std::net::Ipv4Addr;
use std::{collections::BTreeMap, num::NonZeroU32};

use camino::Utf8PathBuf;
use chrono_tz::Tz;
use hue::api::RoomArchetype;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{Client, error::BifrostResult};

#[cfg(feature = "mac")]
use mac_address::MacAddress;
#[cfg(not(feature = "mac"))]
type MacAddress = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub name: String,
    pub mac: MacAddress,
    pub ipaddress: Ipv4Addr,
    pub http_port: u16,
    pub https_port: u16,
    pub entm_port: u16,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub timezone: Tz,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct BifrostConfig {
    pub state_file: Utf8PathBuf,
    pub cert_file: Utf8PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Z2mConfig {
    #[serde(flatten)]
    pub servers: BTreeMap<String, Z2mServer>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Z2mServer {
    pub url: Url,
    pub group_prefix: Option<String>,
    pub disable_tls_verify: Option<bool>,
    pub streaming_fps: Option<NonZeroU32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, Eq, PartialEq)]
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
    pub rooms: BTreeMap<String, RoomConfig>,
}

impl Z2mServer {
    #[must_use]
    pub fn get_url(&self) -> Url {
        let mut url = self.url.clone();
        // z2m version 1.x allows both / and /api as endpoints for the
        // websocket, but version 2.x only allows /api. By adding /api (if
        // missing), we ensure compatibility with both versions.
        if !url.path().ends_with("/api") {
            if let Ok(mut path) = url.path_segments_mut() {
                path.push("api");
            }
        }

        // z2m version 2.x requires an auth token on the websocket. If one is
        // not specified in the z2m configuration, the literal string
        // `your-secret-token` is used!
        //
        // To be compatible, we mirror this behavior here. If "token" is set
        // manually by the user, we do nothing.
        if !url.query_pairs().any(|(key, _)| key == "token") {
            url.query_pairs_mut()
                .append_pair("token", "your-secret-token");
        }

        url
    }

    #[must_use]
    #[allow(clippy::option_if_let_else)]
    fn sanitize_url(url: &str) -> String {
        match url.find("token=") {
            Some(offset) => {
                let token = &url[offset + "token=".len()..];
                if token == "your-secret-token" {
                    // this is the standard "blank" token, it's safe to show
                    url.to_string()
                } else {
                    // this is an actual secret token, blank it out with a
                    // standard-length placeholder.
                    format!("{}token={}", &url[..offset], "<<REDACTED>>")
                }
            }
            None => url.to_string(),
        }
    }

    #[must_use]
    pub fn get_sanitized_url(&self) -> String {
        Self::sanitize_url(self.get_url().as_str())
    }
}

impl Client {
    pub async fn config(&self) -> BifrostResult<AppConfig> {
        self.get("config").await
    }
}
