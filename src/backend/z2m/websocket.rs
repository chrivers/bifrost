use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{SinkExt, Stream};
use hue::zigbee::{HueZigbeeUpdate, ZigbeeMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use z2m::api::{DeviceRead, DeviceRemove, GroupMemberChange, PermitJoin};
use z2m::request::Z2mPayload;
use z2m::update::DeviceUpdate;
use z2m::{api::RawMessage, request::Z2mRequest};

use crate::backend::z2m::zclcommand::hue_zclcommand;
use crate::error::ApiResult;

pub struct Z2mWebSocket {
    pub name: String,
    pub socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Z2mWebSocket {
    pub const fn new(name: String, socket: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self { name, socket }
    }

    pub async fn send(&mut self, topic: &str, payload: &Z2mRequest<'_>) -> ApiResult<()> {
        /* let Some(link) = self.map.get(topic) else { */
        /*     log::trace!( */
        /*         "[{}] Topic [{topic}] unknown on this z2m connection", */
        /*         self.name */
        /*     ); */
        /*     return Ok(()); */
        /* }; */

        /* log::trace!( */
        /*     "[{}] Topic [{topic}] known as {link:?} on this z2m connection, sending event..", */
        /*     self.name */
        /* ); */

        let api_req = match &payload {
            Z2mRequest::GroupMemberAdd(value) => RawMessage {
                topic: "bridge/request/group/members/add".into(),
                payload: serde_json::to_value(value)?,
            },
            Z2mRequest::GroupMemberRemove(value) => RawMessage {
                topic: "bridge/request/group/members/remove".into(),
                payload: serde_json::to_value(value)?,
            },
            Z2mRequest::PermitJoin(value) => RawMessage {
                topic: "bridge/request/permit_join".into(),
                payload: serde_json::to_value(value)?,
            },
            Z2mRequest::DeviceRemove(dev) => RawMessage {
                topic: "bridge/request/device/remove".into(),
                payload: serde_json::to_value(dev)?,
            },
            Z2mRequest::DeviceRead(read) => RawMessage {
                topic: format!("{topic}/get"),
                payload: serde_json::to_value(read)?,
            },
            _ => RawMessage {
                topic: format!("{topic}/set"),
                payload: serde_json::to_value(payload)?,
            },
        };

        let json = serde_json::to_string(&api_req)?;
        if matches!(payload, Z2mRequest::EntertainmentFrame(_)) {
            log::trace!("[{}] Entertainment: {json}", self.name);
        } else {
            log::debug!("[{}] Sending {json}", self.name);
        }

        let msg = Message::text(json);
        Ok(self.socket.send(msg).await?)
    }

    pub async fn send_scene_store(&mut self, topic: &str, name: &str, id: u32) -> ApiResult<()> {
        let z2mreq = Z2mRequest::SceneStore { name, id };

        self.send(topic, &z2mreq).await
    }

    pub async fn send_scene_recall(&mut self, topic: &str, index: u32) -> ApiResult<()> {
        let z2mreq = Z2mRequest::SceneRecall(index);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_scene_remove(&mut self, topic: &str, index: u32) -> ApiResult<()> {
        let z2mreq = Z2mRequest::SceneRemove(index);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_update(&mut self, topic: &str, payload: &DeviceUpdate) -> ApiResult<()> {
        let z2mreq = Z2mRequest::Update(payload);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_read(&mut self, topic: &str, payload: &DeviceRead) -> ApiResult<()> {
        let z2mreq = Z2mRequest::DeviceRead(payload);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_group_member_add(
        &mut self,
        topic: &str,
        friendly_name: &str,
    ) -> ApiResult<()> {
        let change = GroupMemberChange {
            device: friendly_name.to_string(),
            group: topic.to_string(),
            endpoint: None,
            skip_disable_reporting: None,
        };
        let z2mreq = Z2mRequest::GroupMemberAdd(change);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_group_member_remove(
        &mut self,
        topic: &str,
        friendly_name: &str,
    ) -> ApiResult<()> {
        let change = GroupMemberChange {
            device: friendly_name.to_string(),
            group: topic.to_string(),
            endpoint: None,
            skip_disable_reporting: None,
        };
        let z2mreq = Z2mRequest::GroupMemberRemove(change);

        self.send(topic, &z2mreq).await
    }

    pub async fn send_zigbee_message(&mut self, topic: &str, msg: &ZigbeeMessage) -> ApiResult<()> {
        let z2mreq = Z2mRequest::Raw(hue_zclcommand(msg));
        self.send(topic, &z2mreq).await
    }

    pub async fn send_entertainment_frame(
        &mut self,
        topic: &str,
        msg: &ZigbeeMessage,
    ) -> ApiResult<()> {
        let z2mreq = Z2mRequest::EntertainmentFrame(hue_zclcommand(msg));
        self.send(topic, &z2mreq).await
    }

    pub async fn send_hue_effects(&mut self, topic: &str, hz: HueZigbeeUpdate) -> ApiResult<()> {
        let data = hz.to_vec()?;
        log::debug!("Sending hue-specific frame: {}", hex::encode(&data));

        let z2mreq = Z2mRequest::Command {
            cluster: 0xFC03,
            command: 0,
            payload: Z2mPayload { data },
        };

        self.send(topic, &z2mreq).await
    }

    pub async fn send_permit_join(&mut self, time: u32, device: Option<String>) -> ApiResult<()> {
        let z2mreq = Z2mRequest::PermitJoin(PermitJoin { time, device });

        self.send("", &z2mreq).await
    }

    pub async fn send_device_remove(&mut self, id: String) -> ApiResult<()> {
        let z2mreq = Z2mRequest::DeviceRemove(DeviceRemove { id });

        self.send("", &z2mreq).await
    }
}

impl Stream for Z2mWebSocket
where
    Self: Unpin,
{
    type Item = Result<Message, tungstenite::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        WebSocketStream::poll_next(Pin::new(&mut self.socket), cx)
    }
}
