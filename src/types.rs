use serde::{Deserialize, Serialize};

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct XY {
    pub x: f64,
    pub y: f64,
}

impl XY {
    pub const D65_WHITE_POINT: Self = Self {
        x: 0.31271,
        y: 0.32902,
    };

    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
