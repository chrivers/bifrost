use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::sync::Arc;

use mac_address::MacAddress;
use serde_json::json;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::{AppConfig, MqttConfig};
use crate::hue::event::EventBlock;
use crate::hue::v1::{ApiConfig, ApiShortConfig, Whitelist};
use crate::hue::v2::{
    Bridge, Device, DeviceProductData, Light, Metadata, Resource, ResourceLink, ResourceRecord,
    ResourceType, Room, RoomArchetypes, Scene, TimeZone,
};

#[derive(Clone)]
pub struct AppState {
    conf: AppConfig,
    pub res: Arc<Mutex<Resources>>,
}

pub struct Resources {
    id_v1: u32,
    pub res: HashMap<Uuid, Resource>,
    pub chan: Sender<EventBlock>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            id_v1: 1,
            res: HashMap::new(),
            chan: Sender::new(10),
        }
    }

    pub fn load(&mut self, rdr: impl Read) -> Result<(), serde_yaml::Error> {
        self.res = serde_yaml::from_reader(rdr)?;
        Ok(())
    }

    pub fn save(&self, wr: impl Write) -> Result<(), serde_yaml::Error> {
        serde_yaml::to_writer(wr, &self.res)
    }

    pub fn init(&mut self, bridge_id: &str) {
        self.add_bridge(bridge_id.to_owned());
        let link = self.add_light();
        self.add_room_init(&[link]);
    }

    fn next_idv1(&mut self) -> u32 {
        self.id_v1 += 1;
        self.id_v1
    }

    fn add(&mut self, link: &ResourceLink, obj: Resource) {
        assert!(
            link.rtype == obj.rtype(),
            "Link type failed: {:?} expected but {:?} given",
            link.rtype,
            obj.rtype()
        );

        self.res.insert(link.rid, obj);

        if let Ok(fd) = File::create("state.yaml") {
            self.save(fd).unwrap();
        }

        let evt = EventBlock::add(
            serde_json::to_value(self.get_resource_by_id(link.rid).unwrap()).unwrap(),
        );

        log::info!("evt: {evt:?}");

        let _ = self.chan.send(evt);
    }

    pub fn delete(&mut self, link: &ResourceLink) {
        let evt = EventBlock::delete(
            serde_json::to_value(self.get_resource_by_id(link.rid).unwrap()).unwrap(),
        );

        self.res.remove(&link.rid);

        log::info!("evt: {evt:?}");

        let _ = self.chan.send(evt);
    }

    pub fn add_bridge(&mut self, bridge_id: String) {
        let link_device = ResourceType::Device.link();
        let link_bridge = ResourceType::Bridge.link();

        let dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::hue_bridge("bifrost"),
            identify: json!({}),
            services: vec![link_bridge.clone()],
        };

        let br = Bridge {
            bridge_id,
            owner: link_device.clone(),
            time_zone: TimeZone::best_guess(),
        };

        self.add(&link_device, Resource::Device(dev));
        self.add(&link_bridge, Resource::Bridge(br));
    }

    pub fn add_light(&mut self) -> ResourceLink {
        let link_device = ResourceType::Device.link();
        let link_light = ResourceType::Light.link();

        let dev = Device {
            product_data: DeviceProductData::hue_color_spot(),
            metadata: Metadata::spot_bulb("Hue color spot 1"),
            identify: json!({}),
            services: vec![link_light.clone()],
        };

        let light = Light::new(self.next_idv1(), link_device.clone());

        let res = link_device.clone();

        self.add(&link_device, Resource::Device(dev));
        self.add(&link_light, Resource::Light(light));

        res
    }

    pub fn add_room_init(&mut self, children: &[ResourceLink]) {
        let link_room = ResourceType::Room.link();

        let room = Room {
            id_v1: Some("/room/1".to_string()),
            children: children.to_owned(),
            metadata: Metadata::room(RoomArchetypes::Computer, "Room 1"),
            services: vec![],
        };

        self.add(&link_room, Resource::Room(room));
    }

    pub fn add_scene(&mut self, scene: Scene) -> ResourceLink {
        let link = ResourceType::Scene.link();
        self.add(&link, Resource::Scene(scene));
        link
    }

    pub fn add_room(&mut self, room: Room) -> ResourceLink {
        let link = ResourceType::Room.link();
        self.add(&link, Resource::Room(room));
        link
    }

    pub fn get_resource(&self, ty: ResourceType, id: Uuid) -> Option<ResourceRecord> {
        self.res
            .get(&id)
            .filter(|id| id.rtype() == ty)
            .map(|r| ResourceRecord::from_ref((&id, r)))
    }

    pub fn get_resource_by_id(&self, id: Uuid) -> Option<ResourceRecord> {
        self.res
            .get(&id)
            .map(|r| ResourceRecord::from_ref((&id, r)))
    }

    pub fn get_resources(&self) -> Vec<ResourceRecord> {
        self.res.iter().map(ResourceRecord::from_ref).collect()
    }

    pub fn get_resources_by_type(&self, ty: ResourceType) -> Vec<ResourceRecord> {
        self.res
            .iter()
            .filter(|(_, r)| r.rtype() == ty)
            .map(ResourceRecord::from_ref)
            .collect()
    }
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
                    create_date: "2020-01-01T01:01:01".to_string(),
                    last_use_date: "2020-01-01T01:01:01".to_string(),
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

    pub async fn get_resource(&self, ty: ResourceType, id: Uuid) -> Option<ResourceRecord> {
        self.res.lock().await.get_resource(ty, id)
    }
}
