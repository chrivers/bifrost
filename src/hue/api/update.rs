use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hue::api::{
    DeviceUpdate, GroupedLightUpdate, LightUpdate, RType, RoomUpdate, SceneUpdate,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Update {
    /* BehaviorScript(BehaviorScriptUpdate), */
    /* BehaviorInstance(BehaviorInstanceUpdate), */
    /* Bridge(BridgeUpdate), */
    /* BridgeHome(BridgeHomeUpdate), */
    Device(DeviceUpdate),
    /* Entertainment(EntertainmentUpdate), */
    /* GeofenceClient(GeofenceClientUpdate), */
    /* Geolocation(GeolocationUpdate), */
    GroupedLight(GroupedLightUpdate),
    /* Homekit(HomekitUpdate), */
    Light(LightUpdate),
    /* Matter(MatterUpdate), */
    /* PublicImage(PublicImageUpdate), */
    Room(RoomUpdate),
    Scene(SceneUpdate),
    /* SmartScene(SmartSceneUpdate), */
    /* ZigbeeConnectivity(ZigbeeConnectivityUpdate), */
    /* ZigbeeDeviceDiscovery(ZigbeeDeviceDiscoveryUpdate), */
    /* Zone(ZoneUpdate), */
}

impl Update {
    #[must_use]
    pub const fn rtype(&self) -> RType {
        match self {
            Self::GroupedLight(_) => RType::GroupedLight,
            Self::Device(_) => RType::Device,
            Self::Light(_) => RType::Light,
            Self::Room(_) => RType::Room,
            Self::Scene(_) => RType::Scene,
        }
    }

    #[must_use]
    pub fn id_v1_scope(&self, id: u32, uuid: &Uuid) -> Option<String> {
        match self {
            Self::Room(_) | Self::GroupedLight(_) => Some(format!("/groups/{id}")),
            Self::Device(_) => Some(format!("/device/{id}")),
            Self::Light(_) => Some(format!("/lights/{id}")),
            Self::Scene(_) => Some(format!("/scenes/{uuid}")),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRecord {
    id: Uuid,
    id_v1: Option<String>,
    #[serde(flatten)]
    pub upd: Update,
}

impl UpdateRecord {
    #[must_use]
    pub fn new(uuid: &Uuid, id_v1: Option<u32>, upd: Update) -> Self {
        Self {
            id: *uuid,
            id_v1: id_v1.and_then(|id| upd.id_v1_scope(id, uuid)),
            upd,
        }
    }
}
