use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hue::{
    api::On,
    update::{ColorTemperatureUpdate, ColorUpdate, DimmingUpdate},
};

use super::ResourceLink;

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "active", rename_all = "snake_case")]
pub enum SceneStatus {
    Inactive,
    Static,
    DynamicPalette,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SceneStatusUpdate {
    Active,
    Static,
    DynamicPalette,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    pub actions: Vec<SceneActionElement>,
    #[serde(default)]
    pub auto_dynamic: bool,
    pub group: ResourceLink,
    pub metadata: SceneMetadata,
    /* palette: { */
    /*     color: [], */
    /*     color_temperature: [ */
    /*         { */
    /*             color_temperature: { */
    /*                 mirek: u32 */
    /*             }, */
    /*             dimming: { */
    /*                 brightness: f64, */
    /*             } */
    /*         } */
    /*     ], */
    /*     dimming: [], */
    /*     effects: [] */
    /* }, */
    pub palette: Value,
    pub speed: f64,
    pub status: Option<SceneStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_temperature: Option<ColorTemperatureUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<DimmingUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<On>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneActionElement {
    pub action: SceneAction,
    pub target: ResourceLink,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appdata: Option<String>,
    pub image: Option<ResourceLink>,
    pub name: String,
}
