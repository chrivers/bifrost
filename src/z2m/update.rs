use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{hue::api::On, types::XY};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DeviceUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DeviceState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_mode: Option<DeviceColorMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<DeviceColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkquality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_options: Option<ColorOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temp_startup: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_config: Option<LevelConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub elapsed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub power_on_behavior: Option<PowerOnBehavior>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub update: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<f64>,
}

impl DeviceUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_state(self, state: Option<bool>) -> Self {
        Self {
            state: state.map(DeviceState::from),
            ..self
        }
    }

    #[must_use]
    pub fn with_brightness(self, brightness: Option<f64>) -> Self {
        Self { brightness, ..self }
    }

    #[must_use]
    pub fn with_color_temp(self, mirek: Option<u32>) -> Self {
        Self {
            color_temp: mirek,
            ..self
        }
    }

    #[must_use]
    pub fn with_color_xy(self, xy: Option<XY>) -> Self {
        Self {
            color: xy.map(DeviceColor::xy),
            ..self
        }
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeviceColor {
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    h: Option<f64>,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    s: Option<f64>,

    pub hue: Option<f64>,
    pub saturation: Option<f64>,

    #[serde(flatten)]
    pub xy: XY,
}

impl DeviceColor {
    #[must_use]
    pub const fn xy(xy: XY) -> Self {
        Self {
            h: None,
            s: None,
            hue: None,
            saturation: None,
            xy,
        }
    }

    #[must_use]
    pub const fn hs(h: f64, s: f64) -> Self {
        Self {
            h: None,
            s: None,
            hue: Some(h),
            saturation: Some(s),
            xy: XY::new(0.0, 0.0),
        }
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub enum PowerOnBehavior {
    #[default]
    Unknown,

    #[serde(rename = "on")]
    On,

    #[serde(rename = "off")]
    Off,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColorOptions {
    pub execute_if_off: bool,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LevelConfig {
    pub execute_if_off: Option<bool>,
    pub on_off_transition_time: Option<u16>,
    pub on_transition_time: Option<u16>,
    pub off_transition_time: Option<u16>,
    pub current_level_startup: Option<CurrentLevelStartup>,
    pub on_level: Option<OnLevel>,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CurrentLevelStartup {
    Previous,
    Minimum,
    #[serde(untagged)]
    Value(u8),
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OnLevel {
    Previous,
    #[serde(untagged)]
    Value(u8),
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DeviceColorMode {
    ColorTemp,
    Xy,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceState {
    On,
    Off,
}

impl From<bool> for DeviceState {
    fn from(value: bool) -> Self {
        if value {
            Self::On
        } else {
            Self::Off
        }
    }
}

impl From<DeviceState> for bool {
    fn from(value: DeviceState) -> Self {
        match value {
            DeviceState::On => true,
            DeviceState::Off => false,
        }
    }
}

impl From<DeviceState> for On {
    fn from(value: DeviceState) -> Self {
        Self { on: value.into() }
    }
}
