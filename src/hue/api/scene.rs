use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hue::api::{ColorTemperatureUpdate, ColorUpdate, DimmingUpdate, On, ResourceLink};

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SceneUpdate {
    pub actions: Option<Vec<SceneActionElement>>,
    pub recall: Option<SceneRecall>,
    pub metadata: Option<SceneMetadata>,
    pub palette: Option<Value>,
    pub speed: Option<f64>,
    pub auto_dynamic: Option<bool>,
}

impl SceneUpdate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_actions(self, actions: Option<Vec<SceneActionElement>>) -> Self {
        Self { actions, ..self }
    }

    #[must_use]
    pub fn with_recall_action(self, action: Option<SceneStatus>) -> Self {
        Self {
            recall: Some(SceneRecall {
                action: match action {
                    Some(SceneStatus::DynamicPalette) => Some(SceneStatusUpdate::DynamicPalette),
                    Some(SceneStatus::Static) => Some(SceneStatusUpdate::Active),
                    Some(SceneStatus::Inactive) | None => None,
                },
                duration: None,
                dimming: None,
            }),
            ..self
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneRecall {
    pub action: Option<SceneStatusUpdate>,
    pub duration: Option<u32>,
    pub dimming: Option<DimmingUpdate>,
}
