use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use serde_json::json;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::event::EventBlock;
use crate::hue::v2::{
    Bridge, Device, DeviceProductData, Light, Metadata, Resource, ResourceLink, ResourceRecord,
    ResourceType, Room, RoomArchetypes, Scene, TimeZone,
};

pub struct Resources {
    id_v1: u32,
    pub res: HashMap<Uuid, Resource>,
    pub chan: Sender<EventBlock>,
}

impl Resources {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            id_v1: 1,
            res: HashMap::new(),
            chan: Sender::new(10),
        }
    }

    pub fn load(&mut self, rdr: impl Read) -> ApiResult<()> {
        self.res = serde_yaml::from_reader(rdr)?;
        Ok(())
    }

    pub fn save(&self, wr: impl Write) -> ApiResult<()> {
        Ok(serde_yaml::to_writer(wr, &self.res)?)
    }

    pub fn init(&mut self, bridge_id: &str) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned())?;
        let link = self.add_light(Uuid::new_v4(), "Hue color spot 1")?;
        self.add_room_init(&[link])
    }

    fn next_idv1(&mut self) -> u32 {
        self.id_v1 += 1;
        self.id_v1
    }

    pub fn add_resource(&mut self, mut obj: Resource) -> ApiResult<ResourceLink> {
        let link = obj.rtype().link();
        if obj.assign_id_v1(self.id_v1) {
            self.id_v1 += 1;
        }

        self.add(&link, obj)?;
        Ok(link)
    }

    pub fn has(&self, id: &Uuid) -> bool {
        self.res.contains_key(id)
    }

    pub fn add(&mut self, link: &ResourceLink, obj: Resource) -> ApiResult<()> {
        assert!(
            link.rtype == obj.rtype(),
            "Link type failed: {:?} expected but {:?} given",
            link.rtype,
            obj.rtype()
        );

        self.res.insert(link.rid, obj);

        if let Ok(fd) = File::create("state.yaml") {
            self.save(fd)?;
        }

        let evt = EventBlock::add(serde_json::to_value(self.get_resource_by_id(link.rid)?)?);

        log::info!("## EVENT ##: {evt:?}");

        let _ = self.chan.send(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        let evt = EventBlock::delete(link)?;

        self.res
            .remove(&link.rid)
            .ok_or(ApiError::NotFound(link.rid))?;

        log::info!("## EVENT ##: {evt:?}");

        let _ = self.chan.send(evt);

        Ok(())
    }

    pub fn add_bridge(&mut self, bridge_id: String) -> ApiResult<()> {
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

        self.add(&link_device, Resource::Device(dev))?;
        self.add(&link_bridge, Resource::Bridge(br))
    }

    pub fn add_light(&mut self, uuid: Uuid, name: &str) -> ApiResult<ResourceLink> {
        let link_device = ResourceType::Device.link_to(uuid);
        let link_light = ResourceType::Light.link();

        let dev = Device {
            product_data: DeviceProductData::hue_color_spot(),
            metadata: Metadata::spot_bulb(name),
            identify: json!({}),
            services: vec![link_light.clone()],
        };

        let light = Light::new(self.next_idv1(), link_device.clone());

        let res = link_device.clone();

        self.add(&link_device, Resource::Device(dev))?;
        self.add(&link_light, Resource::Light(light))?;

        Ok(res)
    }

    pub fn add_room_init(&mut self, children: &[ResourceLink]) -> ApiResult<()> {
        let link_room = ResourceType::Room.link();

        let room = Room {
            id_v1: Some("/room/1".to_string()),
            children: children.to_owned(),
            metadata: Metadata::room(RoomArchetypes::Computer, "Room 1"),
            services: vec![],
        };

        self.add(&link_room, Resource::Room(room))
    }

    pub fn add_room_z2m(
        &mut self,
        name: &str,
        uuid: Uuid,
        id_v1: u32,
        children: &[ResourceLink],
    ) -> ApiResult<()> {
        let link_room = ResourceType::Room.link_to(uuid);

        let room = Room {
            id_v1: Some(format!("/room/{id_v1}")),
            children: children.to_owned(),
            metadata: Metadata::room(RoomArchetypes::Computer, name),
            services: vec![],
        };

        self.add(&link_room, Resource::Room(room))
    }

    pub fn add_scene(&mut self, scene: Scene) -> ApiResult<ResourceLink> {
        let link = ResourceType::Scene.link();
        self.add(&link, Resource::Scene(scene))?;

        Ok(link)
    }

    pub fn add_room(&mut self, room: Room) -> ApiResult<ResourceLink> {
        let link = ResourceType::Room.link();
        self.add(&link, Resource::Room(room))?;

        Ok(link)
    }

    pub fn get_resource(&self, ty: ResourceType, id: Uuid) -> ApiResult<ResourceRecord> {
        self.res
            .get(&id)
            .filter(|id| id.rtype() == ty)
            .map(|r| ResourceRecord::from_ref((&id, r)))
            .ok_or(ApiError::NotFound(id))
    }

    pub fn get_resource_by_id(&self, id: Uuid) -> ApiResult<ResourceRecord> {
        self.res
            .get(&id)
            .map(|r| ResourceRecord::from_ref((&id, r)))
            .ok_or(ApiError::NotFound(id))
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
