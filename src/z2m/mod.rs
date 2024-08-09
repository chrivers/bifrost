pub mod api;
pub mod update;

use std::collections::{HashMap, HashSet};
use std::hash::RandomState;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::hue;
use crate::hue::api::{
    ColorTemperatureUpdate, ColorUpdate, Device, DeviceProductData, Dimming, DimmingUpdate,
    GroupedLight, Light, Metadata, On, RType, Resource, ResourceLink, Room, RoomArchetypes, Scene,
    SceneAction, SceneActionElement, SceneMetadata, SceneStatus,
};

use crate::error::{ApiError, ApiResult};
use crate::hue::scene_icons;
use crate::resource::AuxData;
use crate::resource::Resources;
use crate::z2m::api::{Message, Other};
use crate::z2m::update::DeviceUpdate;

#[derive(Debug)]
struct LearnScene {
    pub expire: DateTime<Utc>,
    pub missing: HashSet<Uuid>,
    pub known: HashMap<Uuid, SceneAction>,
}

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, Uuid>,
    learn: HashMap<Uuid, LearnScene>,
}

#[allow(clippy::used_underscore_binding)]
#[derive(Debug, Deserialize)]
struct SceneRecall {
    _scene: ResourceLink,
    _room: ResourceLink,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Z2mLightUpdate {
    device: ResourceLink,
    upd: DeviceUpdate,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Z2mGroupUpdate {
    device: ResourceLink,
    upd: DeviceUpdate,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Z2mSceneStore {
    room: ResourceLink,
    id: u32,
    name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Z2mSceneRecall {
    scene: ResourceLink,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Z2mSceneRemove {
    scene: ResourceLink,
}

#[derive(Clone, Debug, Deserialize)]
pub enum ClientRequest {
    LightUpdate(Z2mLightUpdate),
    GroupUpdate(Z2mGroupUpdate),
    SceneStore(Z2mSceneStore),
    SceneRecall(Z2mSceneRecall),
    SceneRemove(Z2mSceneRemove),
}

impl ClientRequest {
    #[must_use]
    pub const fn light_update(device: ResourceLink, upd: DeviceUpdate) -> Self {
        Self::LightUpdate(Z2mLightUpdate { device, upd })
    }

    #[must_use]
    pub const fn group_update(device: ResourceLink, upd: DeviceUpdate) -> Self {
        Self::GroupUpdate(Z2mGroupUpdate { device, upd })
    }

    #[must_use]
    pub const fn scene_remove(scene: ResourceLink) -> Self {
        Self::SceneRemove(Z2mSceneRemove { scene })
    }

    #[must_use]
    pub const fn scene_recall(scene: ResourceLink) -> Self {
        Self::SceneRecall(Z2mSceneRecall { scene })
    }

    #[must_use]
    pub const fn scene_store(room: ResourceLink, id: u32, name: String) -> Self {
        Self::SceneStore(Z2mSceneStore { room, id, name })
    }
}

impl Client {
    pub async fn new(conn: &str, state: Arc<Mutex<Resources>>) -> ApiResult<Self> {
        let (socket, _) = connect_async(conn).await?;
        let map = HashMap::new();
        let learn = HashMap::new();
        Ok(Self {
            socket,
            state,
            map,
            learn,
        })
    }

    pub async fn add_light(&mut self, dev: &api::Device) -> ApiResult<()> {
        let name = &dev.friendly_name;

        let link_device = RType::Device.deterministic(&dev.ieee_address);
        let link_light = RType::Light.deterministic(&dev.ieee_address);

        let dev = hue::api::Device {
            product_data: DeviceProductData::hue_color_spot(),
            metadata: Metadata::spot_bulb(name),
            identify: json!({}),
            services: vec![link_light],
        };

        self.map.insert(name.to_string(), link_light.rid);

        let mut res = self.state.lock().await;
        let mut light = Light::new(link_device);
        light.metadata.name = name.to_string();

        res.aux_set(&link_light, AuxData::new().with_topic(name));
        res.add(&link_device, Resource::Device(dev))?;
        res.add(&link_light, Resource::Light(light))?;
        drop(res);

        Ok(())
    }

    pub async fn add_group(&mut self, grp: &crate::z2m::api::Group) -> ApiResult<()> {
        let link_room = RType::Room.deterministic(&grp.friendly_name);
        let link_glight = RType::GroupedLight.deterministic((link_room.rid, grp.id));

        let children = grp
            .members
            .iter()
            .map(|f| RType::Device.deterministic(&f.ieee_address))
            .collect();

        let mut services = vec![link_glight];

        let topic = grp.friendly_name.to_string();

        let mut res = self.state.lock().await;

        for scn in &grp.scenes {
            let scene = Scene {
                actions: vec![],
                auto_dynamic: false,
                group: link_room,
                metadata: SceneMetadata {
                    appdata: None,
                    image: guess_scene_icon(&scn.name),
                    name: scn.name.to_string(),
                },
                palette: json!({
                    "color": [],
                    "dimming": [],
                    "color_temperature": [],
                    "effects": [],
                }),
                speed: 0.5,
                status: Some(SceneStatus::Inactive),
            };

            let link_scene = RType::Scene.deterministic((link_room.rid, scn.id));

            res.aux_set(
                &link_scene,
                AuxData::new().with_topic(&topic).with_index(scn.id),
            );

            services.push(link_scene);
            res.add(&link_scene, Resource::Scene(scene))?;
        }

        if let Ok(room) = res.get::<Room>(&link_room) {
            log::debug!("{link_room:?} is known, updating..");
            let new: HashSet<&ResourceLink, RandomState> = HashSet::from_iter(&services[..]);
            let old: HashSet<&ResourceLink, RandomState> = HashSet::from_iter(&room.services[..]);
            let gone = old.difference(&new);
            for rlink in gone {
                log::debug!("Deleting orphaned {rlink:?} in {link_room:?}");
                res.delete(rlink)?;
            }
        } else {
            log::debug!("{link_room:?} is new, adding..");
        }

        let room = Room {
            children,
            metadata: Metadata::room(RoomArchetypes::Computer, &topic),
            services,
        };

        self.map.insert(topic, link_glight.rid);
        res.add(&link_room, Resource::Room(room))?;

        let glight = GroupedLight {
            alert: Value::Null,
            dimming: None,
            on: On { on: true },
            owner: link_room,
            signaling: Value::Null,
        };

        res.add(&link_glight, Resource::GroupedLight(glight))?;
        drop(res);

        Ok(())
    }

    pub async fn handle_update(&mut self, rid: &Uuid, payload: Value) -> ApiResult<()> {
        let upd: DeviceUpdate = serde_json::from_value(payload)?;

        let obj = self.state.lock().await.get_resource_by_id(rid)?.obj;
        match obj {
            Resource::Light(_) => {
                if let Err(e) = self.handle_update_light(rid, &upd).await {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            Resource::GroupedLight(_) => {
                if let Err(e) = self.handle_update_grouped_light(rid, &upd).await {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_update_light(&mut self, uuid: &Uuid, upd: &DeviceUpdate) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        res.update::<Light>(uuid, move |light| {
            if let Some(state) = &upd.state {
                light.on.on = (*state).into();
            }

            if let Some(b) = upd.brightness {
                light.dimming = Some(Dimming {
                    brightness: b / 254.0 * 100.0,
                    min_dim_level: None,
                });
            }

            light.color_mode = upd.color_mode;

            if let Some(ct) = upd.color_temp {
                light.color_temperature.mirek = ct;
                /* light.color_temperature.mirek_valid = true; */
            }

            if let Some(col) = upd.color {
                light.color.xy = col.xy;
                /* light.color_temperature.mirek_valid = false; */
            }
        })?;

        for learn in self.learn.values_mut() {
            if learn.missing.remove(uuid) {
                let rlink = RType::Light.link_to(*uuid);
                let light = res.get::<Light>(&rlink)?;
                let mut color_temperature = None;
                let mut color = None;
                if let Some(col) = upd.color {
                    color = Some(ColorUpdate { xy: col.xy });
                } else if let Some(mirek) = upd.color_temp {
                    color_temperature = Some(ColorTemperatureUpdate { mirek });
                }

                learn.known.insert(
                    *uuid,
                    SceneAction {
                        color,
                        color_temperature,
                        dimming: light.dimming.map(|b| DimmingUpdate {
                            brightness: b.brightness,
                        }),
                        on: Some(light.on),
                    },
                );
            }
            log::info!("Learn: {learn:?}");
        }

        let keys: Vec<Uuid> = self.learn.keys().copied().collect();
        for uuid in &keys {
            if self.learn[uuid].missing.is_empty() {
                let lscene = self.learn.remove(uuid).unwrap();
                log::info!("Learned all lights {uuid}");
                let actions: Vec<SceneActionElement> = lscene
                    .known
                    .into_iter()
                    .map(|(uuid, action)| SceneActionElement {
                        action,
                        target: RType::Light.link_to(uuid),
                    })
                    .collect();
                res.update(uuid, move |scene: &mut Scene| {
                    scene.actions = actions;
                })?;
            }
        }
        drop(res);

        Ok(())
    }

    async fn handle_update_grouped_light(&self, uuid: &Uuid, upd: &DeviceUpdate) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        res.update::<GroupedLight>(uuid, move |glight| {
            if let Some(state) = &upd.state {
                glight.on.on = (*state).into();
            }

            if let Some(b) = upd.brightness {
                glight.dimming = Some(DimmingUpdate {
                    brightness: b / 254.0 * 100.0,
                });
            }
        })
    }

    async fn handle_message(&mut self, msg: Message) -> ApiResult<()> {
        match msg {
            /* Message::BridgeInfo(ref obj) => { */
            /*     println!("{obj:#?}"); */
            /* } */
            /* Message::BridgeLogging(ref obj) => { */
            /*     println!("{obj:#?}"); */
            /* } */
            /* Message::BridgeExtensions(ref obj) => { */
            /*     println!("{obj:#?}"); */
            /* } */
            Message::BridgeDevices(ref obj) => {
                //println!("{obj:#?}");
                for dev in obj {
                    if dev.expose_light() {
                        log::info!(
                            "Adding light {:?}: [{}] ({})",
                            dev.ieee_address,
                            dev.friendly_name,
                            dev.model_id.as_deref().unwrap_or("<unknown model>")
                        );
                        self.add_light(dev).await?;
                    }
                }
            }

            Message::BridgeGroups(ref obj) => {
                /* println!("{obj:#?}"); */
                for grp in obj {
                    self.add_group(grp).await?;
                }
            }

            /* Message::BridgeDefinitions(ref obj) => { */
            /*     /\* println!("{obj:#?}"); *\/ */
            /* } */

            /* Message::BridgeState(ref obj) => { */
            /*     /\* println!("{obj:#?}"); *\/ */
            /* } */
            Message::Other(obj) => {
                if obj.topic.contains('/') {
                    return Ok(());
                }

                let Some(ref val) = self.map.get(&obj.topic).copied() else {
                    log::warn!("Notification on unknown topic {}", &obj.topic);
                    return Ok(());
                };

                self.handle_update(val, obj.payload).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn websocket_read(&mut self, pkt: tungstenite::Message) -> ApiResult<()> {
        let tungstenite::Message::Text(txt) = pkt else {
            log::error!("Received non-text message on websocket :(");
            return Err(ApiError::UnexpectedZ2mReply(pkt));
        };

        let data = serde_json::from_str(&txt);

        match data {
            Ok(msg) => self.handle_message(msg).await,
            Err(err) => {
                log::error!(
                    "Invalid websocket message: {:#?} [{}..]",
                    err,
                    &txt.chars().take(128).collect::<String>()
                );
                Err(err)?
            }
        }
    }

    fn learn_cleanup(&mut self) {
        let now = Utc::now();
        self.learn.retain(|uuid, lscene| {
            let res = lscene.expire < now;
            if !res {
                log::warn!("Failed to learn scene {uuid} before deadline");
            }
            res
        });
    }

    async fn learn_scene_recall(&mut self, upd: SceneRecall) -> ApiResult<()> {
        log::info!("recall scene: {upd:?}");
        let lock = self.state.lock().await;
        let scene: Scene = lock.get(&upd._scene)?;

        if scene.actions.is_empty() {
            let room: Room = lock.get(&upd._room)?;

            let devices: Vec<Device> = room
                .children
                .iter()
                .filter_map(|rl| lock.get(rl).ok())
                .collect();

            let lights: Vec<Uuid> = devices
                .iter()
                .filter_map(Device::light)
                .map(|rl| rl.rid)
                .collect();

            drop(lock);

            log::info!("{scene:?}");
            log::info!("{room:?}");
            log::info!("{lights:#?}");

            let learn = LearnScene {
                expire: Utc::now() + Duration::seconds(5),
                missing: HashSet::from_iter(lights),
                known: HashMap::new(),
            };
            self.learn.insert(upd._scene.rid, learn);
        }

        Ok(())
    }

    #[allow(clippy::single_match_else)]
    async fn websocket_write(&mut self, api_req: Other) -> ApiResult<()> {
        self.learn_cleanup();

        let topic = api_req
            .topic
            .as_str()
            .strip_suffix("/set")
            .unwrap_or(&api_req.topic);
        match self.map.get(topic) {
            Some(uuid) => {
                log::trace!(
                    "Topic [{topic}] known as {uuid} on this z2m connection, sending event.."
                );
                if let Ok(upd) = serde_json::from_value(api_req.payload.clone()) {
                    self.learn_scene_recall(upd).await?;
                }
                let msg = tungstenite::Message::Text(serde_json::to_string(&api_req)?);
                Ok(self.socket.send(msg).await?)
            }
            None => {
                log::trace!("Topic [{topic}] unknown on this z2m connection");
                Ok(())
            }
        }
    }

    pub async fn event_loop(mut self) -> ApiResult<()> {
        let mut chan = self.state.lock().await.z2m_channel();
        loop {
            let res = select! {
                pkt = chan.recv() => {
                    let api_req = pkt?;
                    self.websocket_write(api_req).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    Ok(())
                },
                pkt = self.socket.next() => {
                    self.websocket_read(pkt.ok_or(ApiError::UnexpectedZ2mEof)??).await
                },
            };

            if let Err(ref err) = res {
                log::error!("Event loop failed!: {err:?}");
                return res;
            }
        }
    }
}

fn guess_scene_icon(name: &str) -> Option<ResourceLink> {
    let icon = match name {
        "Bright" => scene_icons::BRIGHT,
        "Relax" => scene_icons::RELAX,
        "Night" => scene_icons::NIGHT_LIGHT,
        _ => return None,
    };

    Some(ResourceLink {
        rid: icon,
        rtype: RType::PublicImage,
    })
}
