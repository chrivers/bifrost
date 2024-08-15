use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hue::api::{ColorTemperatureUpdate, ColorUpdate, DimmingUpdate, On, ResourceLink};
use crate::model::types::XY;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupedLight {
    pub alert: Value,
    pub dimming: Option<DimmingUpdate>,
    pub on: Option<On>,
    pub owner: ResourceLink,
    pub signaling: Value,
}

impl GroupedLight {
    #[must_use]
    pub fn as_brightness_opt(&self) -> Option<f64> {
        self.dimming.as_ref().map(|br| br.brightness)
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
    pub fn with_brightness(self, brightness: Option<f64>) -> Self {
        Self {
            dimming: brightness.map(|brightness| DimmingUpdate { brightness }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_on(self, on: Option<On>) -> Self {
        Self { on, ..self }
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
