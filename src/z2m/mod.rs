pub mod api;
pub mod update;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::hue;
use crate::hue::api::{
    Button, ButtonData, ButtonMetadata, ButtonReport, ColorTemperature, ColorTemperatureUpdate,
    ColorUpdate, Device, DeviceArchetype, DeviceProductData, Dimming, DimmingUpdate, GroupedLight,
    Light, Metadata, MirekSchema, On, RType, Resource, ResourceLink, Room, RoomArchetype,
    RoomMetadata, Scene, SceneAction, SceneActionElement, SceneMetadata, SceneStatus,
    ZigbeeConnectivity, ZigbeeConnectivityStatus,
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
    name: String,
    conn: String,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, Uuid>,
    learn: HashMap<Uuid, LearnScene>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum ClientRequest {
    LightUpdate {
        device: ResourceLink,
        upd: DeviceUpdate,
    },

    GroupUpdate {
        device: ResourceLink,
        upd: DeviceUpdate,
    },

    SceneStore {
        room: ResourceLink,
        id: u32,
        name: String,
    },

    SceneRecall {
        scene: ResourceLink,
    },

    SceneRemove {
        scene: ResourceLink,
    },
}

impl ClientRequest {
    #[must_use]
    pub const fn light_update(device: ResourceLink, upd: DeviceUpdate) -> Self {
        Self::LightUpdate { device, upd }
    }

    #[must_use]
    pub const fn group_update(device: ResourceLink, upd: DeviceUpdate) -> Self {
        Self::GroupUpdate { device, upd }
    }

    #[must_use]
    pub const fn scene_remove(scene: ResourceLink) -> Self {
        Self::SceneRemove { scene }
    }

    #[must_use]
    pub const fn scene_recall(scene: ResourceLink) -> Self {
        Self::SceneRecall { scene }
    }

    #[must_use]
    pub const fn scene_store(room: ResourceLink, id: u32, name: String) -> Self {
        Self::SceneStore { room, id, name }
    }
}

impl Client {
    pub fn new(name: String, conn: String, state: Arc<Mutex<Resources>>) -> ApiResult<Self> {
        let map = HashMap::new();
        let learn = HashMap::new();
        Ok(Self {
            name,
            conn,
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
            product_data: DeviceProductData::guess_from_device(dev),
            metadata: Metadata::new(DeviceArchetype::SpotBulb, name),
            services: vec![link_light],
        };

        self.map.insert(name.to_string(), link_light.rid);

        let mut res = self.state.lock().await;
        let mut light = Light::new(link_device, dev.metadata.clone());
        light.metadata.name = name.to_string();

        res.aux_set(&link_light, AuxData::new().with_topic(name));
        res.add(&link_device, Resource::Device(dev))?;
        res.add(&link_light, Resource::Light(light))?;
        drop(res);

        Ok(())
    }

    pub async fn add_switch(&mut self, dev: &api::Device) -> ApiResult<()> {
        let name = &dev.friendly_name;

        let link_device = RType::Device.deterministic(&dev.ieee_address);
        let link_button = RType::Button.deterministic(&dev.ieee_address);
        let link_zbc = RType::ZigbeeConnectivity.deterministic(&dev.ieee_address);

        let dev = hue::api::Device {
            product_data: DeviceProductData::guess_from_device(dev),
            metadata: Metadata::new(DeviceArchetype::UnknownArchetype, "foo"),
            services: vec![link_button, link_zbc],
        };

        self.map.insert(name.to_string(), link_button.rid);

        let mut res = self.state.lock().await;
        let button = Button {
            owner: link_device,
            metadata: ButtonMetadata { control_id: 0 },
            button: ButtonData {
                button_report: Some(ButtonReport {
                    updated: Utc::now(),
                    event: String::from("initial_press"),
                }),
                repeat_interval: Some(100),
                event_values: Some(json!(["initial_press", "repeat"])),
            },
        };

        let zbc = ZigbeeConnectivity {
            owner: link_device,
            mac_address: String::from("11:22:33:44:55:66:77:89"),
            status: ZigbeeConnectivityStatus::ConnectivityIssue,
            channel: Some(json!({
                "status": "set",
                "value": "channel_25",
            })),
            extended_pan_id: String::from("0123456789abcdef"),
        };

        res.add(&link_device, Resource::Device(dev))?;
        res.add(&link_button, Resource::Button(button))?;
        res.add(&link_zbc, Resource::ZigbeeConnectivity(zbc))?;
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

        let topic = grp.friendly_name.to_string();

        let mut res = self.state.lock().await;

        let mut scenes_new = HashSet::new();

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

            scenes_new.insert(link_scene.rid);
            res.add(&link_scene, Resource::Scene(scene))?;
        }

        if let Ok(room) = res.get::<Room>(&link_room) {
            log::info!(
                "[{}] {link_room:?} ({}) known, updating..",
                self.name,
                room.metadata.name
            );

            let scenes_old: HashSet<Uuid> =
                HashSet::from_iter(res.get_scenes_for_room(&link_room.rid));

            log::trace!("[{}] old scenes: {scenes_old:?}", self.name);
            log::trace!("[{}] new scenes: {scenes_new:?}", self.name);
            let gone = scenes_old.difference(&scenes_new);
            log::trace!("[{}]   deleted: {gone:?}", self.name);
            for uuid in gone {
                log::debug!(
                    "[{}] Deleting orphaned {uuid:?} in {link_room:?}",
                    self.name
                );
                let _ = res.delete(&RType::Scene.link_to(*uuid));
            }
        } else {
            log::debug!("[{}] {link_room:?} is new, adding..", self.name);
        }

        let room = Room {
            children,
            metadata: RoomMetadata::new(RoomArchetype::Computer, &topic),
            services: vec![link_glight],
        };

        self.map.insert(topic.clone(), link_glight.rid);

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
                match &mut light.color_temperature {
                    Some(lct) => lct.mirek = ct,
                    None => {
                        light.color_temperature = Some(ColorTemperature {
                            mirek: ct,
                            mirek_schema: MirekSchema::DEFAULT,
                            mirek_valid: true,
                        });
                    }
                }
                /* light.color_temperature.mirek_valid = true; */
            }

            if let Some(col) = upd.color {
                match &mut light.color {
                    Some(lcol) => lcol.xy = col.xy,
                    None => {}
                }
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
            log::info!("[{}] Learn: {learn:?}", self.name);
        }

        let keys: Vec<Uuid> = self.learn.keys().copied().collect();
        for uuid in &keys {
            if self.learn[uuid].missing.is_empty() {
                let lscene = self.learn.remove(uuid).unwrap();
                log::info!("[{}] Learned all lights {uuid}", self.name);
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
                    if dev.expose_light().is_some() {
                        log::info!(
                            "[{}] Adding light {:?}: [{}] ({})",
                            self.name,
                            dev.ieee_address,
                            dev.friendly_name,
                            dev.model_id.as_deref().unwrap_or("<unknown model>")
                        );
                        self.add_light(dev).await?;
                    }
                    /*
                    if dev.expose_action() {
                        log::info!(
                            "[{}] Adding switch {:?}: [{}] ({})",
                            self.name,
                            dev.ieee_address,
                            dev.friendly_name,
                            dev.model_id.as_deref().unwrap_or("<unknown model>")
                        );
                        self.add_switch(dev).await?;
                    }
                    */
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
                    log::warn!(
                        "[{}] Notification on unknown topic {}",
                        self.name,
                        &obj.topic
                    );
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
            log::error!("[{}] Received non-text message on websocket :(", self.name);
            return Err(ApiError::UnexpectedZ2mReply(pkt));
        };

        let data = serde_json::from_str(&txt);

        match data {
            Ok(msg) => self.handle_message(msg).await,
            Err(err) => {
                log::error!(
                    "[{}] Invalid websocket message: {:#?} [{}..]",
                    self.name,
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
                log::warn!(
                    "[{}] Failed to learn scene {uuid} before deadline",
                    self.name
                );
            }
            res
        });
    }

    async fn learn_scene_recall(&mut self, lscene: &ResourceLink) -> ApiResult<()> {
        log::info!("[{}] Recall scene: {lscene:?}", self.name);
        let lock = self.state.lock().await;
        let scene: &Scene = lock.get(lscene)?;

        if scene.actions.is_empty() {
            let room: &Room = lock.get(&scene.group)?;

            let lights: Vec<Uuid> = room
                .children
                .iter()
                .filter_map(|rl| lock.get(rl).ok())
                .filter_map(Device::light_service)
                .map(|rl| rl.rid)
                .collect();

            drop(lock);

            let learn = LearnScene {
                expire: Utc::now() + Duration::seconds(5),
                missing: HashSet::from_iter(lights),
                known: HashMap::new(),
            };

            self.learn.insert(lscene.rid, learn);
        }

        Ok(())
    }

    #[allow(clippy::single_match_else)]
    async fn websocket_send<T: Serialize + Send>(
        &self,
        socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        topic: &str,
        payload: T,
    ) -> ApiResult<()> {
        match self.map.get(topic) {
            Some(uuid) => {
                log::trace!(
                    "[{}] Topic [{topic}] known as {uuid} on this z2m connection, sending event..",
                    self.name
                );
                let api_req = Other {
                    payload: serde_json::to_value(payload)?,
                    topic: format!("{topic}/set"),
                };
                let json = serde_json::to_string(&api_req)?;
                log::debug!("[{}] Sending {json}", self.name);
                let msg = tungstenite::Message::Text(json);
                Ok(socket.send(msg).await?)
            }
            None => {
                log::trace!(
                    "[{}] Topic [{topic}] unknown on this z2m connection",
                    self.name
                );
                Ok(())
            }
        }
    }

    #[allow(clippy::single_match_else)]
    async fn websocket_write(
        &mut self,
        socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        req: Arc<ClientRequest>,
    ) -> ApiResult<()> {
        self.learn_cleanup();

        let lock = self.state.lock().await;

        match &*req {
            ClientRequest::LightUpdate { device, upd } => {
                let dev = lock.get::<Light>(device)?;
                let topic = dev.metadata.name.clone();
                drop(lock);

                self.websocket_send(socket, &topic, &upd).await
            }
            ClientRequest::GroupUpdate { device, upd } => {
                let group = lock.get::<GroupedLight>(device)?;
                let room = lock.get::<Room>(&group.owner)?;
                let topic = room.metadata.name.clone();
                drop(lock);

                self.websocket_send(socket, &topic, &upd).await
            }
            ClientRequest::SceneStore { room, id, name } => {
                let room = lock.get::<Room>(room)?;
                let topic = room.metadata.name.clone();
                drop(lock);

                let payload = json!({
                    "scene_store": {
                        "ID": id,
                        "name": name,
                    }
                });
                self.websocket_send(socket, &topic, payload).await
            }
            ClientRequest::SceneRecall { scene } => {
                let scn = lock.get::<Scene>(scene)?;
                let room = lock.get::<Room>(&scn.group)?;
                let topic = room.metadata.name.clone();
                let index = lock.aux_get(scene)?.index;
                drop(lock);

                if self.map.contains_key(&topic) {
                    self.learn_scene_recall(scene).await?;
                }

                let payload = json!({"scene_recall": index});
                self.websocket_send(socket, &topic, payload).await
            }
            ClientRequest::SceneRemove { scene } => {
                let scn = lock.get::<Scene>(scene)?;
                let room = lock.get::<Room>(&scn.group)?;
                let topic = room.metadata.name.clone();
                let index = lock.aux_get(scene)?.index;
                drop(lock);

                let payload = json!({
                    "scene_remove": index,
                });

                self.websocket_send(socket, &topic, payload).await
            }
        }
    }

    pub async fn event_loop(
        &mut self,
        chan: &mut Receiver<Arc<ClientRequest>>,
        mut socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> ApiResult<()> {
        loop {
            let res = select! {
                pkt = chan.recv() => {
                    let api_req = pkt?;
                    let res = self.websocket_write(&mut socket, api_req).await;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    res
                },
                pkt = socket.next() => {
                    self.websocket_read(pkt.ok_or(ApiError::UnexpectedZ2mEof)??).await
                },
            };

            if let Err(ref err) = res {
                log::error!("[{}] Event loop failed!: {err:?}", self.name);
                return res;
            }
        }
    }

    pub async fn run_forever(mut self) -> ApiResult<()> {
        let mut chan = self.state.lock().await.z2m_channel();
        loop {
            log::info!("[{}] Connecting to {}", self.name, self.conn);
            match connect_async(&self.conn).await {
                Ok((socket, _)) => {
                    let res = self.event_loop(&mut chan, socket).await;
                    log::error!("[{}] Event loop broke: {res:?}", self.name);
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                }
                Err(err) => {
                    log::error!("[{}] Connect failed: {err:?}", self.name);
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                }
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
