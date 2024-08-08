mod device;
mod grouped_light;
mod light;
mod resource;
mod room;
mod scene;
mod stubs;

pub use device::{Device, DeviceProductData};
pub use grouped_light::GroupedLight;
pub use light::Light;
pub use resource::{RType, ResourceLink, ResourceRecord};
pub use room::{Room, RoomArchetypes};
pub use scene::{
    Scene, SceneAction, SceneActionElement, SceneMetadata, SceneStatus, SceneStatusUpdate,
};
pub use stubs::{
    BehaviorInstance, BehaviorScript, DollarRef, Entertainment, EntertainmentSegment,
    EntertainmentSegments, GeofenceClient, Geolocation, Homekit, Matter, PublicImage, SmartScene,
    ZigbeeConnectivity, ZigbeeConnectivityStatus, ZigbeeDeviceDiscovery, Zone,
};

use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use crate::error::{ApiError, ApiResult};
use crate::{hue::best_guess_timezone, types::XY};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    BehaviorScript(BehaviorScript),
    BehaviorInstance(BehaviorInstance),
    Bridge(Bridge),
    BridgeHome(BridgeHome),
    Device(Device),
    Entertainment(Entertainment),
    GeofenceClient(GeofenceClient),
    Geolocation(Geolocation),
    GroupedLight(GroupedLight),
    Homekit(Homekit),
    Light(Light),
    Matter(Matter),
    PublicImage(PublicImage),
    Room(Room),
    Scene(Scene),
    SmartScene(SmartScene),
    ZigbeeConnectivity(ZigbeeConnectivity),
    ZigbeeDeviceDiscovery(ZigbeeDeviceDiscovery),
    Zone(Zone),
}

impl Resource {
    #[must_use]
    pub const fn rtype(&self) -> RType {
        match self {
            Self::BehaviorScript(_) => RType::BehaviorScript,
            Self::BehaviorInstance(_) => RType::BehaviorInstance,
            Self::Bridge(_) => RType::Bridge,
            Self::BridgeHome(_) => RType::BridgeHome,
            Self::Device(_) => RType::Device,
            Self::Entertainment(_) => RType::Entertainment,
            Self::GeofenceClient(_) => RType::GeofenceClient,
            Self::Geolocation(_) => RType::Geolocation,
            Self::GroupedLight(_) => RType::GroupedLight,
            Self::Homekit(_) => RType::Homekit,
            Self::Light(_) => RType::Light,
            Self::Matter(_) => RType::Matter,
            Self::PublicImage(_) => RType::PublicImage,
            Self::Room(_) => RType::Room,
            Self::Scene(_) => RType::Scene,
            Self::SmartScene(_) => RType::SmartScene,
            Self::ZigbeeConnectivity(_) => RType::ZigbeeConnectivity,
            Self::ZigbeeDeviceDiscovery(_) => RType::ZigbeeDeviceDiscovery,
            Self::Zone(_) => RType::Zone,
        }
    }

    pub fn from_value(rtype: RType, obj: Value) -> ApiResult<Self> {
        let res = match rtype {
            RType::BehaviorScript => Self::BehaviorScript(from_value(obj)?),
            RType::BehaviorInstance => Self::BehaviorInstance(from_value(obj)?),
            RType::Bridge => Self::Bridge(from_value(obj)?),
            RType::BridgeHome => Self::BridgeHome(from_value(obj)?),
            RType::Device => Self::Device(from_value(obj)?),
            RType::Entertainment => Self::Entertainment(from_value(obj)?),
            RType::GeofenceClient => Self::GeofenceClient(from_value(obj)?),
            RType::Geolocation => Self::Geolocation(from_value(obj)?),
            RType::GroupedLight => Self::GroupedLight(from_value(obj)?),
            RType::Homekit => Self::Homekit(from_value(obj)?),
            RType::Light => Self::Light(from_value(obj)?),
            RType::Matter => Self::Matter(from_value(obj)?),
            RType::PublicImage => Self::PublicImage(from_value(obj)?),
            RType::Room => Self::Room(from_value(obj)?),
            RType::Scene => Self::Scene(from_value(obj)?),
            RType::SmartScene => Self::SmartScene(from_value(obj)?),
            RType::ZigbeeConnectivity => Self::ZigbeeConnectivity(from_value(obj)?),
            RType::ZigbeeDeviceDiscovery => Self::ZigbeeDeviceDiscovery(from_value(obj)?),
            RType::Zone => Self::Zone(from_value(obj)?),
        };
        Ok(res)
    }
}

#[macro_export]
macro_rules! resource_conversion_impl {
    ( $name:ident ) => {
        impl<'a> TryFrom<&'a mut Resource> for &'a mut $name {
            type Error = ApiError;

            fn try_from(value: &'a mut Resource) -> Result<Self, Self::Error> {
                if let Resource::$name(obj) = value {
                    Ok(obj)
                } else {
                    Err(ApiError::WrongType(RType::Light, value.rtype()))
                }
            }
        }

        impl TryFrom<Resource> for $name {
            type Error = ApiError;

            fn try_from(value: Resource) -> Result<Self, Self::Error> {
                if let Resource::$name(obj) = value {
                    Ok(obj)
                } else {
                    Err(ApiError::WrongType(RType::Light, value.rtype()))
                }
            }
        }

        impl From<$name> for Resource {
            fn from(value: $name) -> Self {
                Resource::$name(value)
            }
        }
    };
}

resource_conversion_impl!(BehaviorScript);
resource_conversion_impl!(BehaviorInstance);
resource_conversion_impl!(Bridge);
resource_conversion_impl!(BridgeHome);
resource_conversion_impl!(Device);
resource_conversion_impl!(Entertainment);
resource_conversion_impl!(GeofenceClient);
resource_conversion_impl!(Geolocation);
resource_conversion_impl!(GroupedLight);
resource_conversion_impl!(Homekit);
resource_conversion_impl!(Light);
resource_conversion_impl!(Matter);
resource_conversion_impl!(PublicImage);
resource_conversion_impl!(Room);
resource_conversion_impl!(Scene);
resource_conversion_impl!(SmartScene);
resource_conversion_impl!(ZigbeeConnectivity);
resource_conversion_impl!(ZigbeeDeviceDiscovery);
resource_conversion_impl!(Zone);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bridge {
    pub bridge_id: String,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeHome {
    pub children: Vec<ResourceLink>,
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorGamut {
    pub red: XY,
    pub green: XY,
    pub blue: XY,
}

impl ColorGamut {
    pub const GAMUT_C: Self = Self {
        blue: XY {
            x: 0.1532,
            y: 0.0475,
        },
        green: XY {
            x: 0.1700,
            y: 0.7000,
        },
        red: XY {
            x: 0.6915,
            y: 0.3083,
        },
    };
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightColor {
    pub gamut: Option<ColorGamut>,
    pub gamut_type: Option<String>,
    pub xy: XY,
}

impl LightColor {
    #[must_use]
    pub fn dummy() -> Self {
        Self {
            gamut: Some(ColorGamut {
                red: XY {
                    x: 0.681_235,
                    y: 0.318_186,
                },
                green: XY {
                    x: 0.391_898,
                    y: 0.525_033,
                },
                blue: XY {
                    x: 0.150_241,
                    y: 0.027_116,
                },
            }),
            gamut_type: Some("Other".to_string()),
            xy: XY { x: 0.4573, y: 0.41 },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MirekSchema {
    mirek_minimum: u32,
    mirek_maximum: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperature {
    pub mirek: u32,
    pub mirek_schema: MirekSchema,
    pub mirek_valid: bool,
}

impl ColorTemperature {
    #[must_use]
    pub const fn dummy() -> Self {
        Self {
            mirek_schema: MirekSchema {
                mirek_maximum: 454,
                mirek_minimum: 250,
            },
            mirek_valid: true,
            mirek: 366,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dimming {
    pub brightness: f64,
    pub min_dim_level: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Delta {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub name: String,
    archetype: String,
}

impl Metadata {
    #[must_use]
    pub fn new(archetype: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            archetype: archetype.to_string(),
        }
    }

    #[must_use]
    pub fn room(archetype: RoomArchetypes, name: &str) -> Self {
        Self::new(json!(archetype).as_str().unwrap(), name)
    }

    #[must_use]
    pub fn hue_bridge(name: &str) -> Self {
        Self::new("bridge_v2", name)
    }

    #[must_use]
    pub fn spot_bulb(name: &str) -> Self {
        Self::new("spot_bulb", name)
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct On {
    pub on: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Reply<T> {
    pub data: Vec<T>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZone {
    pub time_zone: String,
}

impl TimeZone {
    #[must_use]
    pub fn best_guess() -> Self {
        Self {
            time_zone: best_guess_timezone(),
        }
    }
}
