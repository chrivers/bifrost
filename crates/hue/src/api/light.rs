use std::collections::BTreeSet;
use std::ops::{AddAssign, Sub};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::api::{DeviceArchetype, Identify, Metadata, MetadataUpdate, ResourceLink, Stub};
use crate::hs::HS;
use crate::xy::XY;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    pub owner: ResourceLink,
    pub metadata: LightMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_data: Option<LightProductData>,

    pub alert: Option<LightAlert>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<LightColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature: Option<ColorTemperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature_delta: Option<Stub>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<Dimming>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming_delta: Option<Stub>,
    pub dynamics: Option<LightDynamics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<LightEffects>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects_v2: Option<LightEffectsV2>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<LightGradient>,
    #[serde(default)]
    pub identify: Identify,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_effects: Option<LightTimedEffects>,
    pub mode: LightMode,
    pub on: On,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powerup: Option<LightPowerup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signaling: Option<LightSignaling>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LightFunction {
    Functional,
    Decorative,
    Mixed,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct LightMetadata {
    pub name: String,
    pub archetype: DeviceArchetype,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<LightFunction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_mired: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightProductData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<LightFunction>,
}

impl LightMetadata {
    #[must_use]
    pub fn new(archetype: DeviceArchetype, name: &str) -> Self {
        Self {
            archetype,
            name: name.to_string(),
            function: Some(LightFunction::Decorative),
            fixed_mired: None,
        }
    }
}

impl From<LightMetadata> for Metadata {
    fn from(value: LightMetadata) -> Self {
        Self {
            name: value.name,
            archetype: value.archetype,
        }
    }
}

impl Light {
    #[must_use]
    pub fn new(owner: ResourceLink, metadata: LightMetadata) -> Self {
        Self {
            alert: Some(LightAlert {
                action_values: BTreeSet::from([String::from("breathe")]),
            }),
            color: None,
            color_temperature: None,
            color_temperature_delta: Some(Stub),
            dimming: None,
            dimming_delta: Some(Stub),
            dynamics: Some(LightDynamics::default()),
            effects: None,
            effects_v2: None,
            service_id: Some(0),
            gradient: None,
            identify: Identify {},
            timed_effects: Some(LightTimedEffects {
                status_values: json!(["no_effect", "sunrise", "sunset"]),
                status: json!("no_effect"),
                effect_values: json!(["no_effect", "sunrise", "sunset"]),
            }),
            mode: LightMode::Normal,
            on: On { on: true },
            product_data: Some(LightProductData {
                function: Some(LightFunction::Decorative),
            }),
            metadata,
            owner,
            powerup: Some(LightPowerup {
                preset: LightPowerupPreset::Safety,
                configured: true,
                on: LightPowerupOn::On {
                    on: On { on: true },
                },
                dimming: LightPowerupDimming::Dimming {
                    dimming: DimmingUpdate { brightness: 100.0 },
                },
                color: LightPowerupColor::ColorTemperature {
                    color_temperature: ColorTemperatureUpdate { mirek: 366 },
                },
            }),
            signaling: Some(LightSignaling {
                signal_values: vec![
                    LightSignal::NoSignal,
                    LightSignal::OnOff,
                    LightSignal::OnOffColor,
                    LightSignal::Alternating,
                ],
                status: Value::Null,
            }),
        }
    }

    #[must_use]
    pub fn as_dimming_opt(&self) -> Option<DimmingUpdate> {
        self.dimming.as_ref().map(|dim| DimmingUpdate {
            brightness: dim.brightness,
        })
    }

    #[must_use]
    pub fn as_mirek_opt(&self) -> Option<u16> {
        self.color_temperature.as_ref().and_then(|ct| ct.mirek)
    }

    #[must_use]
    pub fn as_color_opt(&self) -> Option<XY> {
        self.color.as_ref().map(|col| col.xy)
    }

    #[must_use]
    pub fn as_gradient_opt(&self) -> Option<Vec<XY>> {
        self.gradient
            .as_ref()
            .map(|grad| grad.points.iter().map(|p| p.color.xy).collect())
    }
}

impl AddAssign<LightUpdate> for Light {
    fn add_assign(&mut self, upd: LightUpdate) {
        if let Some(md) = upd.metadata {
            if let Some(name) = md.name {
                self.metadata.name = name;
            }
            if let Some(archetype) = md.archetype {
                self.metadata.archetype = archetype;
            }
        }

        if let Some(state) = upd.on {
            self.on.on = state.on;
        }

        if let Some(dim) = &mut self.dimming {
            if let Some(b) = upd.dimming {
                dim.brightness = b.brightness;
            }
        }

        if let Some(ct) = &mut self.color_temperature {
            ct.mirek = upd.color_temperature.map(|c| c.mirek);
        }

        if let Some(col) = upd.color {
            if let Some(lcol) = &mut self.color {
                lcol.xy = col.xy;
            }
            if let Some(ct) = &mut self.color_temperature {
                ct.mirek = None;
            }
        }

        if let Some(grad) = &mut self.gradient {
            if let Some(grupd) = upd.gradient {
                grad.mode = grupd.mode.unwrap_or(grad.mode);
                grad.points = grupd.points;
            }
        }
    }
}

#[allow(clippy::if_not_else)]
impl Sub<&Light> for &Light {
    type Output = LightUpdate;

    fn sub(self, rhs: &Light) -> Self::Output {
        let mut upd = Self::Output::default();

        if self.metadata != rhs.metadata {
            upd.metadata = Some(MetadataUpdate {
                name: if self.metadata.name != rhs.metadata.name {
                    Some(rhs.metadata.name.clone())
                } else {
                    None
                },
                archetype: if self.metadata.archetype != rhs.metadata.archetype {
                    Some(rhs.metadata.archetype.clone())
                } else {
                    None
                },
            });
        }

        if self.on != rhs.on {
            upd.on = Some(rhs.on);
        }

        if self.dimming != rhs.dimming {
            upd.dimming = rhs.dimming.map(Into::into);
        }

        if self.as_mirek_opt() != rhs.as_mirek_opt() {
            upd = upd.with_color_temperature(rhs.as_mirek_opt());
        }

        if self.as_color_opt() != rhs.as_color_opt() {
            upd = upd.with_color_xy(rhs.as_color_opt());
        }

        if self.gradient != rhs.gradient {
            upd = upd.with_color_xy(rhs.as_color_opt());
        }

        upd
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LightMode {
    #[default]
    Normal,
    Streaming,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightAlert {
    action_values: BTreeSet<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialOrd, Ord, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LightGradientMode {
    #[default]
    InterpolatedPalette,
    InterpolatedPaletteMirrored,
    RandomPixelated,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct LightGradientPoint {
    pub color: ColorUpdate,
}

impl LightGradientPoint {
    #[must_use]
    pub const fn xy(xy: XY) -> Self {
        Self {
            color: ColorUpdate { xy },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LightGradient {
    pub mode: LightGradientMode,
    pub mode_values: BTreeSet<LightGradientMode>,
    pub points_capable: u32,
    pub points: Vec<LightGradientPoint>,
    pub pixel_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightGradientUpdate {
    #[serde(default)]
    pub mode: Option<LightGradientMode>,
    #[serde(default)]
    pub points: Vec<LightGradientPoint>,
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

    pub configured: bool,
    #[serde(default, skip_serializing_if = "LightPowerupOn::is_none")]
    pub on: LightPowerupOn,
    #[serde(default, skip_serializing_if = "LightPowerupDimming::is_none")]
    pub dimming: LightPowerupDimming,
    #[serde(default, skip_serializing_if = "LightPowerupColor::is_none")]
    pub color: LightPowerupColor,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LightPowerupOn {
    // Not a real powerup.on.mode option, but used to indicate that
    // powerup.on itself is null
    #[default]
    None,
    Previous,
    On {
        on: On,
    },
}

impl LightPowerupOn {
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LightPowerupColor {
    // Not a real powerup.color.mode option, but used to indicate that
    // powerup.color itself is null
    #[default]
    None,
    Previous,
    Color {
        color: ColorUpdate,
    },
    ColorTemperature {
        color_temperature: ColorTemperatureUpdate,
    },
}

impl LightPowerupColor {
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LightPowerupDimming {
    // Not a real powerup.dimming.mode option, but used to indicate that
    // powerup.dimming itself is null
    #[default]
    None,
    Previous,
    Dimming {
        dimming: DimmingUpdate,
    },
}

impl LightPowerupDimming {
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightSignaling {
    pub signal_values: Vec<LightSignal>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Value::is_null")]
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
    pub status_values: Vec<LightDynamicsStatus>,
    pub speed: f64,
    pub speed_valid: bool,
}

impl Default for LightDynamics {
    fn default() -> Self {
        Self {
            status: LightDynamicsStatus::None,
            status_values: vec![
                LightDynamicsStatus::None,
                LightDynamicsStatus::DynamicPalette,
            ],
            speed: 0.0,
            speed_valid: false,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum LightEffect {
    #[default]
    NoEffect,
    Prism,
    Opal,
    Glisten,
    Sparkle,
    Fire,
    Candle,
    Underwater,
    Cosmos,
    Sunbeam,
    Enchant,
    Sunrise,
}

impl LightEffect {
    pub const ALL: [Self; 11] = [
        Self::NoEffect,
        Self::Candle,
        Self::Fire,
        Self::Prism,
        Self::Sparkle,
        Self::Opal,
        Self::Glisten,
        Self::Underwater,
        Self::Cosmos,
        Self::Sunbeam,
        Self::Enchant,
    ];
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffects {
    pub status_values: Vec<LightEffect>,
    pub status: LightEffect,
    pub effect_values: Vec<LightEffect>,
}

impl LightEffects {
    #[must_use]
    pub fn all() -> Self {
        Self {
            status_values: Vec::from(LightEffect::ALL),
            status: LightEffect::NoEffect,
            effect_values: Vec::from(LightEffect::ALL),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffectsV2 {
    pub action: LightEffectValues,
    pub status: LightEffectStatus,
}

impl LightEffectsV2 {
    #[must_use]
    pub fn all() -> Self {
        Self {
            action: LightEffectValues {
                effect_values: Vec::from(LightEffect::ALL),
            },
            status: LightEffectStatus {
                effect: LightEffect::NoEffect,
                effect_values: Vec::from(LightEffect::ALL),
                parameters: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffectsV2Update {
    #[serde(default)]
    pub action: Option<LightEffectActionUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LightEffectActionUpdate {
    #[serde(default)]
    pub effect: Option<LightEffect>,
    pub parameters: LightEffectParameters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LightEffectParameters {
    #[serde(default)]
    pub color: Option<ColorUpdate>,
    #[serde(default)]
    pub color_temperature: Option<ColorTemperatureUpdate>,
    pub speed: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffectValues {
    pub effect_values: Vec<LightEffect>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightEffectStatus {
    pub effect: LightEffect,
    pub effect_values: Vec<LightEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
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
    pub metadata: Option<MetadataUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<On>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<DimmingUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature: Option<ColorTemperatureUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<LightGradientUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects_v2: Option<LightEffectsV2Update>,

    #[serde(skip)]
    pub transition: Option<f64>,
}

impl LightUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_brightness(self, dim: Option<impl Into<f64>>) -> Self {
        Self {
            dimming: dim.map(Into::into).map(DimmingUpdate::new),
            ..self
        }
    }

    #[must_use]
    pub fn with_on(self, on: impl Into<Option<On>>) -> Self {
        Self {
            on: on.into(),
            ..self
        }
    }

    #[must_use]
    pub fn with_color_temperature(self, mirek: impl Into<Option<u16>>) -> Self {
        Self {
            color_temperature: mirek.into().map(ColorTemperatureUpdate::new),
            ..self
        }
    }

    #[must_use]
    pub fn with_color_xy(self, xy: impl Into<Option<XY>>) -> Self {
        Self {
            color: self.color.or_else(|| xy.into().map(ColorUpdate::new)),
            ..self
        }
    }

    #[must_use]
    pub fn with_color_hs(self, hs: impl Into<Option<HS>>) -> Self {
        Self {
            color: hs.into().map(|hs| XY::from_hs(hs).0).map(ColorUpdate::new),
            ..self
        }
    }

    #[must_use]
    pub fn with_gradient(self, grad: Option<Vec<XY>>) -> Self {
        Self {
            gradient: grad.map(|colors| LightGradientUpdate {
                mode: None,
                points: colors.into_iter().map(LightGradientPoint::xy).collect(),
            }),
            ..self
        }
    }

    #[must_use]
    pub fn with_transition(self, transition: Option<impl Into<f64>>) -> Self {
        Self {
            transition: transition.map(Into::into),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct DimmingUpdate {
    pub brightness: f64,
}

impl DimmingUpdate {
    #[must_use]
    pub const fn new(brightness: f64) -> Self {
        Self { brightness }
    }
}

impl From<Dimming> for DimmingUpdate {
    fn from(value: Dimming) -> Self {
        Self {
            brightness: value.brightness,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Delta {}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct On {
    pub on: bool,
}

impl On {
    #[must_use]
    pub const fn new(on: bool) -> Self {
        Self { on }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct ColorUpdate {
    pub xy: XY,
}

impl ColorUpdate {
    #[must_use]
    pub const fn new(xy: XY) -> Self {
        Self { xy }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ColorTemperatureUpdate {
    pub mirek: u16,
}

impl ColorTemperatureUpdate {
    #[must_use]
    pub const fn new(mirek: u16) -> Self {
        Self { mirek }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorGamut {
    pub red: XY,
    pub green: XY,
    pub blue: XY,
}

impl ColorGamut {
    pub const GAMUT_C: Self = Self {
        red: XY {
            x: 0.6915,
            y: 0.3083,
        },
        green: XY {
            x: 0.1700,
            y: 0.7000,
        },
        blue: XY {
            x: 0.1532,
            y: 0.0475,
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
    #[serde(rename = "other")]
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
    pub mirek: Option<u16>,
    pub mirek_schema: MirekSchema,
    pub mirek_valid: bool,
}

impl From<ColorTemperature> for Option<ColorTemperatureUpdate> {
    fn from(value: ColorTemperature) -> Self {
        value.mirek.map(ColorTemperatureUpdate::new)
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Dimming {
    pub brightness: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_dim_level: Option<f64>,
}

impl From<Dimming> for f64 {
    fn from(value: Dimming) -> Self {
        value.brightness
    }
}
