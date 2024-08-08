use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    hue::api::{On, RType, SceneActionElement, SceneStatus, SceneStatusUpdate},
    types::XY,
};

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LightUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<On>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<DimmingUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature: Option<ColorTemperatureUpdate>,
}

impl LightUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_brightness(self, brightness: f64) -> Self {
        Self {
            dimming: Some(DimmingUpdate { brightness }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_on(self, on: bool) -> Self {
        Self {
            on: Some(On { on }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_color_temperature(self, mirek: u32) -> Self {
        Self {
            color_temperature: Some(ColorTemperatureUpdate { mirek }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_color_xy(self, xy: XY) -> Self {
        Self {
            color: Some(ColorUpdate { xy }),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GroupedLightUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<On>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<DimmingUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature: Option<ColorTemperatureUpdate>,
}

impl GroupedLightUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_brightness(self, brightness: f64) -> Self {
        Self {
            dimming: Some(DimmingUpdate { brightness }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_on(self, on: bool) -> Self {
        Self {
            on: Some(On { on }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_color_temperature(self, mirek: u32) -> Self {
        Self {
            color_temperature: Some(ColorTemperatureUpdate { mirek }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_color_xy(self, xy: XY) -> Self {
        Self {
            color: Some(ColorUpdate { xy }),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DimmingUpdate {
    pub brightness: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorUpdate {
    pub xy: XY,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperatureUpdate {
    pub mirek: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SceneUpdate {
    pub actions: Option<Vec<SceneActionElement>>,
    pub recall: Option<SceneRecall>,
}

impl SceneUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_actions(self, actions: Option<Vec<SceneActionElement>>) -> Self {
        Self { actions, ..self }
    }

    #[must_use]
    pub fn with_recall_action(self, action: Option<SceneStatus>) -> Self {
        Self {
            recall: Some(SceneRecall {
                action: match action {
                    Some(SceneStatus::DynamicPalette) => Some(SceneStatusUpdate::DynamicPalette),
                    Some(SceneStatus::Static) => Some(SceneStatusUpdate::Active),
                    Some(SceneStatus::Inactive) | None => None,
                },
                duration: None,
                dimming: None,
            }),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneRecall {
    pub action: Option<SceneStatusUpdate>,
    pub duration: Option<u32>,
    pub dimming: Option<DimmingUpdate>,
}
