use serde::{Deserialize, Serialize};

use crate::hue::api::ResourceLink;
use crate::z2m::update::DeviceUpdate;

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

    #[serde(untagged)]
    Update(&'a DeviceUpdate),
}
