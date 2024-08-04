use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use serde_json::json;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::event::EventBlock;
use crate::hue::v2::{
    Bridge, BridgeHome, Device, DeviceProductData, GroupedLight, Light, Metadata, Resource,
    ResourceLink, ResourceRecord, ResourceType, TimeZone,
};

pub struct Resources {
    id_v1: u32,
    pub res: HashMap<Uuid, Resource>,
    pub chan: Sender<EventBlock>,
}

impl Resources {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            id_v1: 1,
            res: HashMap::new(),
            chan: Sender::new(100),
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
        self.add_bridge(bridge_id.to_owned())
    }

    pub fn next_idv1(&mut self) -> u32 {
        self.id_v1 += 1;
        self.id_v1
    }

    pub fn add_resource(&mut self, mut obj: Resource) -> ApiResult<ResourceLink> {
        let link = ResourceLink::random(obj.rtype());
        if obj.assign_id_v1(self.id_v1) {
            self.id_v1 += 1;
        }

        self.add(&link, obj)?;
        Ok(link)
    }

    #[must_use]
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

        let evt = EventBlock::add(serde_json::to_value(self.get_resource_by_id(&link.rid)?)?);

        log::info!("## EVENT ##: {evt:?}");

        let _ = self.chan.send(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        let evt = EventBlock::delete(link)?;

        self.res
            .remove(&link.rid)
            .ok_or(ApiError::NotFound(link.rid))?;

        let _ = self.chan.send(evt);

        Ok(())
    }

    pub fn add_bridge(&mut self, bridge_id: String) -> ApiResult<()> {
        let link_bridge_dev = ResourceLink::random(ResourceType::Device);
        let link_bridge_home_dev = ResourceLink::random(ResourceType::Device);
        let link_bridge = link_bridge_dev.for_type(ResourceType::Bridge);
        let link_bridge_home = link_bridge_home_dev.for_type(ResourceType::BridgeHome);

        let bridge_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::hue_bridge("bifrost"),
            identify: json!({}),
            services: vec![link_bridge.clone()],
        };

        let bridge = Bridge {
            bridge_id,
            owner: link_bridge_dev.clone(),
            time_zone: TimeZone::best_guess(),
        };

        let bridge_home_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::hue_bridge("bifrost bridge home"),
            identify: json!({}),
            services: vec![link_bridge.clone()],
        };

        let bridge_home = BridgeHome {
            children: vec![link_bridge_dev.clone()],
            id_v1: Some("/groups/0".to_string()),
            services: vec![ResourceLink::random(ResourceType::GroupedLight)],
        };

        self.add(&link_bridge_dev, Resource::Device(bridge_dev))?;
        self.add(&link_bridge, Resource::Bridge(bridge))?;
        self.add(&link_bridge_home_dev, Resource::Device(bridge_home_dev))?;
        self.add(&link_bridge_home, Resource::BridgeHome(bridge_home))?;

        Ok(())
    }

    pub fn get_resource(&self, ty: ResourceType, id: &Uuid) -> ApiResult<ResourceRecord> {
        self.res
            .get(id)
            .filter(|id| id.rtype() == ty)
            .map(|r| ResourceRecord::from_ref((id, r)))
            .ok_or_else(|| ApiError::NotFound(*id))
    }

    pub fn get_resource_by_id(&self, id: &Uuid) -> ApiResult<ResourceRecord> {
        self.res
            .get(id)
            .map(|r| ResourceRecord::from_ref((id, r)))
            .ok_or_else(|| ApiError::NotFound(*id))
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
