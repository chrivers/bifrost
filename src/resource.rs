use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::Arc;

use serde_json::json;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::api::{
    Bridge, BridgeHome, Device, DeviceArchetype, DeviceProductData, Identify, Metadata, RType,
    Resource, ResourceLink, ResourceRecord, TimeZone, ZigbeeConnectivity, ZigbeeConnectivityStatus,
    ZigbeeDeviceDiscovery,
};
use crate::hue::api::{GroupedLightUpdate, LightUpdate, SceneUpdate, Update};
use crate::hue::event::EventBlock;
use crate::model::state::{AuxData, State};
use crate::z2m::request::ClientRequest;

#[derive(Clone, Debug)]
pub struct Resources {
    state: State,
    state_updates: Arc<Notify>,
    pub hue_updates: Sender<EventBlock>,
    pub z2m_updates: Sender<Arc<ClientRequest>>,
}

impl Resources {
    const MAX_SCENE_ID: u32 = 100;

    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: State::new(),
            state_updates: Arc::new(Notify::new()),
            hue_updates: Sender::new(32),
            z2m_updates: Sender::new(32),
        }
    }

    pub fn read(&mut self, rdr: impl Read) -> ApiResult<()> {
        self.state = serde_yaml::from_reader(rdr)?;
        Ok(())
    }

    pub fn write(&self, wr: impl Write) -> ApiResult<()> {
        Ok(serde_yaml::to_writer(wr, &self.state)?)
    }

    pub fn serialize(&self) -> ApiResult<String> {
        Ok(serde_yaml::to_string(&self.state)?)
    }

    pub fn init(&mut self, bridge_id: &str) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned())
    }

    pub fn aux_get(&self, link: &ResourceLink) -> ApiResult<&AuxData> {
        self.state.aux_get(link)
    }

    pub fn aux_set(&mut self, link: &ResourceLink, aux: AuxData) {
        self.state.aux_set(link, aux);
    }

    fn generate_update(obj: &Resource) -> ApiResult<Option<Update>> {
        match obj {
            Resource::Light(light) => {
                let upd = LightUpdate::new()
                    .with_brightness(light.dimming)
                    .with_on(light.on)
                    .with_color_temperature(light.as_mirek_opt())
                    .with_color_xy(light.as_color_opt());

                Ok(Some(Update::Light(upd)))
            }
            Resource::GroupedLight(glight) => {
                let upd = GroupedLightUpdate::new()
                    .with_on(glight.on)
                    .with_brightness(glight.as_brightness_opt());

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
        let obj = self.state.get_mut(id)?;
        func(obj.try_into()?)?;

        if let Some(delta) = Self::generate_update(obj)? {
            let id_v1 = self.state.id_v1(id);
            self.hue_event(EventBlock::update(id, id_v1, delta)?);
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

    #[must_use]
    pub fn get_scenes_for_room(&self, id: &Uuid) -> Vec<Uuid> {
        self.state
            .res
            .iter()
            .filter_map(|(k, v)| {
                if let Resource::Scene(scn) = v {
                    if &scn.group.rid == id {
                        Some(k)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .copied()
            .collect()
    }

    pub fn add(&mut self, link: &ResourceLink, obj: Resource) -> ApiResult<()> {
        assert!(
            link.rtype == obj.rtype(),
            "Link type failed: {:?} expected but {:?} given",
            link.rtype,
            obj.rtype()
        );

        if self.state.res.contains_key(&link.rid) {
            log::trace!("Resource {link:?} is already known");
            return Ok(());
        }

        self.state.insert(link.rid, obj);

        self.state_updates.notify_one();

        let evt = EventBlock::add(serde_json::to_value(self.get_resource_by_id(&link.rid)?)?);

        log::trace!("Send event: {evt:?}");

        self.hue_event(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        log::info!("Deleting {link:?}..");
        self.state.remove(&link.rid)?;

        self.state_updates.notify_one();

        let evt = EventBlock::delete(link)?;

        self.hue_event(evt);

        Ok(())
    }

    pub fn add_bridge(&mut self, bridge_id: String) -> ApiResult<()> {
        let link_bridge = RType::Bridge.deterministic(&bridge_id);
        let link_bridge_home = RType::BridgeHome.deterministic(format!("{bridge_id}HOME"));
        let link_bridge_dev = RType::Device.deterministic(link_bridge.rid);
        let link_bridge_home_dev = RType::Device.deterministic(link_bridge_home.rid);
        let link_zbdd = RType::ZigbeeDeviceDiscovery.deterministic(link_bridge.rid);
        let link_zbc = RType::ZigbeeConnectivity.deterministic(link_bridge.rid);

        let bridge_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::new(DeviceArchetype::BridgeV2, "Bifrost"),
            services: vec![link_bridge, link_zbdd, link_zbc],
            identify: Identify {},
        };

        let bridge = Bridge {
            bridge_id,
            owner: link_bridge_dev,
            time_zone: TimeZone::best_guess(),
        };

        let bridge_home_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(),
            metadata: Metadata::new(DeviceArchetype::BridgeV2, "Bifrost Bridge Home"),
            services: vec![link_bridge],
            identify: Identify {},
        };

        let bridge_home = BridgeHome {
            children: vec![link_bridge_dev],
            services: vec![RType::GroupedLight.deterministic(link_bridge_home.rid)],
        };

        let zbdd = ZigbeeDeviceDiscovery {
            owner: link_bridge_dev,
            status: String::from("ready"),
        };

        let zbc = ZigbeeConnectivity {
            owner: link_bridge_dev,
            mac_address: String::from("11:22:33:44:55:66:77:88"),
            status: ZigbeeConnectivityStatus::ConnectivityIssue,
            channel: Some(json!({
                "status": "set",
                "value": "channel_25",
            })),
            extended_pan_id: None,
        };

        self.add(&link_bridge_dev, Resource::Device(bridge_dev))?;
        self.add(&link_bridge, Resource::Bridge(bridge))?;
        self.add(&link_bridge_home_dev, Resource::Device(bridge_home_dev))?;
        self.add(&link_bridge_home, Resource::BridgeHome(bridge_home))?;
        self.add(&link_zbdd, Resource::ZigbeeDeviceDiscovery(zbdd))?;
        self.add(&link_zbc, Resource::ZigbeeConnectivity(zbc))?;

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
                }) = self.state.try_aux_get(&scene.id)
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

    pub fn get<'a, T>(&'a self, link: &ResourceLink) -> ApiResult<&'a T>
    where
        &'a T: TryFrom<&'a Resource, Error = ApiError>,
    {
        self.state.get(&link.rid)?.try_into()
    }

    /*
    behavior_script           null
    bridge_home               /groups/{id}
    bridge                    null
    device                    /lights/{id} | null
    entertainment             /lights/{id} | null
    geofence_client           null
    geolocation               null
    grouped_light             /groups/{id}
    homekit                   null
    light                     /lights/{id}
    matter                    null
    room                      /groups/{id}
    scene                     /scenes/{id}
    smart_scene               null
    zigbee_connectivity       /lights/{id}
    zigbee_connectivity       null
    zigbee_device_discovery   null
     */

    #[must_use]
    fn id_v1_scope(&self, id: &Uuid, res: &Resource) -> Option<String> {
        let id = self.state.id_v1(id)?;
        match res {
            Resource::GroupedLight(_) => Some(format!("/groups/{id}")),
            Resource::Light(_) => Some(format!("/lights/{id}")),
            Resource::Scene(_) => Some(format!("/scenes/{id}")),

            /* Rooms map to their grouped_light service's id_v1 */
            Resource::Room(room) => room
                .grouped_light_service()
                .and_then(|glight| self.state.id_v1(&glight.rid))
                .map(|id| format!("/groups/{id}")),

            /* Devices (that are lights) map to the light service's id_v1 */
            Resource::Device(dev) => dev
                .light_service()
                .and_then(|light| self.state.id_v1(&light.rid))
                .map(|id| format!("/lights/{id}")),

            /* BridgeHome maps to "group 0" that seems to be present in the v1 api */
            Resource::BridgeHome(_) => Some(String::from("/groups/0")),

            /* No id v1 */
            Resource::BehaviorInstance(_)
            | Resource::DevicePower(_)
            | Resource::BehaviorScript(_)
            | Resource::Bridge(_)
            | Resource::Button(_)
            | Resource::Entertainment(_)
            | Resource::EntertainmentConfiguration(_)
            | Resource::GeofenceClient(_)
            | Resource::Geolocation(_)
            | Resource::Homekit(_)
            | Resource::LightLevel(_)
            | Resource::Matter(_)
            | Resource::Motion(_)
            | Resource::PublicImage(_)
            | Resource::RelativeRotary(_)
            | Resource::SmartScene(_)
            | Resource::Taurus(_)
            | Resource::Temperature(_)
            | Resource::ZigbeeConnectivity(_)
            | Resource::Zone(_)
            | Resource::ZigbeeDeviceDiscovery(_) => None,
        }
    }

    fn make_resource_record(&self, id: &Uuid, res: &Resource) -> ResourceRecord {
        ResourceRecord::new(*id, self.id_v1_scope(id, res), res)
    }

    pub fn get_resource(&self, ty: RType, id: &Uuid) -> ApiResult<ResourceRecord> {
        self.state
            .res
            .get(id)
            .filter(|res| res.rtype() == ty)
            .map(|res| self.make_resource_record(id, res))
            .ok_or_else(|| ApiError::NotFound(*id))
    }

    pub fn get_resource_by_id(&self, id: &Uuid) -> ApiResult<ResourceRecord> {
        self.state
            .get(id)
            .map(|res| self.make_resource_record(id, res))
    }

    #[must_use]
    pub fn get_resources(&self) -> Vec<ResourceRecord> {
        self.state
            .res
            .iter()
            .map(|(id, res)| self.make_resource_record(id, res))
            .collect()
    }

    #[must_use]
    pub fn get_resources_by_type(&self, ty: RType) -> Vec<ResourceRecord> {
        self.state
            .res
            .iter()
            .filter(|(_, r)| r.rtype() == ty)
            .map(|(id, res)| self.make_resource_record(id, res))
            .collect()
    }

    pub fn get_id_v1_index(&self, uuid: Uuid) -> ApiResult<u32> {
        self.state.id_v1(&uuid).ok_or(ApiError::NotFound(uuid))
    }

    pub fn get_id_v1(&self, uuid: Uuid) -> ApiResult<String> {
        Ok(self.get_id_v1_index(uuid)?.to_string())
    }

    pub fn from_id_v1(&self, id: u32) -> ApiResult<Uuid> {
        self.state.from_id_v1(&id).ok_or(ApiError::V1NotFound(id))
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
    pub fn z2m_channel(&self) -> Receiver<Arc<ClientRequest>> {
        self.z2m_updates.subscribe()
    }

    pub fn z2m_request(&self, req: ClientRequest) -> ApiResult<()> {
        log::debug!("z2m request: {req:#?}");

        self.z2m_updates.send(Arc::new(req))?;

        Ok(())
    }
}
