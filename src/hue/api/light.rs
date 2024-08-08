use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::hue::api::ResourceLink;
use crate::{types::XY, z2m::update::DeviceColorMode};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    /* This field does not exist in the hue api, but we need it to keep track of
     * last-used color mode for a light. */
    #[serde(skip, default)]
    pub color_mode: Option<DeviceColorMode>,

    pub owner: ResourceLink,
    pub metadata: LightMetadata,

    pub alert: Value,
    pub color: LightColor,
    pub color_temperature: ColorTemperature,
    pub color_temperature_delta: Delta,
    pub dimming: Dimming,
    pub dimming_delta: Delta,
    pub dynamics: Value,
    pub effects: Value,
    pub identify: Value,
    pub mode: String,
    pub on: On,
    pub powerup: Value,
    pub signaling: Value,
    /* powerup: { */
    /*     color: { */
    /*         color_temperature: { */
    /*             mirek: 366 */
    /*         }, */
    /*         mode: color_temperature */
    /*     }, */
    /*     configured: true, */
    /*     dimming: { */
    /*         dimming: { */
    /*             brightness: 100 */
    /*         }, */
    /*         mode: dimming */
    /*     }, */
    /*     on: { */
    /*         mode: on, */
    /*         on: { */
    /*             on: true */
    /*         } */
    /*     }, */
    /*     preset: safety */
    /* }, */
    /* signaling: { */
    /*     signal_values: [ */
    /*         no_signal, */
    /*         on_off, */
    /*         on_off_color, */
    /*         alternating */
    /*     ] */
    /* }, */
}

impl Light {
    #[must_use]
    pub fn new(owner: ResourceLink) -> Self {
        Self {
            alert: json!({"action_values": ["breathe"]}),
            color_mode: None,
            color: LightColor::dummy(),
            color_temperature: ColorTemperature::dummy(),
            color_temperature_delta: Delta {},
            dimming: Dimming {
                brightness: 100.0,
                min_dim_level: Some(0.2),
            },
            dimming_delta: Delta {},
            dynamics: json!({
                "speed": 0,
                "speed_valid": false,
                "status": "none",
                "status_values": [
                    "none",
                    "dynamic_palette",
                ]
            }),
            effects: json!({
                "effect_values": [
                    "no_effect",
                    "candle",
                    "fire",
                    "prism"
                ],
                "status": "no_effect",
                "status_values": [
                    "no_effect",
                    "candle",
                    "fire",
                    "prism"
                ]
            }),
            identify: json!({}),
            mode: "normal".to_string(),
            on: On { on: true },
            metadata: LightMetadata {
                archetype: "spot_bulb".to_string(),
                name: "Light 1".to_string(),
            },
            owner,
            powerup: json!({
                "color": {
                    "color": {
                        "xy": XY { x: 0.4573, y: 0.41 },
                    },
                    "color_temperature": {
                        "mirek": 366
                    },
                    "mode": "color_temperature"
                },
                "configured": true,
                "dimming": {
                    "dimming": {
                        "brightness": 100
                    },
                    "mode": "dimming"
                },
                "on": {
                    "mode": "on",
                    "on": {
                        "on": true
                    }
                },
                "preset": "safety"
            }),
            signaling: json!({
                "signal_values": [
                    "no_signal",
                    "on_off",
                    "on_off_color",
                    "alternating"
                ]
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightMetadata {
    pub name: String,
    pub archetype: String,
}

impl LightMetadata {
    #[must_use]
    pub fn new(archetype: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            archetype: archetype.to_string(),
        }
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightColor {
    pub gamut: Option<ColorGamut>,
    pub gamut_type: String,
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
            gamut_type: "Other".to_string(),
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
