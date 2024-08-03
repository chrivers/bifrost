pub mod api;

use futures::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::{
    error::ApiResult,
    hue::v2::{Metadata, Resource, ResourceType, Room, RoomArchetypes},
    state::AppState,
    z2m::api::Message,
};

pub struct Client {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
    state: AppState,
}

impl Client {
    pub async fn new(conn: &str, state: AppState) -> ApiResult<Self> {
        let (socket, _) = connect_async(conn).await?;

        Ok(Self { socket, state })
    }

    pub async fn event_loop(mut self) -> ApiResult<()> {
        loop {
            let Some(pkt) = self.socket.next().await else {
                break;
            };

            let tungstenite::Message::Text(txt) = pkt? else {
                break;
            };

            let data = serde_json::from_str(&txt);

            let Ok(msg) = data else {
                log::error!("INVALID: {:#?}", data);
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
                                if (id == "TRADFRI bulb GU10 CWS 345lm") || (id == "LCG002") =>
                            {
                                println!("{:?}", dev.ieee_address.uuid());
                                println!("{:?}", dev.friendly_name);

                                let uuid = dev.ieee_address.uuid();
                                if !res.has(&uuid) {
                                    res.add_light(uuid, &dev.friendly_name)?;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Message::BridgeGroups(ref obj) => {
                    /* println!("{obj:#?}"); */
                    for grp in obj {
                        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, &grp.id.to_le_bytes());
                        if !res.has(&uuid) {
                            let children = grp
                                .members
                                .iter()
                                .map(|f| ResourceType::Device.link_to(f.ieee_address.uuid()))
                                .collect();

                            let link_room = ResourceType::Room.link_to(uuid);

                            let room = Room {
                                id_v1: Some(format!("/room/{}", grp.id)),
                                children,
                                metadata: Metadata::room(
                                    RoomArchetypes::Computer,
                                    &grp.friendly_name,
                                ),
                                services: vec![],
                            };

                            res.add(&link_room, Resource::Room(room))?;
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
                    if obj.topic.contains('/') {
                        println!("{:#?}", obj.topic);
                    }
                }
                _ => {}
            }

            drop(res);
        }
        Ok(())
    }
}
