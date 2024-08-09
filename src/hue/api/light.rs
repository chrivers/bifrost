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
    pub dynamics: Option<LightDynamics>,
    pub effects: Option<LightEffects>,
    pub timed_effects: Option<LightTimedEffects>,
    pub identify: Value,
    pub mode: LightMode,
    pub on: On,
    pub powerup: Option<LightPowerup>,
    pub signaling: Option<LightSignaling>,
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
            dynamics: None,
            effects: None,
            timed_effects: None,
            identify: json!({}),
            mode: LightMode::Normal,
            on: On { on: true },
            metadata: LightMetadata {
                archetype: "spot_bulb".to_string(),
                name: "Light 1".to_string(),
            },
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

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LightArchetype {
    UnknownArchetype,
    ClassicBulb,
    SultanBulb,
    FloodBulb,
    SpotBulb,
    CandleBulb,
    LusterBulb,
    PendantRound,
    PendantLong,
    CeilingRound,
    CeilingSquare,
    FloorShade,
    FloorLantern,
    TableShade,
    RecessedCeiling,
    RecessedFloor,
    SingleSpot,
    DoubleSpot,
    TableWash,
    WallLantern,
    WallShade,
    FlexibleLamp,
    GroundSpot,
    WallSpot,
    Plug,
    HueGo,
    HueLightstrip,
    HueIris,
    HueBloom,
    Bollard,
    WallWasher,
    HuePlay,
    VintageBulb,
    VintageCandleBulb,
    EllipseBulb,
    TriangleBulb,
    SmallGlobeBulb,
    LargeGlobeBulb,
    EdisonBulb,
    ChristmasTree,
    StringLight,
    HueCentris,
    HueLightstripTv,
    HueLightstripPc,
    HueTube,
    HueSigne,
    PendantSpot,
    CeilingHorizontal,
    CeilingTube,
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
    pub gamut: Option<ColorGamut>,
    pub gamut_type: GamutType,
    pub xy: XY,
}

impl LightColor {
    #[must_use]
    pub const fn dummy() -> Self {
        Self {
            gamut: Some(ColorGamut::IKEA_ESTIMATE),
            gamut_type: GamutType::Other,
            xy: XY { x: 0.4573, y: 0.41 },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MirekSchema {
    pub mirek_minimum: u32,
    pub mirek_maximum: u32,
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
