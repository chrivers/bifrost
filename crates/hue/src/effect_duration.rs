#[derive(PartialEq, Eq, Debug)]
pub struct EffectDuration(pub u8);

const RESOLUTION_01S_BASE: u8 = 0xFC;
const RESOLUTION_05S_BASE: u8 = 0xCC;
const RESOLUTION_15S_BASE: u8 = 0xA5;
const RESOLUTION_01M_BASE: u8 = 0x79;
const RESOLUTION_05M_BASE: u8 = 0x3F;

const RESOLUTION_01S: u32 = 1; // 01s.
const RESOLUTION_05S: u32 = 5; // 05s.
const RESOLUTION_15S: u32 = 15; // 15s.
const RESOLUTION_01M: u32 = 60; // 01min.
const RESOLUTION_05M: u32 = 5 * 600; // 05min.

const RESOLUTION_01S_LIMIT: u32 = 60; // 01min.
const RESOLUTION_05S_LIMIT: u32 = 5 * 60; // 05min.
const RESOLUTION_15S_LIMIT: u32 = 15 * 60; // 15min.
const RESOLUTION_01M_LIMIT: u32 = 60 * 60; // 60min.
const RESOLUTION_05M_LIMIT: u32 = 6 * 60 * 60; // 06hrs.

impl EffectDuration {
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_seconds(seconds: u32) -> Self {
        let (base, resolution) = if seconds < RESOLUTION_01S_LIMIT {
            (RESOLUTION_01S_BASE, RESOLUTION_01S)
        } else if seconds < RESOLUTION_05S_LIMIT {
            (RESOLUTION_05S_BASE, RESOLUTION_05S)
        } else if seconds < RESOLUTION_15S_LIMIT {
            (RESOLUTION_15S_BASE, RESOLUTION_15S)
        } else if seconds < RESOLUTION_01M_LIMIT {
            (RESOLUTION_01M_BASE, RESOLUTION_01M)
        } else if seconds < RESOLUTION_05M_LIMIT {
            (RESOLUTION_05M_BASE, RESOLUTION_05M)
        } else {
            return Self(0);
        };
        Self(base - ((seconds / resolution) as u8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn seconds_to_effect_duration() {
        // sniffed from the real Hue hub
        let values = vec![
            (5, 145),
            (10, 125),
            (15, 106),
            (20, 101),
            (25, 96),
            (30, 91),
            (35, 86),
            (40, 81),
            (45, 76),
            (50, 71),
            (55, 66),
            (60, 62),
        ];
        for (input, output) in values {
            assert_eq!(
                EffectDuration::from_seconds(input * 60),
                EffectDuration(output)
            );
        }
    }
}
