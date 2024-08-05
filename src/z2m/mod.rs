pub mod api;

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use serde_json::json;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::hue::v2::{
    ColorTemperature, Device, DeviceProductData, Dimming, GroupedLight, Light, LightColor,
    Metadata, On, Resource, ResourceLink, ResourceType, Room, RoomArchetypes, Scene, SceneMetadata,
    SceneStatus,
};

use crate::resource::AuxData;
use crate::{
    error::ApiResult,
    hue::scene_icons,
    resource::Resources,
    z2m::api::{DeviceUpdate, Message, Other},
};

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: Arc<Mutex<Resources>>,
    map: HashMap<String, Uuid>,
}

fn handle_light(uuid: &Uuid, res: &mut Resources, obj: &Other) -> ApiResult<()> {
    let upd: DeviceUpdate = serde_json::from_value(obj.payload.clone())?;

    res.update_light(uuid, move |light| {
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
            light.color.xy.x = col.x;
            light.color.xy.y = col.y;
            /* light.color_temperature.mirek_valid = false; */
        }
    })?;

    Ok(())
}

fn handle_grouped_light(uuid: &Uuid, res: &mut Resources, obj: &Other) -> ApiResult<()> {
    let upd: DeviceUpdate = serde_json::from_value(obj.payload.clone())?;

    res.update_grouped_light(uuid, move |glight| {
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
            glight.color.xy.x = col.x;
            glight.color.xy.y = col.y;
            /* glight.color_temperature.mirek_valid = false; */
        }
    })?;

    Ok(())
}

impl Client {
    pub async fn new(conn: &str, state: Arc<Mutex<Resources>>) -> ApiResult<Self> {
        let (socket, _) = connect_async(conn).await?;
        let map = HashMap::new();
        Ok(Self { socket, state, map })
    }

    pub async fn event_loop(mut self) -> ApiResult<()> {
        loop {
            let Some(pkt) = self.socket.next().await else {
                log::error!("Websocket broke :(");
                break;
            };

            let tungstenite::Message::Text(txt) = pkt? else {
                break;
            };

            let data = serde_json::from_str(&txt);

            let Ok(msg) = data else {
                log::error!(
                    "INVALID: {:#?} [{}..]",
                    data,
                    &txt.chars().take(128).collect::<String>()
                );
                continue;
            };

            let mut res = self.state.lock().await;

            #[allow(unused_variables)]
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
                        match dev.model_id {
                            Some(ref id)
                                if (id == "TRADFRI bulb GU10 CWS 345lm")
                                    || (id == "LCG002")
                                    || (id == "TRADFRI bulb E27 CWS 806lm") =>
                            {
                                println!("{:?}", dev.ieee_address);
                                println!("{:?}", dev.friendly_name);

                                let name = &dev.friendly_name;

                                let link_device =
                                    ResourceType::Device.deterministic(&dev.ieee_address);
                                let link_light =
                                    ResourceType::Light.deterministic(&dev.ieee_address);

                                let dev = Device {
                                    product_data: DeviceProductData::hue_color_spot(),
                                    metadata: Metadata::spot_bulb(name),
                                    identify: json!({}),
                                    services: vec![link_light.clone()],
                                };

                                self.map.insert(name.to_string(), link_light.rid);
                                res.aux
                                    .insert(link_light.rid, AuxData::new().with_topic(name));

                                let mut light = Light::new(res.next_idv1(), link_device.clone());
                                light.metadata.name = name.to_string();

                                res.add(&link_device, Resource::Device(dev))?;
                                res.add(&link_light, Resource::Light(light))?;
                            }
                            _ => {}
                        }
                    }
                }

                Message::BridgeGroups(ref obj) => {
                    /* println!("{obj:#?}"); */
                    for grp in obj {
                        let link_room = ResourceType::Room.deterministic(grp.id);
                        let link_glight = ResourceType::GroupedLight.deterministic(grp.id);

                        let children = grp
                            .members
                            .iter()
                            .map(|f| ResourceType::Device.deterministic(&f.ieee_address))
                            .collect();

                        let mut services = vec![link_glight.clone()];

                        for scn in &grp.scenes {
                            let scene = Scene {
                                actions: vec![],
                                auto_dynamic: false,
                                group: link_room.clone(),
                                id_v1: Some("/scene/blob".to_string()),
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
                                recall: None,
                                speed: 0.5,
                                status: Some(SceneStatus {
                                    active: "inactive".to_string(),
                                }),
                            };

                            let link_scene =
                                ResourceType::Scene.deterministic((grp.id, scn.id));

                            res.aux.insert(
                                link_scene.rid,
                                AuxData::new()
                                    .with_topic(&grp.friendly_name)
                                    .with_index(scn.id),
                            );

                            services.push(link_scene.clone());
                            res.add(&link_scene, Resource::Scene(scene))?;
                        }

                        let room = Room {
                            id_v1: Some(format!("/room/{}", grp.id)),
                            children,
                            metadata: Metadata::room(
                                RoomArchetypes::Computer,
                                &grp.friendly_name,
                            ),
                            services,
                        };

                        self.map
                            .insert(grp.friendly_name.to_string(), link_glight.rid);
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
                            id_v1: Some(format!("/groups/{}", grp.id)),
                            on: On { on: true },
                            owner: link_room,
                            signaling: json!({
                                "signal_values": [],
                            }),
                        };

                        res.add(&link_glight, Resource::GroupedLight(glight))?;
                    }
                }

                /* Message::BridgeDefinitions(ref obj) => { */
                /*     /\* println!("{obj:#?}"); *\/ */
                /* } */

                /* Message::BridgeState(ref obj) => { */
                /*     /\* println!("{obj:#?}"); *\/ */
                /* } */
                Message::Other(ref obj) => {
                    println!("{:#?}", obj.topic);
                    if obj.topic.contains('/') {
                        continue;
                    }

                    let Some(val) = self.map.get(&obj.topic) else {
                        log::warn!("Notification on unknown topic {}", &obj.topic);
                        continue;
                    };

                    match res.get_resource_by_id(val)?.obj {
                        Resource::Light(light) => {
                            if let Err(e) = handle_light(val, &mut res, obj) {
                                log::error!("FAIL: {e:?} in {obj:?}");
                            }
                        }
                        Resource::GroupedLight(light) => {
                            if let Err(e) = handle_grouped_light(val, &mut res, obj) {
                                log::error!("FAIL: {e:?} in {obj:?}");
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            drop(res);
        }
        Ok(())
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
        rtype: ResourceType::PublicImage,
    })
}
