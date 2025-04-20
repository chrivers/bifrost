use std::collections::HashSet;
use std::io::{Read, Write};
use std::sync::Arc;

use hue::error::{HueError, HueResult};
use maplit::btreeset;
use serde_json::{json, Value};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Notify;
use uuid::{uuid, Uuid};

use hue::api::{
    BehaviorInstanceUpdate, BehaviorScript, BehaviorScriptMetadata, Bridge, BridgeHome, Device,
    DeviceArchetype, DeviceProductData, DeviceUpdate, DimmingUpdate, DollarRef, Entertainment,
    EntertainmentConfiguration, EntertainmentConfigurationLocationsUpdate,
    EntertainmentConfigurationStatus, EntertainmentConfigurationStreamProxyMode,
    EntertainmentConfigurationStreamProxyUpdate, EntertainmentConfigurationUpdate, GroupedLight,
    GroupedLightUpdate, Light, LightMode, LightUpdate, Metadata, On, RType, Resource, ResourceLink,
    ResourceRecord, RoomUpdate, SceneUpdate, Stub, TimeZone, Update, ZigbeeConnectivity,
    ZigbeeConnectivityStatus, ZigbeeDeviceDiscovery,
};
use hue::event::EventBlock;
use hue::version::SwVersion;

use crate::backend::BackendRequest;
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
        self.state_updates.notify_waiters();
    }

    pub fn reset_all_streaming(&mut self) -> ApiResult<()> {
        for id in self.get_resource_ids_by_type(RType::Light) {
            let light: &Light = self.get_id(id)?;
            if light.mode != LightMode::Normal {
                log::warn!("Clearing streaming state of Light {}", id);
                self.update::<Light>(&id, |light| {
                    light.mode = LightMode::Normal;
                })?;
            }
        }

        for id in self.get_resource_ids_by_type(RType::EntertainmentConfiguration) {
            let ec: &EntertainmentConfiguration = self.get_id(id)?;
            if ec.active_streamer.is_some()
                || ec.status != EntertainmentConfigurationStatus::Inactive
            {
                log::warn!("Clearing streaming state of EntertainmentConfiguration {id}");
                self.update::<EntertainmentConfiguration>(&id, |ec| {
                    ec.active_streamer = None;
                    ec.status = EntertainmentConfigurationStatus::Inactive;
                })?;
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

    pub fn init(&mut self, bridge_id: &str) -> ApiResult<()> {
        self.add_bridge(bridge_id.to_owned())?;
        self.add_behavior_scripts()?;
        Ok(())
    }

    pub fn aux_get(&self, link: &ResourceLink) -> HueResult<&AuxData> {
        self.state.aux_get(link)
    }

    pub fn aux_set(&mut self, link: &ResourceLink, aux: AuxData) {
        self.state.aux_set(link, aux);
    }

    fn generate_update(obj: &Resource) -> HueResult<Option<Update>> {
        match obj {
            Resource::Light(light) => {
                let upd = LightUpdate::new()
                    .with_brightness(light.dimming)
                    .with_on(light.on)
                    .with_color_temperature(light.as_mirek_opt())
                    .with_color_xy(light.as_color_opt())
                    .with_gradient(light.as_gradient_opt());

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
            Resource::Device(device) => {
                let upd = DeviceUpdate::new().with_metadata(device.metadata.clone());

                Ok(Some(Update::Device(upd)))
            }
            Resource::Room(room) => {
                let upd = RoomUpdate::new().with_metadata(room.metadata.clone());

                Ok(Some(Update::Room(upd)))
            }
            Resource::BridgeHome(_home) => Ok(None),
            Resource::EntertainmentConfiguration(ent) => {
                let upd = EntertainmentConfigurationUpdate {
                    configuration_type: Some(ent.configuration_type.clone()),
                    metadata: Some(ent.metadata.clone()),
                    action: None,
                    stream_proxy: match ent.stream_proxy.mode {
                        EntertainmentConfigurationStreamProxyMode::Auto => {
                            Some(EntertainmentConfigurationStreamProxyUpdate::Auto)
                        }
                        EntertainmentConfigurationStreamProxyMode::Manual => {
                            debug_assert!(ent.stream_proxy.node.rtype == RType::Entertainment);
                            Some(EntertainmentConfigurationStreamProxyUpdate::Manual {
                                node: ent.stream_proxy.node,
                            })
                        }
                    },
                    locations: Some(EntertainmentConfigurationLocationsUpdate {
                        service_locations: ent
                            .locations
                            .service_locations
                            .clone()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                    }),
                };

                Ok(Some(Update::EntertainmentConfiguration(upd)))
            }
            Resource::BehaviorInstance(behavior_instance) => {
                let upd = BehaviorInstanceUpdate::new()
                    .with_metadata(behavior_instance.metadata.clone())
                    .with_enabled(behavior_instance.enabled)
                    .with_configuration(behavior_instance.configuration.clone());

                Ok(Some(Update::BehaviorInstance(upd)))
            }
            obj => Err(HueError::UpdateUnsupported(obj.rtype())),
        }
    }

    pub fn try_update<T>(
        &mut self,
        id: &Uuid,
        func: impl FnOnce(&mut T) -> ApiResult<()>,
    ) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = HueError>,
    {
        let obj = self.state.get_mut(id)?;
        func(obj.try_into()?)?;

        if let Some(delta) = Self::generate_update(obj)? {
            let id_v1 = self.state.id_v1(id);
            log::trace!("Hue event: {id_v1:?} {delta:#?}");
            self.hue_event_stream
                .hue_event(EventBlock::update(id, id_v1, delta)?);
        }

        self.state_updates.notify_waiters();

        Ok(())
    }

    pub fn update<T>(&mut self, id: &Uuid, func: impl FnOnce(&mut T)) -> ApiResult<()>
    where
        for<'a> &'a mut T: TryFrom<&'a mut Resource, Error = HueError>,
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

        self.state_updates.notify_waiters();

        let evt = EventBlock::add(serde_json::to_value(self.get_resource_by_id(&link.rid)?)?);

        log::trace!("Send event: {evt:?}");

        self.hue_event_stream.hue_event(evt);

        Ok(())
    }

    pub fn delete(&mut self, link: &ResourceLink) -> ApiResult<()> {
        log::info!("Deleting {link:?}..");
        self.state.remove(&link.rid)?;

        self.state_updates.notify_waiters();

        let evt = EventBlock::delete(link)?;

        self.hue_event_stream.hue_event(evt);

        Ok(())
    }

    pub fn add_bridge(&mut self, bridge_id: String) -> ApiResult<()> {
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
            time_zone: TimeZone::best_guess(),
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
            status: String::from("ready"),
            action: Value::Null,
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

    pub fn add_behavior_scripts(&mut self) -> ApiResult<()> {
        let wake_up_link = ResourceLink::new(
            uuid!("ff8957e3-2eb9-4699-a0c8-ad2cb3ede704"),
            RType::BehaviorScript,
        );
        let wake_up = BehaviorScript {
            configuration_schema: DollarRef {
                dref: Some("basic_wake_up_config.json#".to_string()),
            },
            description:
                "Get your body in the mood to wake up by fading on the lights in the morning."
                    .to_string(),
            max_number_instances: None,
            metadata: BehaviorScriptMetadata {
                name: "Basic wake up routine".to_string(),
                category: "automation".to_string(),
            },
            state_schema: DollarRef { dref: None },
            supported_features: vec!["style_sunrise".to_string(), "intensity".to_string()],
            trigger_schema: DollarRef {
                dref: Some("trigger.json#".to_string()),
            },
            version: "0.0.1".to_string(),
        };
        self.add(&wake_up_link, Resource::BehaviorScript(wake_up))?;

        Ok(())
    }

    pub fn get_next_scene_id(&self, room: &ResourceLink) -> HueResult<u32> {
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
            | Resource::DevicePower(_)
            | Resource::DeviceSoftwareUpdate(_)
            | Resource::BehaviorScript(_)
            | Resource::Bridge(_)
            | Resource::Button(_)
            | Resource::GeofenceClient(_)
            | Resource::Geolocation(_)
            | Resource::GroupedMotion(_)
            | Resource::GroupedLightLevel(_)
            | Resource::Homekit(_)
            | Resource::LightLevel(_)
            | Resource::Matter(_)
            | Resource::Motion(_)
            | Resource::PrivateGroup(_)
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
        ResourceRecord::new(*id, self.id_v1_scope(id, res), res.clone())
    }

    pub fn get_resource(&self, ty: RType, id: &Uuid) -> HueResult<ResourceRecord> {
        self.state
            .res
            .get(id)
            .filter(|res| res.rtype() == ty)
            .map(|res| self.make_resource_record(id, res))
            .ok_or(HueError::NotFound(*id))
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
            log::debug!("z2m request: {req:#?}");
        }

        self.backend_updates.send(Arc::new(req))?;

        Ok(())
    }
}
