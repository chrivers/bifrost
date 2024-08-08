use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::sync::Arc;

use serde::{self, Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::api::{
    Bridge, BridgeHome, Device, DeviceProductData, Metadata, RType, Resource, ResourceLink,
    ResourceRecord, TimeZone,
};
use crate::hue::api::{GroupedLightUpdate, LightUpdate, SceneUpdate, Update};
use crate::hue::event::EventBlock;
use crate::z2m::api::Other;
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
    aux: HashMap<Uuid, AuxData>,
    state_updates: Arc<Notify>,
    pub res: HashMap<Uuid, Resource>,
    pub hue_updates: Sender<EventBlock>,
    pub z2m_updates: Sender<Other>,
}

impl Resources {
    const MAX_SCENE_ID: u32 = 100;

    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            res: HashMap::new(),
            aux: HashMap::new(),
            state_updates: Arc::new(Notify::new()),
            hue_updates: Sender::new(32),
            z2m_updates: Sender::new(32),
        }
    }

    pub fn read(&mut self, rdr: impl Read) -> ApiResult<()> {
        (self.res, self.aux) = serde_yaml::from_reader(rdr)?;
        Ok(())
    }

    pub fn write(&self, wr: impl Write) -> ApiResult<()> {
        Ok(serde_yaml::to_writer(wr, &(&self.res, &self.aux))?)
    }

    pub fn init(&mut self, bridge_id: &str) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned())
    }

    pub fn aux_get(&self, link: &ResourceLink) -> ApiResult<&AuxData> {
        self.aux
            .get(&link.rid)
            .ok_or_else(|| ApiError::AuxNotFound(*link))
    }

    pub fn aux_set(&mut self, link: &ResourceLink, aux: AuxData) {
        self.aux.insert(link.rid, aux);
    }

    fn generate_update(obj: &Resource) -> ApiResult<Option<Update>> {
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

                Ok(Some(Update::Light(upd)))
            }
            Resource::GroupedLight(glight) => {
                let mut upd = GroupedLightUpdate::new()
                    .with_brightness(glight.dimming.brightness)
                    .with_on(glight.on.on);

                match glight.color_mode {
                    Some(DeviceColorMode::ColorTemp) => {
                        upd = upd.with_color_temperature(glight.color_temperature.mirek);
                    }
                    Some(DeviceColorMode::Xy) => {
                        upd = upd.with_color_xy(glight.color.xy);
                    }
                    None => {}
                }

                Ok(Some(Update::GroupedLight(upd)))
            }
            Resource::Scene(scene) => {
                let upd = SceneUpdate::new()
                    .with_actions(Some(scene.actions.clone()))
                    .with_recall_action(scene.status);

                Ok(Some(Update::Scene(upd)))
            }
            Resource::Room(_) => Ok(None),
            obj => Err(ApiError::UpdateUnsupported(obj.rtype())),
        }
    }

    pub fn try_update<T>(
        &mut self,
        id: &Uuid,
        func: impl FnOnce(&mut T) -> ApiResult<()>,
    ) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = ApiError>,
    {
        let obj = self.res.get_mut(id).ok_or(ApiError::NotFound(*id))?;
        func(obj.try_into()?)?;

        if let Some(delta) = Self::generate_update(obj)? {
            self.hue_event(EventBlock::update(id, delta)?);
        }

        self.state_updates.notify_one();

        Ok(())
    }

    pub fn update<T>(&mut self, id: &Uuid, func: impl FnOnce(&mut T)) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = ApiError>,
    {
        self.try_update(id, |obj: &mut T| {
            func(obj);
            Ok(())
        })
    }

    pub fn add(&mut self, link: &ResourceLink, obj: Resource) -> ApiResult<()> {
        assert!(
            link.rtype == obj.rtype(),
            "Link type failed: {:?} expected but {:?} given",
            link.rtype,
            obj.rtype()
        );

        if self.res.contains_key(&link.rid) {
            log::debug!("Resource {link:?} is already known");
            return Ok(());
        }

        self.res.insert(link.rid, obj);

        self.state_updates.notify_one();

        let evt = EventBlock::add(serde_json::to_value(self.get_resource_by_id(&link.rid)?)?);

        log::trace!("Send event: {evt:?}");

        self.hue_event(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        self.res
            .remove(&link.rid)
            .ok_or(ApiError::NotFound(link.rid))?;

        let evt = EventBlock::delete(link)?;

        self.hue_event(evt);

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
            services: vec![link_bridge],
        };

        let bridge = Bridge {
            bridge_id,
            owner: link_bridge_dev,
            time_zone: TimeZone::best_guess(),
        };

        let bridge_home_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::hue_bridge("bifrost bridge home"),
            identify: json!({}),
            services: vec![link_bridge],
        };

        let bridge_home = BridgeHome {
            children: vec![link_bridge_dev],
            services: vec![RType::GroupedLight.deterministic(link_bridge_home.rid)],
        };

        self.add(&link_bridge_dev, Resource::Device(bridge_dev))?;
        self.add(&link_bridge, Resource::Bridge(bridge))?;
        self.add(&link_bridge_home_dev, Resource::Device(bridge_home_dev))?;
        self.add(&link_bridge_home, Resource::BridgeHome(bridge_home))?;

        Ok(())
    }

    pub fn get_next_scene_id(&self, room: &ResourceLink) -> ApiResult<u32> {
        let mut set: HashSet<u32> = HashSet::new();

        for scene in self.get_resources_by_type(RType::Scene) {
            let Resource::Scene(scn) = scene.obj else {
                continue;
            };

            if &scn.group == room {
                let Some(AuxData {
                    index: Some(index), ..
                }) = self.aux.get(&scene.id)
                else {
                    continue;
                };

                set.insert(*index);
            }
        }

        for x in 0..Self::MAX_SCENE_ID {
            if !set.contains(&x) {
                return Ok(x);
            }
        }
        Err(ApiError::Full(RType::Scene))
    }

    pub fn get<T>(&self, link: &ResourceLink) -> ApiResult<T>
    where
        T: TryFrom<Resource, Error = ApiError>,
    {
        self.res
            .get(&link.rid)
            .filter(|id| id.rtype() == link.rtype)
            .ok_or_else(|| ApiError::NotFound(link.rid))?
            .clone()
            .try_into()
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

    #[must_use]
    pub fn state_channel(&self) -> Arc<Notify> {
        self.state_updates.clone()
    }

    #[must_use]
    pub fn hue_channel(&self) -> Receiver<EventBlock> {
        self.hue_updates.subscribe()
    }

    fn hue_event(&self, evt: EventBlock) {
        if let Err(err) = self.hue_updates.send(evt) {
            log::trace!("Overflow on hue event pipe: {err}");
        }
    }

    #[must_use]
    pub fn z2m_channel(&self) -> Receiver<Other> {
        self.z2m_updates.subscribe()
    }

    pub fn z2m_send<T: Serialize + Send>(&self, topic: String, payload: T) -> ApiResult<()> {
        let api_req = crate::z2m::api::Other {
            topic,
            payload: serde_json::to_value(payload)?,
        };

        log::debug!("z2m request: {api_req:#?}");

        self.z2m_updates.send(api_req)?;

        Ok(())
    }

    pub fn z2m_send_set<T: Serialize + Send>(&self, topic: &str, payload: T) -> ApiResult<()> {
        self.z2m_send(format!("{topic}/set"), payload)
    }
}
