use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::Arc;

use chrono_tz::Tz;
use itertools::Itertools;
use maplit::btreeset;
use serde::Serialize;
use serde_json::json;
use tokio::sync::Notify;
use tokio::sync::broadcast::{Receiver, Sender};
use uuid::Uuid;

use bifrost_api::backend::BackendRequest;
use hue::api::{
    Bridge, BridgeHome, Device, DeviceArchetype, DeviceProductData, DimmingUpdate, Entertainment,
    EntertainmentConfiguration, GroupedLight, Light, Metadata, On, RType, Resource, ResourceLink,
    ResourceRecord, Room, Stub, ZigbeeConnectivity, ZigbeeConnectivityStatus,
    ZigbeeDeviceDiscovery, ZigbeeDeviceDiscoveryAction, ZigbeeDeviceDiscoveryStatus, Zone,
};
use hue::error::{HueError, HueResult};
use hue::event::EventBlock;
use hue::version::SwVersion;

use crate::error::ApiResult;
use crate::model::state::{AuxData, State};
use crate::server::hueevents::HueEventStream;

#[derive(Clone, Debug)]
pub struct Resources {
    state: State,
    version: SwVersion,
    state_updates: Arc<Notify>,
    backend_updates: Sender<Arc<BackendRequest>>,
    hue_event_stream: HueEventStream,
}

impl Resources {
    const MAX_SCENE_ID: u32 = 100;
    const HUE_EVENTS_BUFFER_SIZE: usize = 128;

    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new(version: SwVersion, state: State) -> Self {
        Self {
            state,
            version,
            state_updates: Arc::new(Notify::new()),
            backend_updates: Sender::new(32),
            hue_event_stream: HueEventStream::new(Self::HUE_EVENTS_BUFFER_SIZE),
        }
    }

    pub fn update_bridge_version(&mut self, version: SwVersion) {
        self.version = version;
        self.state.patch_bridge_version(&self.version);
        self.state_updates.notify_one();
    }

    pub fn reset_all_streaming(&mut self) -> ApiResult<()> {
        for id in self.get_resource_ids_by_type(RType::Light) {
            let light: &Light = self.get_id(id)?;
            if light.is_streaming() {
                log::warn!("Clearing streaming state of Light {}", id);
                self.update(&id, Light::stop_streaming)?;
            }
        }

        for id in self.get_resource_ids_by_type(RType::EntertainmentConfiguration) {
            let ec: &EntertainmentConfiguration = self.get_id(id)?;
            if ec.is_streaming() {
                log::warn!("Clearing streaming state of EntertainmentConfiguration {id}");
                self.update(&id, EntertainmentConfiguration::stop_streaming)?;
            }
        }

        Ok(())
    }

    pub fn read(&mut self, rdr: impl Read) -> ApiResult<()> {
        self.state = State::from_reader(rdr)?;
        Ok(())
    }

    pub fn write(&self, wr: impl Write) -> ApiResult<()> {
        Ok(serde_yml::to_writer(wr, &self.state)?)
    }

    pub fn serialize(&self) -> ApiResult<String> {
        Ok(serde_yml::to_string(&self.state)?)
    }

    pub fn init(&mut self, bridge_id: &str, timezone: Tz) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned(), timezone)
    }

    pub fn aux_get(&self, link: &ResourceLink) -> ApiResult<&AuxData> {
        self.state.aux_get(&link.rid)
    }

    pub fn aux_set(&mut self, link: &ResourceLink, aux: AuxData) {
        self.state.aux_set(link.rid, aux);
    }

    pub fn try_update<T: Serialize>(
        &mut self,
        id: &Uuid,
        func: impl FnOnce(&mut T) -> ApiResult<()>,
    ) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = HueError>,
    {
        let id_v1 = self.id_v1_scope(id, self.state.get(id)?);
        let resource = self.state.get_mut(id)?;

        let obj: &mut T = resource.try_into()?;

        // capture before and after serializations of object
        let before = serde_json::to_value(&obj)?;
        func(obj)?;
        let after = serde_json::to_value(&obj)?;

        // if the function affected a meaningful difference, send an update event
        if let Some(delta) = hue::diff::event_update_diff(before, after)? {
            log::trace!("Hue event: {id_v1:?} {delta:#?}");
            self.hue_event_stream.hue_event(EventBlock::update(
                id,
                id_v1,
                resource.rtype(),
                delta,
            )?);

            self.state_updates.notify_one();
        }

        Ok(())
    }

    pub fn update<T: Serialize>(&mut self, id: &Uuid, func: impl FnOnce(&mut T)) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = HueError>,
    {
        self.try_update(id, |obj: &mut T| {
            func(obj);
            Ok(())
        })
    }

    pub fn update_by_type<T: Serialize>(&mut self, func: impl Fn(&mut T)) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = HueError>,
    {
        let ids = self.state.res.keys().copied().collect_vec();
        for id in &ids {
            let obj = self.state.get_mut(id)?;
            let x: Result<&mut T, _> = obj.try_into();
            if x.is_ok() {
                self.try_update(id, |obj: &mut T| {
                    func(obj);
                    Ok(())
                })?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn get_scenes_for_room(&self, id: &Uuid) -> Vec<Uuid> {
        self.state
            .res
            .iter()
            .filter_map(|(k, v)| {
                if let Resource::Scene(scn) = v {
                    if &scn.group.rid == id { Some(k) } else { None }
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

        let evt = EventBlock::add(vec![self.get_resource_by_id(&link.rid)?]);

        log::trace!("Send event: {evt:?}");

        self.hue_event_stream.hue_event(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        log::info!("Deleting {link:?}..");

        // Delete references to this object from other objects
        self.update_by_type(|bridge_home: &mut BridgeHome| {
            bridge_home.children.remove(link);
            bridge_home.services.remove(link);
        })?;

        self.update_by_type(|device: &mut Device| {
            device.services.remove(link);
        })?;

        self.update_by_type(|ec: &mut EntertainmentConfiguration| {
            ec.locations
                .service_locations
                .retain(|sl| sl.service != *link);
            ec.channels
                .retain(|chan| !chan.members.iter().any(|c| c.service == *link));
            ec.light_services.retain(|ls| ls != link);
        })?;

        self.update_by_type(|room: &mut Room| {
            room.children.remove(link);
            room.services.remove(link);
        })?;

        self.update_by_type(|zone: &mut Zone| {
            zone.children.remove(link);
            zone.services.remove(link);
        })?;

        // Get id_v1 before deleting
        let id_v1 = self.id_v1_scope(&link.rid, self.state.get(&link.rid)?);

        // Remove resource from state database
        self.state.remove(&link.rid)?;

        // Find ids of all resources owned by the deleted node
        let owned_by = self
            .state
            .res
            .iter()
            .filter_map(|(rid, res)| {
                if res.owner() == Some(*link) {
                    Some(ResourceLink::new(*rid, res.rtype()))
                } else {
                    None
                }
            })
            .collect_vec();

        // Delete all resources owned by the deleted node
        for owned in owned_by {
            self.delete(&owned)?;
        }

        self.state_updates.notify_one();

        let evt = EventBlock::delete(*link, id_v1)?;

        self.hue_event_stream.hue_event(evt);

        Ok(())
    }

    pub fn add_bridge(&mut self, bridge_id: String, timezone: Tz) -> ApiResult<()> {
        let link_bridge = RType::Bridge.deterministic(&bridge_id);
        let link_bridge_home = RType::BridgeHome.deterministic(format!("{bridge_id}HOME"));
        let link_bridge_dev = RType::Device.deterministic(link_bridge.rid);
        let link_bridge_home_dev = RType::Device.deterministic(link_bridge_home.rid);
        let link_bridge_ent = RType::Entertainment.deterministic(link_bridge.rid);
        let link_zbdd = RType::ZigbeeDeviceDiscovery.deterministic(link_bridge.rid);
        let link_zbc = RType::ZigbeeConnectivity.deterministic(link_bridge.rid);
        let link_bhome_glight = RType::GroupedLight.deterministic(link_bridge_home.rid);

        let bridge_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(&self.version),
            metadata: Metadata::new(DeviceArchetype::BridgeV2, "Bifrost"),
            services: btreeset![link_bridge, link_zbc, link_bridge_ent, link_zbdd],
            identify: Some(Stub),
            usertest: None,
        };

        let bridge = Bridge {
            bridge_id,
            owner: link_bridge_dev,
            time_zone: timezone.into(),
        };

        let bridge_home_dev = Device {
            product_data: DeviceProductData::hue_bridge_v2(&self.version),
            metadata: Metadata::new(DeviceArchetype::BridgeV2, "Bifrost Bridge Home"),
            services: btreeset![link_bridge],
            identify: None,
            usertest: None,
        };

        let bridge_home = BridgeHome {
            children: btreeset![link_bridge_dev],
            services: btreeset![link_bhome_glight],
        };

        let bhome_glight = GroupedLight {
            alert: json!({
                "action_values": [
                    "breathe",
                ]
            }),
            dimming: Some(DimmingUpdate { brightness: 8.7 }),
            color: Some(Stub),
            color_temperature: Some(Stub),
            color_temperature_delta: Some(Stub),
            dimming_delta: Stub,
            dynamics: Stub,
            on: Some(On { on: true }),
            owner: link_bridge_home,
            signaling: json!({
                "signal_values": [
                    "alternating",
                    "no_signal",
                    "on_off",
                    "on_off_color",
                ]
            }),
        };

        let zbdd = ZigbeeDeviceDiscovery {
            owner: link_bridge_dev,
            status: ZigbeeDeviceDiscoveryStatus::Ready,
            action: ZigbeeDeviceDiscoveryAction {
                action_type_values: vec![],
                search_codes: vec![],
            },
        };

        let zbc = ZigbeeConnectivity {
            owner: link_bridge_dev,
            mac_address: String::from("11:22:33:44:55:66:77:88"),
            status: ZigbeeConnectivityStatus::Connected,
            channel: Some(json!({
                "status": "set",
                "value": "channel_25",
            })),
            extended_pan_id: None,
        };

        let brent = Entertainment {
            equalizer: false,
            owner: link_bridge_dev,
            proxy: true,
            renderer: false,
            max_streams: Some(1),
            renderer_reference: None,
            segments: None,
        };

        self.add(&link_bridge_dev, Resource::Device(bridge_dev))?;
        self.add(&link_bridge, Resource::Bridge(bridge))?;
        self.add(&link_bridge_home_dev, Resource::Device(bridge_home_dev))?;
        self.add(&link_bridge_home, Resource::BridgeHome(bridge_home))?;
        self.add(&link_zbdd, Resource::ZigbeeDeviceDiscovery(zbdd))?;
        self.add(&link_zbc, Resource::ZigbeeConnectivity(zbc))?;
        self.add(&link_bridge_ent, Resource::Entertainment(brent))?;
        self.add(&link_bhome_glight, Resource::GroupedLight(bhome_glight))?;

        Ok(())
    }

    pub fn get_next_scene_id(&self, room: &ResourceLink) -> HueResult<u32> {
        let mut set: HashSet<u32> = HashSet::new();

        for scene in self.get_resources_by_type(RType::Scene) {
            let Resource::Scene(scn) = scene.obj else {
                continue;
            };

            if &scn.group == room {
                let Ok(AuxData {
                    index: Some(index), ..
                }) = self.state.aux_get(&scene.id)
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
        Err(HueError::Full(RType::Scene))
    }

    pub fn get<'a, T>(&'a self, link: &ResourceLink) -> HueResult<&'a T>
    where
        &'a T: TryFrom<&'a Resource, Error = HueError>,
    {
        self.get_id(link.rid)
    }

    pub fn get_id<'a, T>(&'a self, id: Uuid) -> HueResult<&'a T>
    where
        &'a T: TryFrom<&'a Resource, Error = HueError>,
    {
        self.state.get(&id)?.try_into()
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
            Resource::Light(_) => Some(format!("/lights/{id}")),
            Resource::Scene(_) => Some(format!("/scenes/{id}")),

            /* GroupedLights are mapped to their (room) owner's id_v1 */
            Resource::GroupedLight(grp) => {
                let id = self.state.id_v1(&grp.owner.rid)?;
                Some(format!("/groups/{id}"))
            }

            /* Rooms are mapped directly */
            Resource::Room(_) => Some(format!("/groups/{id}")),

            /* Devices (that are lights) map to the light service's id_v1 */
            Resource::Device(dev) => dev
                .light_service()
                .and_then(|light| self.state.id_v1(&light.rid))
                .map(|id| format!("/lights/{id}")),

            Resource::EntertainmentConfiguration(_dev) => Some(format!("/groups/{id}")),

            Resource::Entertainment(ent) => {
                let dev: &Device = self.get(&ent.owner).ok()?;
                dev.light_service()
                    .and_then(|light| self.state.id_v1(&light.rid))
                    .map(|id| format!("/lights/{id}"))
            }

            /* BridgeHome maps to "group 0" that seems to be present in the v1 api */
            Resource::BridgeHome(_) => Some(String::from("/groups/0")),

            /* No id v1 */
            Resource::AuthV1(_)
            | Resource::BehaviorInstance(_)
            | Resource::BehaviorScript(_)
            | Resource::Bridge(_)
            | Resource::Button(_)
            | Resource::CameraMotion(_)
            | Resource::Contact(_)
            | Resource::DevicePower(_)
            | Resource::DeviceSoftwareUpdate(_)
            | Resource::GeofenceClient(_)
            | Resource::Geolocation(_)
            | Resource::GroupedLightLevel(_)
            | Resource::GroupedMotion(_)
            | Resource::Homekit(_)
            | Resource::LightLevel(_)
            | Resource::Matter(_)
            | Resource::MatterFabric(_)
            | Resource::Motion(_)
            | Resource::PrivateGroup(_)
            | Resource::PublicImage(_)
            | Resource::RelativeRotary(_)
            | Resource::ServiceGroup(_)
            | Resource::SmartScene(_)
            | Resource::Tamper(_)
            | Resource::Taurus(_)
            | Resource::Temperature(_)
            | Resource::ZgpConnectivity(_)
            | Resource::ZigbeeConnectivity(_)
            | Resource::ZigbeeDeviceDiscovery(_)
            | Resource::Zone(_) => None,
        }
    }

    fn make_resource_record(&self, id: &Uuid, res: &Resource) -> ResourceRecord {
        ResourceRecord::new(*id, self.id_v1_scope(id, res), res.clone())
    }

    pub fn get_resource(&self, rlink: &ResourceLink) -> HueResult<ResourceRecord> {
        self.state
            .res
            .get(&rlink.rid)
            .filter(|res| res.rtype() == rlink.rtype)
            .map(|res| self.make_resource_record(&rlink.rid, res))
            .ok_or(HueError::NotFound(rlink.rid))
    }

    pub fn get_resource_by_id(&self, id: &Uuid) -> HueResult<ResourceRecord> {
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

    #[must_use]
    pub fn get_resource_ids_by_type(&self, ty: RType) -> Vec<Uuid> {
        self.state
            .res
            .iter()
            .filter(|(_, r)| r.rtype() == ty)
            .map(|(id, _res)| *id)
            .collect()
    }

    #[must_use]
    pub fn get_resources_by_owner(&self, owner: ResourceLink) -> Vec<ResourceRecord> {
        self.state
            .res
            .iter()
            .filter(|(_, r)| r.owner() == Some(owner))
            .map(|(id, res)| self.make_resource_record(id, res))
            .collect()
    }

    pub fn get_id_v1_index(&self, uuid: Uuid) -> HueResult<u32> {
        self.state.id_v1(&uuid).ok_or(HueError::NotFound(uuid))
    }

    pub fn get_id_v1(&self, uuid: Uuid) -> HueResult<String> {
        Ok(self.get_id_v1_index(uuid)?.to_string())
    }

    pub fn from_id_v1(&self, id: u32) -> HueResult<Uuid> {
        self.state.from_id_v1(&id).ok_or(HueError::V1NotFound(id))
    }

    #[must_use]
    pub fn state_channel(&self) -> Arc<Notify> {
        self.state_updates.clone()
    }

    #[must_use]
    pub const fn hue_event_stream(&self) -> &HueEventStream {
        &self.hue_event_stream
    }

    #[must_use]
    pub fn backend_event_stream(&self) -> Receiver<Arc<BackendRequest>> {
        self.backend_updates.subscribe()
    }

    pub fn backend_request(&self, req: BackendRequest) -> ApiResult<()> {
        if !matches!(req, BackendRequest::EntertainmentFrame(_)) {
            log::debug!("Backend request: {req:#?}");
        }

        self.backend_updates.send(Arc::new(req))?;

        Ok(())
    }
}
