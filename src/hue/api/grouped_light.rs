use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ColorTemperature, Dimming, LightColor, On, ResourceLink};
use crate::z2m::update::DeviceColorMode;

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
