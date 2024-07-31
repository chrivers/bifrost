use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;

use mac_address::MacAddress;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::{AppConfig, MqttConfig};
use crate::hue::v1::{ApiConfig, ApiShortConfig, Capabilities, Whitelist};
use crate::hue::v2::{
    Bridge, Device, DeviceProductData, Light, Metadata, Resource, ResourceLink, ResourceRecord,
    ResourceType, Room, RoomArchetypes, TimeZone,
};

#[derive(Clone)]
pub struct AppState {
    conf: AppConfig,
    pub res: Arc<Mutex<Resources>>,
}

pub struct Resources {
    id_v1: u32,
    pub res: HashMap<Uuid, Resource>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            id_v1: 1,
            res: HashMap::new(),
        }
    }

    fn next_idv1(&mut self) -> u32 {
        self.id_v1 += 1;
        self.id_v1
    }

    fn add(&mut self, link: ResourceLink, obj: Resource) {
        if link.rtype != obj.rtype() {
            panic!(
                "Link type failed: {:?} expected but {:?} given",
                link.rtype,
                obj.rtype()
            );
        }

        self.add_named(link.rid, obj);
    }

    fn add_named(&mut self, uuid: Uuid, obj: Resource) {
        self.res.insert(uuid, obj);
    }

    pub fn link(&self, rtype: ResourceType) -> ResourceLink {
        ResourceLink {
            rid: Uuid::new_v4(),
            rtype,
        }
    }

    pub fn add_bridge(&mut self, bridge_id: String) {
        let link_device = self.link(ResourceType::Device);
        let link_bridge = self.link(ResourceType::Bridge);

        let dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::hue_bridge("bifrost"),
            identify: json!({}),
            services: vec![link_bridge.clone()],
        };

        let br = Bridge {
            bridge_id,
            owner: link_device.clone(),
            time_zone: TimeZone {
                time_zone: "Europe/Copenhagen".to_string(),
            },
        };

        self.add(link_device, Resource::Device(dev));
        self.add(link_bridge, Resource::Bridge(br));
    }

    pub fn add_light(&mut self) -> ResourceLink {
        let link_device = self.link(ResourceType::Device);
        let link_light = self.link(ResourceType::Light);

        let dev = Device {
            product_data: DeviceProductData::hue_color_spot(),
            metadata: Metadata::spot_bulb("Hue color spot 1"),
            identify: json!({}),
            services: vec![link_light.clone()],
        };

        let light = Light::new(self.next_idv1(), link_device.clone());

        let res = link_device.clone();

        self.add(link_device, Resource::Device(dev));
        self.add(link_light, Resource::Light(light));

        res
    }

    pub fn add_room(&mut self, children: &[ResourceLink]) {
        let link_room = self.link(ResourceType::Room);

        let room = Room {
            id_v1: "/room/1".to_string(),
            children: children.to_owned(),
            metadata: Metadata::room(RoomArchetypes::Computer, "Room 1"),
            services: vec![],
        };

        self.add(link_room, Resource::Room(room));
    }

    pub fn to_json(&self) -> Value {
        json!({})
    }
}

impl AppState {
    pub fn new(conf: AppConfig) -> Self {
        Self {
            conf,
            res: Arc::new(Mutex::new(Resources::new())),
        }
    }

    pub async fn init(&mut self) {
        let mut res = self.res.lock().await;

        res.add_bridge(self.bridge_id());
        let link = res.add_light();
        res.add_room(&[link])
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

    pub async fn get_resources(&self) -> Vec<ResourceRecord> {
        self.res
            .lock()
            .await
            .res
            .iter()
            .map(ResourceRecord::from_ref)
            .collect()
    }

    pub async fn get_resources_by_type(&self, ty: ResourceType) -> Vec<ResourceRecord> {
        self.res
            .lock()
            .await
            .res
            .iter()
            .filter(|(_, r)| r.rtype() == ty)
            .map(ResourceRecord::from_ref)
            .collect()
    }

    pub async fn get_resource(&self, ty: ResourceType, id: Uuid) -> Option<ResourceRecord> {
        self.res
            .lock()
            .await
            .res
            .get(&id)
            .filter(|id| id.rtype() == ty)
            .map(|r| ResourceRecord::from_ref((&id, r)))
    }

    pub fn capabilities(&self) -> Capabilities {
        Capabilities::new()
    }
}
