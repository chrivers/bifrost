use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use serde::{self, Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast::Sender;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::event::EventBlock;
use crate::hue::update::{LightUpdate, Update, UpdateRecord};
use crate::hue::v2::{
    Bridge, BridgeHome, Device, DeviceProductData, GroupedLight, Light, Metadata, RType, Resource,
    ResourceLink, ResourceRecord, Scene, TimeZone,
};
use crate::z2m::update::DeviceColorMode;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AuxData {
    pub topic: Option<String>,
    pub index: Option<u32>,
}

impl AuxData {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_topic(self, topic: &str) -> Self {
        Self {
            topic: Some(topic.to_string()),
            ..self
        }
    }

    #[must_use]
    pub fn with_index(self, index: u32) -> Self {
        Self {
            index: Some(index),
            ..self
        }
    }
}

#[derive(Clone, Debug)]
pub struct Resources {
    pub res: HashMap<Uuid, Resource>,
    pub aux: HashMap<Uuid, AuxData>,
    pub chan: Sender<EventBlock>,
}

impl Resources {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            res: HashMap::new(),
            aux: HashMap::new(),
            chan: Sender::new(100),
        }
    }

    pub fn load(&mut self, rdr: impl Read) -> ApiResult<()> {
        (self.res, self.aux) = serde_yaml::from_reader(rdr)?;
        Ok(())
    }

    pub fn save(&self, wr: impl Write) -> ApiResult<()> {
        Ok(serde_yaml::to_writer(wr, &(&self.res, &self.aux))?)
    }

    pub fn init(&mut self, bridge_id: &str) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned())
    }

    pub fn add_resource(&mut self, obj: Resource) -> ApiResult<ResourceLink> {
        let link = ResourceLink::new(Uuid::new_v4(), obj.rtype());

        self.add(&link, obj)?;
        Ok(link)
    }

    #[must_use]
    pub fn has(&self, id: &Uuid) -> bool {
        self.res.contains_key(id)
    }

    fn update(&mut self, id: &Uuid, mut func: impl FnMut(&mut Resource)) -> ApiResult<()> {
        let obj = self.res.get_mut(id).ok_or(ApiError::NotFound(*id))?;
        func(obj);
        match obj {
            Resource::Light(light) => {
                let mut upd = LightUpdate::new()
                    .with_brightness(light.dimming.brightness)
                    .with_on(light.on.on);

                match light.color_mode {
                    Some(DeviceColorMode::ColorTemp) => {
                        upd = upd.with_color_temperature(light.color_temperature.mirek);
                    }
                    Some(DeviceColorMode::Xy) => {
                        upd = upd.with_color_xy(light.color.xy);
                    }
                    None => {}
                }

                let json = UpdateRecord::new(id, Update::Light(upd));
                let _ = self.chan.send(EventBlock::update(json)?);
            }
            Resource::GroupedLight(glight) => {
                let json = json!({
                    "id": id,
                    "id_v1": format!("/groups/{id}"),
                    "on": glight.on,
                    "dimming": glight.dimming,
                    "owner": glight.owner,
                    "color_temperature": glight.color_temperature,
                    "type": "grouped_light",
                    /* "color": { */
                    /*     "xy": glight.color.xy */
                    /* } */
                });
                let _ = self.chan.send(EventBlock::update(json)?);
            }
            Resource::Scene(scene) => {
                let json = json!({
                    "id": id,
                    "id_v1": format!("/scenes/{id}"),
                    "status": scene.status,
                });
                let _ = self.chan.send(EventBlock::update(json)?);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn update_light(&mut self, id: &Uuid, mut func: impl FnMut(&mut Light)) -> ApiResult<()> {
        self.update(id, |res| {
            if let Resource::Light(ref mut obj) = res {
                func(obj);
            }
        })
    }

    pub fn update_grouped_light(
        &mut self,
        id: &Uuid,
        mut func: impl FnMut(&mut GroupedLight),
    ) -> ApiResult<()> {
        self.update(id, |res| {
            if let Resource::GroupedLight(ref mut obj) = res {
                func(obj);
            }
        })
    }

    pub fn update_scene(&mut self, id: &Uuid, mut func: impl FnMut(&mut Scene)) -> ApiResult<()> {
        self.update(id, |res| {
            if let Resource::Scene(ref mut obj) = res {
                func(obj);
            }
        })
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
        let link_bridge = RType::Bridge.deterministic(&bridge_id);
        let link_bridge_home = RType::BridgeHome.deterministic(&format!("{bridge_id}HOME"));
        let link_bridge_dev = RType::Device.deterministic(link_bridge.rid);
        let link_bridge_home_dev = RType::Device.deterministic(link_bridge_home.rid);

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
            services: vec![RType::GroupedLight.deterministic(link_bridge_home.rid)],
        };

        self.add(&link_bridge_dev, Resource::Device(bridge_dev))?;
        self.add(&link_bridge, Resource::Bridge(bridge))?;
        self.add(&link_bridge_home_dev, Resource::Device(bridge_home_dev))?;
        self.add(&link_bridge_home, Resource::BridgeHome(bridge_home))?;

        Ok(())
    }

    pub fn get_resource(&self, ty: RType, id: &Uuid) -> ApiResult<ResourceRecord> {
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

    pub fn get_resources_by_type(&self, ty: RType) -> Vec<ResourceRecord> {
        self.res
            .iter()
            .filter(|(_, r)| r.rtype() == ty)
            .map(ResourceRecord::from_ref)
            .collect()
    }
}
