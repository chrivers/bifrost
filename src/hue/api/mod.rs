mod device;
mod grouped_light;
mod light;
mod resource;
mod room;
mod scene;
mod stubs;
mod update;

pub use device::{Device, DeviceArchetype, DeviceProductData};
pub use grouped_light::{GroupedLight, GroupedLightUpdate};
pub use light::{
    ColorGamut, ColorTemperature, ColorTemperatureUpdate, ColorUpdate, Delta, Dimming,
    DimmingUpdate, GamutType, Light, LightColor, LightUpdate, MirekSchema, On,
};
pub use resource::{RType, ResourceLink, ResourceRecord};
pub use room::{Room, RoomArchetype, RoomMetadata};
pub use scene::{
    Scene, SceneAction, SceneActionElement, SceneMetadata, SceneRecall, SceneStatus,
    SceneStatusUpdate, SceneUpdate,
};
pub use stubs::{
    BehaviorInstance, BehaviorScript, Bridge, BridgeHome, Button, ButtonData, ButtonMetadata,
    ButtonReport, DollarRef, Entertainment, EntertainmentSegment, EntertainmentSegments,
    GeofenceClient, Geolocation, Homekit, Matter, Metadata, PublicImage, SmartScene, TimeZone,
    ZigbeeConnectivity, ZigbeeConnectivityStatus, ZigbeeDeviceDiscovery, Zone,
};
pub use update::{Update, UpdateRecord};

use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    BehaviorInstance(BehaviorInstance),
    BehaviorScript(BehaviorScript),
    Bridge(Bridge),
    BridgeHome(BridgeHome),
    Button(Button),
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
            Self::Button(_) => RType::Button,
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
            RType::Button => Self::Button(from_value(obj)?),
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

        impl<'a> TryFrom<&'a Resource> for &'a $name {
            type Error = ApiError;

            fn try_from(value: &'a Resource) -> Result<Self, Self::Error> {
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
resource_conversion_impl!(Button);
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

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Reply<T> {
    pub data: Vec<T>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct V1ReplyBuilder<'a> {
    prefix: String,
    success: Vec<(&'a str, Value)>,
}

impl<'a> V1ReplyBuilder<'a> {
    #[must_use]
    pub const fn new(prefix: String) -> Self {
        Self {
            prefix,
            success: vec![],
        }
    }

    pub fn add<T: Serialize>(mut self, name: &'a str, value: T) -> ApiResult<Self> {
        self.success.push((name, serde_json::to_value(value)?));
        Ok(self)
    }

    pub fn add_option<T: Serialize>(mut self, name: &'a str, value: Option<T>) -> ApiResult<Self> {
        if let Some(val) = value {
            self.success.push((name, serde_json::to_value(val)?));
        }
        Ok(self)
    }

    #[must_use]
    pub fn json(self) -> Value {
        let mut json = vec![];
        let prefix = self.prefix;
        for (name, value) in self.success {
            json.push(json!({"success": {format!("{prefix}/{name}"): value}}));
        }
        json!(json)
    }
}
