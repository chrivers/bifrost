use serde::Serialize;
use serde_json::Value;

use crate::api::{DeviceRead, DeviceRemove, GroupMemberChange, PermitJoin};
use crate::update::DeviceUpdate;

#[derive(Clone, Debug, Serialize)]
pub struct Z2mPayload {
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Z2mRequest<'a> {
    SceneStore {
        name: &'a str,
        #[serde(rename = "ID")]
        id: u32,
    },

    SceneRecall(u32),

    SceneRemove(u32),

    Write {
        cluster: u16,
        payload: Value,
    },

    Command {
        cluster: u16,
        command: u16,
        payload: Z2mPayload,
    },

    #[serde(untagged)]
    GroupMemberAdd(GroupMemberChange),

    #[serde(untagged)]
    GroupMemberRemove(GroupMemberChange),

    #[serde(untagged)]
    PermitJoin(PermitJoin),

    #[serde(untagged)]
    DeviceRemove(DeviceRemove),

    #[serde(untagged)]
    Update(&'a DeviceUpdate),

    #[serde(untagged)]
    DeviceRead(&'a DeviceRead),

    // same as Z2mRequest::Raw, but allows us to suppress logging for these
    #[serde(untagged)]
    EntertainmentFrame(Value),

    #[serde(untagged)]
    Raw(Value),
}
