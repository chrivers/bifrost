pub mod api;

use std::collections::HashMap;

use futures::StreamExt;
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::{
    error::ApiResult,
    hue::v2::{
        ColorTemperature, Device, DeviceProductData, Dimming, GroupedLight, Light, LightColor,
        Metadata, On, Resource, ResourceType, Room, RoomArchetypes,
    },
    resource::Resources,
    state::AppState,
    z2m::api::{DeviceUpdate, Message, Other},
};

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: AppState,
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
    pub async fn new(conn: &str, state: AppState) -> ApiResult<Self> {
        let (socket, _) = connect_async(conn).await?;

        Ok(Self { socket, state })
    }

    pub async fn event_loop(mut self) -> ApiResult<()> {
        let mut map: HashMap<String, Uuid> = HashMap::new();

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

            let mut res = self.state.res.lock().await;

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
                                println!("{:?}", dev.ieee_address.uuid());
                                println!("{:?}", dev.friendly_name);

                                let name = &dev.friendly_name;
                                let uuid = dev.ieee_address.uuid();

                                let link_device = ResourceType::Device.link_to(uuid);
                                let link_light = link_device.for_type(ResourceType::Light);

                                let dev = Device {
                                    product_data: DeviceProductData::hue_color_spot(),
                                    metadata: Metadata::spot_bulb(name),
                                    identify: json!({}),
                                    services: vec![link_light.clone()],
                                };

                                map.insert(name.to_string(), link_light.rid);

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
                        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, &grp.id.to_le_bytes());
                        let uuid_glight = Uuid::new_v5(&Uuid::NAMESPACE_OID, &grp.id.to_be_bytes());
                        if !res.has(&uuid) {
                            let children = grp
                                .members
                                .iter()
                                .map(|f| ResourceType::Device.link_to(f.ieee_address.uuid()))
                                .collect();

                            let link_room = ResourceType::Room.link_to(uuid);
                            let link_glight = ResourceType::GroupedLight.link_to(uuid_glight);

                            let room = Room {
                                id_v1: Some(format!("/room/{}", grp.id)),
                                children,
                                metadata: Metadata::room(
                                    RoomArchetypes::Computer,
                                    &grp.friendly_name,
                                ),
                                services: vec![link_glight.clone()],
                            };

                            map.insert(grp.friendly_name.to_string(), link_glight.rid);
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

                    let Some(val) = map.get(&obj.topic) else {
                        log::warn!("Notification on unknown topic {}", &obj.topic);
                        continue;
                    };

                    match res.get_resource_by_id(val)?.obj {
                        Resource::Light(light) => {
                            if let Err(e) = handle_light(val, &mut res, obj) {
                                log::error!("FAIL: {e:?}");
                            }
                        }
                        Resource::GroupedLight(light) => {
                            if let Err(e) = handle_grouped_light(val, &mut res, obj) {
                                log::error!("FAIL: {e:?}");
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
