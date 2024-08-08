use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hue::api::{GroupedLightUpdate, LightUpdate, RType, SceneUpdate};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Update {
    /* BehaviorScript(BehaviorScriptUpdate), */
    /* BehaviorInstance(BehaviorInstanceUpdate), */
    /* Bridge(BridgeUpdate), */
    /* BridgeHome(BridgeHomeUpdate), */
    /* Device(DeviceUpdate), */
    /* Entertainment(EntertainmentUpdate), */
    /* GeofenceClient(GeofenceClientUpdate), */
    /* Geolocation(GeolocationUpdate), */
    GroupedLight(GroupedLightUpdate),
    /* Homekit(HomekitUpdate), */
    Light(LightUpdate),
    /* Matter(MatterUpdate), */
    /* PublicImage(PublicImageUpdate), */
    /* Room(RoomUpdate), */
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
            Self::Light(_) => RType::Light,
            Self::Scene(_) => RType::Scene,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRecord {
    id: Uuid,
    id_v1: String,
    #[serde(flatten)]
    pub upd: Update,
}

impl UpdateRecord {
    #[must_use]
    pub fn new(id: &Uuid, upd: Update) -> Self {
        Self {
            id: *id,
            id_v1: format!("/legacy/{}", id.as_simple()),
            upd,
        }
    }
}
