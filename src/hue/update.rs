use serde::{Deserialize, Serialize};
use serde_json::{from_value, Value};
use uuid::Uuid;

use crate::{
    error::ApiResult,
    hue::v2::{On, RType},
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
            /* Self::BehaviorScript(_) => RType::BehaviorScript, */
            /* Self::BehaviorInstance(_) => RType::BehaviorInstance, */
            /* Self::Bridge(_) => RType::Bridge, */
            /* Self::BridgeHome(_) => RType::BridgeHome, */
            /* Self::Device(_) => RType::Device, */
            /* Self::Entertainment(_) => RType::Entertainment, */
            /* Self::GeofenceClient(_) => RType::GeofenceClient, */
            /* Self::Geolocation(_) => RType::Geolocation, */
            Self::GroupedLight(_) => RType::GroupedLight,
            /* Self::Homekit(_) => RType::Homekit, */
            Self::Light(_) => RType::Light,
            /* Self::Matter(_) => RType::Matter, */
            /* Self::PublicImage(_) => RType::PublicImage, */
            /* Self::Room(_) => RType::Room, */
            Self::Scene(_) => RType::Scene,
            /* Self::SmartScene(_) => RType::SmartScene, */
            /* Self::ZigbeeConnectivity(_) => RType::ZigbeeConnectivity, */
            /* Self::ZigbeeDeviceDiscovery(_) => RType::ZigbeeDeviceDiscovery, */
            /* Self::Zone(_) => RType::Zone, */
        }
    }

    pub fn from_value(rtype: RType, obj: Value) -> ApiResult<Self> {
        let res = match rtype {
            /* RType::BehaviorScript => Self::BehaviorScript(from_value(obj)?), */
            /* RType::BehaviorInstance => Self::BehaviorInstance(from_value(obj)?), */
            /* RType::Bridge => Self::Bridge(from_value(obj)?), */
            /* RType::BridgeHome => Self::BridgeHome(from_value(obj)?), */
            /* RType::Device => Self::Device(from_value(obj)?), */
            /* RType::Entertainment => Self::Entertainment(from_value(obj)?), */
            /* RType::GeofenceClient => Self::GeofenceClient(from_value(obj)?), */
            /* RType::Geolocation => Self::Geolocation(from_value(obj)?), */
            RType::GroupedLight => Self::GroupedLight(from_value(obj)?),
            /* RType::Homekit => Self::Homekit(from_value(obj)?), */
            RType::Light => Self::Light(from_value(obj)?),
            /* RType::Matter => Self::Matter(from_value(obj)?), */
            /* RType::PublicImage => Self::PublicImage(from_value(obj)?), */
            /* RType::Room => Self::Room(from_value(obj)?), */
            RType::Scene => Self::Scene(from_value(obj)?),
            /* RType::SmartScene => Self::SmartScene(from_value(obj)?), */
            /* RType::ZigbeeConnectivity => Self::ZigbeeConnectivity(from_value(obj)?), */
            /* RType::ZigbeeDeviceDiscovery => Self::ZigbeeDeviceDiscovery(from_value(obj)?), */
            /* RType::Zone => Self::Zone(from_value(obj)?), */
            _ => Err(<serde_json::Error as serde::de::Error>::custom("foo"))?,
        };
        Ok(res)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRecord {
    id: Uuid,
    id_v1: String,
    #[serde(flatten)]
    pub obj: Update,
}

impl UpdateRecord {
    #[must_use]
    pub fn from_ref((id, obj): (&Uuid, &Update)) -> Self {
        Self {
            id: *id,
            id_v1: format!("/legacy/{}", id.as_simple()),
            obj: obj.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightUpdate {
    pub on: Option<On>,
    pub dimming: Option<DimmingUpdate>,
    pub color: Option<ColorUpdate>,
    pub color_temp: Option<f64>,
    pub color_temperature: Option<ColorTemperatureUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupedLightUpdate {
    pub on: Option<On>,
    pub dimming: Option<DimmingUpdate>,
    pub color: Option<ColorUpdate>,
    pub color_temp: Option<f64>,
    pub color_temperature: Option<ColorTemperatureUpdate>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneUpdate {
    pub recall: Option<SceneRecall>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneRecall {
    pub action: Option<SceneRecallAction>,
    pub duration: Option<u32>,
    pub dimming: Option<DimmingUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecallAction {
    Active,
    DynamicPalette,
    Static,
}
