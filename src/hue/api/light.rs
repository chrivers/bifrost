use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hue::api::{Metadata, ResourceLink};
use crate::types::XY;
use crate::z2m::api::Expose;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    pub owner: ResourceLink,
    pub metadata: Metadata,

    pub alert: Option<Vec<Value>>,
    pub color: Option<LightColor>,
    pub color_temperature: Option<ColorTemperature>,
    pub dimming: Option<Dimming>,
    pub dynamics: Option<LightDynamics>,
    pub effects: Option<LightEffects>,
    pub timed_effects: Option<LightTimedEffects>,
    pub mode: LightMode,
    pub on: On,
    pub powerup: Option<LightPowerup>,
    pub signaling: Option<LightSignaling>,
}

impl Light {
    #[must_use]
    pub const fn new(owner: ResourceLink, metadata: Metadata) -> Self {
        Self {
            alert: None,
            color: None,
            color_temperature: None,
            dimming: None,
            dynamics: None,
            effects: None,
            timed_effects: None,
            mode: LightMode::Normal,
            on: On { on: true },
            metadata,
            owner,
            powerup: None,
            signaling: None,
        }
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum LightMode {
    #[default]
    Normal,
    Streaming,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LightPowerupPreset {
    Safety,
    Powerfail,
    LastOnState,
    Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightPowerup {
    pub preset: LightPowerupPreset,
    #[serde(flatten)]
    pub data: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightSignaling {
    pub signal_values: Vec<LightSignal>,
    pub status: Value,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum LightSignal {
    #[default]
    NoSignal,
    OnOff,
    OnOffColor,
    Alternating,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LightDynamicsStatus {
    DynamicPalette,
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightDynamics {
    pub status: LightDynamicsStatus,
    pub status_values: Value,
    pub speed: f64,
    pub speed_valid: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffects {
    pub status_values: Value,
    pub status: Value,
    pub effect_values: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightTimedEffects {
    pub status_values: Value,
    pub status: Value,
    pub effect_values: Value,
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
    pub fn with_brightness(self, dim: Option<Dimming>) -> Self {
        Self {
            dimming: dim.map(|d| DimmingUpdate {
                brightness: d.brightness,
            }),
            ..self
        }
    }

    #[must_use]
    pub const fn with_on(self, on: Option<On>) -> Self {
        Self { on, ..self }
    }

    #[must_use]
    pub fn with_color_temperature(self, mirek: Option<u32>) -> Self {
        Self {
            color_temperature: mirek.map(|mirek| ColorTemperatureUpdate { mirek }),
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
pub struct Delta {}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct On {
    pub on: bool,
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

    pub const IKEA_ESTIMATE: Self = Self {
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
    };
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GamutType {
    A,
    B,
    C,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightColor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamut: Option<ColorGamut>,
    pub gamut_type: GamutType,
    pub xy: XY,
}

impl LightColor {
    #[must_use]
    pub const fn new(xy: XY) -> Self {
        Self {
            gamut: None,
            gamut_type: GamutType::Other,
            xy,
        }
    }

    #[must_use]
    pub const fn extract_from_expose(expose: &Expose) -> Option<Self> {
        let Expose::Composite(_) = expose else {
            return None;
        };

        Some(Self {
            gamut: Some(ColorGamut::GAMUT_C),
            gamut_type: GamutType::C,
            xy: XY::D65_WHITE_POINT,
        })
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct MirekSchema {
    pub mirek_minimum: u32,
    pub mirek_maximum: u32,
}

impl MirekSchema {
    pub const DEFAULT: Self = Self {
        mirek_minimum: 153,
        mirek_maximum: 500,
    };
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperature {
    pub mirek: Option<u32>,
    pub mirek_schema: MirekSchema,
    pub mirek_valid: bool,
}

impl ColorTemperature {
    #[must_use]
    pub fn extract_from_expose(expose: &Expose) -> Option<Self> {
        let Expose::Numeric(num) = expose else {
            return None;
        };

        let schema_opt = num.extract_mirek_schema();
        let mirek_valid = schema_opt.is_some();
        let mirek_schema = schema_opt.unwrap_or(MirekSchema::DEFAULT);
        let mirek = None;

        Some(Self {
            mirek,
            mirek_schema,
            mirek_valid,
        })
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct Dimming {
    pub brightness: f64,
    pub min_dim_level: Option<f64>,
}
