use std::net::Ipv4Addr;

use mac_address::MacAddress;
use uuid::{uuid, Uuid};

use crate::config::{AppConfig, MqttConfig};
use crate::hue::v1::{ApiConfig, ApiShortConfig, Capabilities};
use crate::hue::v2::{Bridge, ClipResourceType, Resource, ResourceLink, TimeZone};

#[derive(Clone, Debug)]
pub struct AppState {
    conf: AppConfig,
}

impl AppState {
    pub fn new(conf: AppConfig) -> Self {
        Self { conf }
    }

    pub fn mac(&self) -> MacAddress {
        self.conf.bridge.mac
    }

    pub fn ip(&self) -> Ipv4Addr {
        self.conf.bridge.ipaddress
    }

    pub fn mqtt_config(&self) -> &MqttConfig {
        &self.conf.mqtt
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
                    create_date: "2020-01-01T01:01:01".to_string(),
                    last_use_date: "2020-01-01T01:01:01".to_string(),
                    name: "User#foo".to_string(),
                },
            )]),
            ..ApiConfig::default()
        }
    }

    pub fn get_bridge(&self) -> Resource {
        let bridge_id = self.bridge_id();
        let bridge = Bridge {
            id: Uuid::new_v5(
                &Uuid::NAMESPACE_URL,
                format!("{bridge_id}device").as_bytes(),
            ),
            bridge_id,
            owner: ResourceLink {
                rid: uuid!("00000000-0000-0000-0000-000000000000"),
                rtype: ClipResourceType::Device,
            },
            time_zone: TimeZone {
                time_zone: "Europe/London".to_string(),
            },
        };
        Resource::Bridge(bridge)
    }

    pub fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }
}
