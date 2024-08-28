pub mod api;
pub mod request;
pub mod update;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::config::{AppConfig, Z2mServer};
use crate::hue;
use crate::hue::api::{
    Button, ButtonData, ButtonMetadata, ButtonReport, ColorTemperature, ColorTemperatureUpdate,
    ColorUpdate, Device, DeviceArchetype, DeviceProductData, Dimming, DimmingUpdate, GroupedLight,
    Identify, Light, LightColor, LightUpdate, Metadata, RType, Resource, ResourceLink, Room,
    RoomArchetype, RoomMetadata, Scene, SceneAction, SceneActionElement, SceneMetadata,
    SceneRecall, SceneStatus, ZigbeeConnectivity, ZigbeeConnectivityStatus,
};

use crate::error::{ApiError, ApiResult};
use crate::hue::scene_icons;
use crate::model::state::AuxData;
use crate::resource::Resources;
use crate::z2m::api::{ExposeLight, Message, Other, RawMessage};
use crate::z2m::request::{ClientRequest, Z2mRequest};
use crate::z2m::update::DeviceUpdate;

#[derive(Debug)]
struct LearnScene {
    pub expire: DateTime<Utc>,
    pub missing: HashSet<Uuid>,
    pub known: HashMap<Uuid, SceneAction>,
}

pub struct Client {
    name: String,
    server: Z2mServer,
    config: Arc<AppConfig>,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, Uuid>,
    rmap: HashMap<Uuid, String>,
    learn: HashMap<Uuid, LearnScene>,
    ignore: HashSet<String>,
}

impl Client {
    pub fn new(
        name: String,
        server: Z2mServer,
        config: Arc<AppConfig>,
        state: Arc<Mutex<Resources>>,
    ) -> ApiResult<Self> {
        let map = HashMap::new();
        let rmap = HashMap::new();
        let learn = HashMap::new();
        let ignore = HashSet::new();
        Ok(Self {
            name,
            server,
            config,
            state,
            map,
            rmap,
            learn,
            ignore,
        })
    }

    pub async fn add_light(&mut self, dev: &api::Device, expose: &ExposeLight) -> ApiResult<()> {
        let name = &dev.friendly_name;

        let link_device = RType::Device.deterministic(&dev.ieee_address);
        let link_light = RType::Light.deterministic(&dev.ieee_address);

        let product_data = DeviceProductData::guess_from_device(dev);
        let metadata = Metadata::new(DeviceArchetype::SpotBulb, name);

        let dev = hue::api::Device {
            product_data,
            metadata: metadata.clone(),
            services: vec![link_light],
            identify: None,
            usertest: None,
        };

        self.map.insert(name.to_string(), link_light.rid);
        self.rmap.insert(link_light.rid, name.to_string());

        let mut res = self.state.lock().await;
        let mut light = Light::new(link_device, metadata);

        light.dimming = expose
            .feature("brightness")
            .and_then(Dimming::extract_from_expose);
        log::trace!("Detected dimming: {:?}", &light.dimming);

        light.color_temperature = expose
            .feature("color_temp")
            .and_then(ColorTemperature::extract_from_expose);
        log::trace!("Detected color temperature: {:?}", &light.color_temperature);

        light.color = expose
            .feature("color_xy")
            .and_then(LightColor::extract_from_expose);
        log::trace!("Detected color: {:?}", &light.color);

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
            identify: None,
            usertest: None,
        };

        self.map.insert(name.to_string(), link_button.rid);
        self.rmap.insert(link_button.rid, name.to_string());

        let mut res = self.state.lock().await;
        let button = Button {
            owner: link_device,
            metadata: ButtonMetadata { control_id: 0 },
            button: ButtonData {
                last_event: None,
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
            extended_pan_id: None,
        };

        res.add(&link_device, Resource::Device(dev))?;
        res.add(&link_button, Resource::Button(button))?;
        res.add(&link_zbc, Resource::ZigbeeConnectivity(zbc))?;
        drop(res);

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub async fn add_group(&mut self, grp: &crate::z2m::api::Group) -> ApiResult<()> {
        let room_name;

        if let Some(ref prefix) = self.server.group_prefix {
            if let Some(name) = grp.friendly_name.strip_prefix(prefix) {
                room_name = name;
            } else {
                log::debug!(
                    "[{}] Ignoring room outside our prefix: {}",
                    self.name,
                    grp.friendly_name
                );
                return Ok(());
            }
        } else {
            room_name = &grp.friendly_name;
        }

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
                recall: SceneRecall {
                    action: None,
                    dimming: None,
                    duration: None,
                },
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
            log::debug!(
                "[{}] {link_room:?} ({}) is new, adding..",
                self.name,
                room_name
            );
        }

        let mut metadata = RoomMetadata::new(RoomArchetype::Home, room_name);
        if let Some(room_conf) = self.config.rooms.get(&topic) {
            if let Some(name) = &room_conf.name {
                metadata.name = name.to_string();
            }
            if let Some(icon) = &room_conf.icon {
                metadata.archetype = *icon;
            }
        };

        let room = Room {
            children,
            metadata,
            services: vec![link_glight],
        };

        self.map.insert(topic.clone(), link_glight.rid);
        self.rmap.insert(link_glight.rid, topic.clone());
        self.rmap.insert(link_room.rid, topic.clone());

        res.add(&link_room, Resource::Room(room))?;

        let glight = GroupedLight::new(link_room);

        res.add(&link_glight, Resource::GroupedLight(glight))?;
        drop(res);

        Ok(())
    }

    pub async fn handle_update(&mut self, rid: &Uuid, payload: &Value) -> ApiResult<()> {
        let upd = DeviceUpdate::deserialize(payload)?;

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

    async fn handle_update_light(&mut self, uuid: &Uuid, devupd: &DeviceUpdate) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        res.update::<Light>(uuid, move |light| {
            let upd = LightUpdate::new()
                .with_on(devupd.state.map(Into::into))
                .with_brightness(devupd.brightness.map(|b| b / 254.0 * 100.0))
                .with_color_temperature(devupd.color_temp)
                .with_color_xy(devupd.color.map(|col| col.xy));

            *light += upd;
        })?;

        for learn in self.learn.values_mut() {
            if learn.missing.remove(uuid) {
                let upd = devupd;
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
                        dimming: light.as_dimming_opt(),
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
                res.update(uuid, |scene: &mut Scene| {
                    scene.actions = actions;
                })?;
            }
        }
        drop(res);

        Ok(())
    }

    async fn handle_update_grouped_light(&self, uuid: &Uuid, upd: &DeviceUpdate) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        res.update::<GroupedLight>(uuid, |glight| {
            if let Some(state) = &upd.state {
                glight.on = Some((*state).into());
            }

            if let Some(b) = upd.brightness {
                glight.dimming = Some(DimmingUpdate {
                    brightness: b / 254.0 * 100.0,
                });
            }
        })
    }

    async fn handle_bridge_message(&mut self, msg: Message) -> ApiResult<()> {
        #[allow(unused_variables)]
        match msg {
            Message::BridgeInfo(ref obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeLogging(ref obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeExtensions(ref obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeEvent(ref obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeDefinitions(ref obj) => { /* println!("{obj:#?}"); */ }
            Message::BridgeState(ref obj) => { /* println!("{obj:#?}"); */ }

            Message::BridgeDevices(ref obj) => {
                for dev in obj {
                    if let Some(exp) = dev.expose_light() {
                        log::info!(
                            "[{}] Adding light {:?}: [{}] ({})",
                            self.name,
                            dev.ieee_address,
                            dev.friendly_name,
                            dev.model_id.as_deref().unwrap_or("<unknown model>")
                        );
                        self.add_light(dev, exp).await?;
                    } else {
                        log::debug!(
                            "[{}] Ignoring unsupported device {}",
                            self.name,
                            dev.friendly_name
                        );
                        self.ignore.insert(dev.friendly_name.to_string());
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
        }
        Ok(())
    }

    async fn handle_device_message(&mut self, msg: RawMessage) -> ApiResult<()> {
        if msg.topic.contains('/') {
            return Ok(());
        }

        let Some(ref val) = self.map.get(&msg.topic).copied() else {
            if !self.ignore.contains(&msg.topic) {
                log::warn!(
                    "[{}] Notification on unknown topic {}",
                    self.name,
                    &msg.topic
                );
            }
            return Ok(());
        };

        let res = self.handle_update(val, &msg.payload).await;
        if let Err(ref err) = res {
            log::error!(
                "Cannot parse update: {err}\n{}",
                serde_json::to_string_pretty(&msg.payload)?
            );
        }

        /* return Ok here, since we do not want to break the event loop */
        Ok(())
    }

    async fn websocket_read(&mut self, pkt: tungstenite::Message) -> ApiResult<()> {
        let tungstenite::Message::Text(txt) = pkt else {
            log::error!("[{}] Received non-text message on websocket :(", self.name);
            return Err(ApiError::UnexpectedZ2mReply(pkt));
        };

        let raw_msg: Result<RawMessage, _> = serde_json::from_str(&txt);

        match raw_msg {
            Ok(msg) => {
                if msg.topic.starts_with("bridge/") {
                    match serde_json::from_str(&txt) {
                        Ok(bridge_msg) => self.handle_bridge_message(bridge_msg).await,
                        Err(err) => {
                            match msg.topic.as_str() {
                                topic @ ("bridge/devices" | "bridge/groups") => {
                                    log::error!(
                                        "[{}] Failed to parse critical z2m bridge message on [{}]:",
                                        self.name,
                                        topic,
                                    );
                                    log::error!(
                                        "[{}] {}",
                                        self.name,
                                        serde_json::to_string(&msg.payload)?
                                    );
                                    Err(err)?
                                }
                                topic => {
                                    log::error!("[{}] Failed to parse (non-critical) z2m bridge message on [{}]:", self.name, topic);
                                    log::error!("{}", serde_json::to_string(&msg.payload)?);

                                    /* Suppress this non-critical error, to avoid breaking the event loop */
                                    Ok(())
                                }
                            }
                        }
                    }
                } else {
                    self.handle_device_message(msg).await
                }
            }
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

    async fn websocket_send<'a>(
        &self,
        socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        topic: &str,
        payload: Z2mRequest<'a>,
    ) -> ApiResult<()> {
        let Some(uuid) = self.map.get(topic) else {
            log::trace!(
                "[{}] Topic [{topic}] unknown on this z2m connection",
                self.name
            );
            return Ok(());
        };

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

    async fn websocket_write(
        &mut self,
        socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        req: Arc<ClientRequest>,
    ) -> ApiResult<()> {
        self.learn_cleanup();

        let lock = self.state.lock().await;

        match &*req {
            ClientRequest::LightUpdate { device, upd } => {
                drop(lock);
                if let Some(topic) = self.rmap.get(&device.rid) {
                    let z2mreq = Z2mRequest::Update(upd);
                    self.websocket_send(socket, topic, z2mreq).await?;
                };
            }

            ClientRequest::GroupUpdate { device, upd } => {
                let room = lock.get::<GroupedLight>(device)?.owner.rid;
                drop(lock);

                if let Some(topic) = self.rmap.get(&room) {
                    let z2mreq = Z2mRequest::Update(upd);
                    self.websocket_send(socket, topic, z2mreq).await?;
                }
            }

            ClientRequest::SceneStore { room, id, name } => {
                drop(lock);
                if let Some(topic) = self.rmap.get(&room.rid) {
                    let z2mreq = Z2mRequest::SceneStore { name, id: *id };
                    self.websocket_send(socket, topic, z2mreq).await?;
                }
            }

            ClientRequest::SceneRecall { scene } => {
                let room = lock.get::<Scene>(scene)?.group.rid;
                let index = lock
                    .aux_get(scene)?
                    .index
                    .ok_or(ApiError::NotFound(scene.rid))?;
                drop(lock);
                if let Some(topic) = self.rmap.get(&room).cloned() {
                    self.learn_scene_recall(scene).await?;
                    let z2mreq = Z2mRequest::SceneRecall(index);
                    self.websocket_send(socket, &topic, z2mreq).await?;
                }
            }

            ClientRequest::SceneRemove { scene } => {
                let room = lock.get::<Scene>(scene)?.group.rid;
                let index = lock
                    .aux_get(scene)?
                    .index
                    .ok_or(ApiError::NotFound(scene.rid))?;
                drop(lock);

                if let Some(topic) = self.rmap.get(&room).cloned() {
                    let z2mreq = Z2mRequest::SceneRemove(index);
                    self.websocket_send(socket, &topic, z2mreq).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn event_loop(
        &mut self,
        chan: &mut Receiver<Arc<ClientRequest>>,
        mut socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> ApiResult<()> {
        loop {
            select! {
                pkt = chan.recv() => {
                    let api_req = pkt?;
                    self.websocket_write(&mut socket, api_req).await?;
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                },
                pkt = socket.next() => {
                    self.websocket_read(pkt.ok_or(ApiError::UnexpectedZ2mEof)??).await?;
                },
            };
        }
    }

    pub async fn run_forever(mut self) -> ApiResult<()> {
        let mut chan = self.state.lock().await.z2m_channel();
        loop {
            log::info!("[{}] Connecting to {}", self.name, self.server.url);
            match connect_async(&self.server.url).await {
                Ok((socket, _)) => {
                    let res = self.event_loop(&mut chan, socket).await;
                    if let Err(err) = res {
                        log::error!("[{}] Event loop broke: {err}", self.name);
                    }
                }
                Err(err) => {
                    log::error!("[{}] Connect failed: {err:?}", self.name);
                }
            }
            sleep(std::time::Duration::from_millis(2000)).await;
        }
    }
}

#[allow(clippy::match_same_arms)]
fn guess_scene_icon(name: &str) -> Option<ResourceLink> {
    let icon = match name {
        /* Built-in names */
        "Bright" => scene_icons::BRIGHT,
        "Relax" => scene_icons::RELAX,
        "Night Light" => scene_icons::NIGHT_LIGHT,
        "Rest" => scene_icons::REST,
        "Concentrate" => scene_icons::CONCENTRATE,
        "Dimmed" => scene_icons::DIMMED,
        "Energize" => scene_icons::ENERGIZE,
        "Read" => scene_icons::READ,
        "Cool Bright" => scene_icons::COOL_BRIGHT,

        /* Aliasas */
        "Night" => scene_icons::NIGHT_LIGHT,
        "Cool" => scene_icons::COOL_BRIGHT,
        "Dim" => scene_icons::DIMMED,

        _ => return None,
    };

    Some(ResourceLink {
        rid: icon,
        rtype: RType::PublicImage,
    })
}
