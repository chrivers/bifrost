pub mod api;
pub mod update;

use std::collections::{HashMap, HashSet};
use std::hash::RandomState;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::hue;
use crate::hue::v2::{
    ColorTemperature, DeviceProductData, Dimming, GroupedLight, Light, LightColor, Metadata, On,
    RType, Resource, ResourceLink, Room, RoomArchetypes, Scene, SceneMetadata, SceneRecallAction,
    SceneStatus,
};

use crate::error::{ApiError, ApiResult};
use crate::hue::scene_icons;
use crate::resource::AuxData;
use crate::resource::Resources;
use crate::z2m::api::Message;
use crate::z2m::update::DeviceUpdate;

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, Uuid>,
}

impl Client {
    pub async fn new(conn: &str, state: Arc<Mutex<Resources>>) -> ApiResult<Self> {
        let (socket, _) = connect_async(conn).await?;
        let map = HashMap::new();
        Ok(Self { socket, state, map })
    }

    pub async fn add_light(&mut self, dev: &api::Device) -> ApiResult<()> {
        let name = &dev.friendly_name;

        let link_device = RType::Device.deterministic(&dev.ieee_address);
        let link_light = RType::Light.deterministic(&dev.ieee_address);

        let dev = hue::v2::Device {
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
                status: Some(SceneStatus {
                    active: SceneRecallAction::Inactive,
                }),
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
            alert: json!({
                "action_values": [],
            }),
            color: LightColor::dummy(),
            color_temperature: ColorTemperature::dummy(),
            color_temperature_delta: json!({}),
            dimming: Dimming {
                brightness: 100.0,
                min_dim_level: None,
            },
            dimming_delta: json!({}),
            dynamics: json!({}),
            on: On { on: true },
            owner: link_room,
            signaling: json!({
                "signal_values": [],
            }),
        };

        res.add(&link_glight, Resource::GroupedLight(glight))?;
        drop(res);

        Ok(())
    }

    pub async fn handle_update(&self, rid: &Uuid, payload: Value) -> ApiResult<()> {
        let mut res = self.state.lock().await;
        let upd: DeviceUpdate = serde_json::from_value(payload)?;

        match res.get_resource_by_id(rid)?.obj {
            Resource::Light(_) => {
                if let Err(e) = Self::handle_update_light(&mut res, rid, &upd) {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            Resource::GroupedLight(_) => {
                if let Err(e) = Self::handle_update_grouped_light(&mut res, rid, &upd) {
                    log::error!("FAIL: {e:?} in {upd:?}");
                }
            }
            _ => {}
        }
        drop(res);

        Ok(())
    }

    fn handle_update_light(res: &mut Resources, uuid: &Uuid, upd: &DeviceUpdate) -> ApiResult<()> {
        res.update::<Light>(uuid, move |light| {
            if let Some(state) = &upd.state {
                light.on.on = (*state).into();
            }

            if let Some(b) = upd.brightness {
                light.dimming.brightness = b / 254.0 * 100.0;
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
        })
    }

    fn handle_update_grouped_light(
        res: &mut Resources,
        uuid: &Uuid,
        upd: &DeviceUpdate,
    ) -> ApiResult<()> {
        res.update::<GroupedLight>(uuid, move |glight| {
            if let Some(state) = &upd.state {
                glight.on.on = (*state).into();
            }

            if let Some(b) = upd.brightness {
                glight.dimming.brightness = b / 254.0 * 100.0;
            }

            /* glight.color_mode = upd.color_mode; */

            if let Some(ct) = upd.color_temp {
                glight.color_temperature.mirek = ct;
                /* glight.color_temperature.mirek_valid = true; */
            }

            if let Some(col) = upd.color {
                glight.color.xy = col.xy;
                /* glight.color_temperature.mirek_valid = false; */
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

                let Some(val) = self.map.get(&obj.topic) else {
                    log::warn!("Notification on unknown topic {}", &obj.topic);
                    return Ok(());
                };

                self.handle_update(val, obj.payload).await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn websocket_packet(&mut self, pkt: tungstenite::Message) -> ApiResult<()> {
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

    pub async fn event_loop(mut self) -> ApiResult<()> {
        let mut chan = self.state.lock().await.z2m_channel();
        loop {
            let res = select! {
                pkt = chan.recv() => {
                    let api_req = pkt?;
                    let topic = api_req.topic.as_str().strip_suffix("/set").unwrap_or(&api_req.topic);
                    if self.map.contains_key(topic) {
                        log::trace!("Topic [{}] found on this z2m connection, sending event..", &topic);
                        let msg = tungstenite::Message::Text(serde_json::to_string(&api_req)?);
                        Ok(self.socket.send(msg).await?)
                    } else {
                        log::trace!("Topic [{}] unknown on this z2m connection", &topic);
                        Ok(())
                    }
                },
                pkt = self.socket.next() => {
                    self.websocket_packet(pkt.ok_or(ApiError::UnexpectedZ2mEof)??).await
                },
            };

            if let Err(ref err) = res {
                log::error!("Event loop failed!: {err:?}");
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
