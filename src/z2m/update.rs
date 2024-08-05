use crate::{types::XY, z2m::api::DeviceColor};

use super::api::{DeviceState, DeviceUpdate};

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
