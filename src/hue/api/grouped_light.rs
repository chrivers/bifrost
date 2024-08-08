use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hue::api::{
    ColorTemperature, ColorTemperatureUpdate, ColorUpdate, Dimming, DimmingUpdate, LightColor, On,
    ResourceLink,
};
use crate::{types::XY, z2m::update::DeviceColorMode};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupedLight {
    /* This field does not exist in the hue api, but we need it to keep track of
     * last-used color mode for a light. */
    #[serde(skip, default)]
    pub color_mode: Option<DeviceColorMode>,

    pub alert: Value,
    pub color: LightColor,
    pub color_temperature: ColorTemperature,
    pub color_temperature_delta: Value,
    pub dimming: Dimming,
    pub dimming_delta: Value,
    pub dynamics: Value,
    pub on: On,
    pub owner: ResourceLink,
    pub signaling: Value,
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
