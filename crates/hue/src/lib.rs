#![doc = include_str!("../../../doc/hue-zigbee-format.md")]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod api;
pub mod clamp;
pub mod colorspace;
pub mod colortemp;
pub mod date_format;
pub mod devicedb;
pub mod diff;
pub mod effect_duration;
pub mod error;
pub mod flags;
pub mod gamma;
pub mod hs;
pub mod legacy_api;
pub mod scene_icons;
pub mod stream;
pub mod update;
pub mod version;
pub mod xy;
pub mod zigbee;

#[cfg(feature = "event")]
pub mod event;

#[cfg(feature = "mac")]
use mac_address::MacAddress;

pub const WIDE_GAMUT_MAX_X: f64 = 0.7347;
pub const WIDE_GAMUT_MAX_Y: f64 = 0.8264;

pub const HUE_BRIDGE_V2_MODEL_ID: &str = "BSB002";
pub const HUE_BRIDGE_V2_DEFAULT_SWVERSION: u64 = 1_970_084_010;
pub const HUE_BRIDGE_V2_DEFAULT_APIVERSION: &str = "1.70.0";

#[cfg(feature = "mac")]
#[must_use]
pub fn bridge_id_raw(mac: MacAddress) -> [u8; 8] {
    let b = mac.bytes();
    [b[0], b[1], b[2], 0xFF, 0xFE, b[3], b[4], b[5]]
}

#[cfg(feature = "mac")]
#[must_use]
pub fn bridge_id(mac: MacAddress) -> String {
    hex::encode(bridge_id_raw(mac))
}

#[cfg(test)]
mod tests {
    use mac_address::MacAddress;

    use crate::version::SwVersion;
    use crate::{HUE_BRIDGE_V2_DEFAULT_APIVERSION, HUE_BRIDGE_V2_DEFAULT_SWVERSION};

    #[macro_export]
    macro_rules! compare_float {
        ($expr:expr, $value:expr, $diff:expr) => {
            let a = $expr;
            let b = $value;
            eprintln!("{a} vs {b:.4} (diff {})", $diff);
            assert!((a - b).abs() < $diff);
        };
    }

    #[macro_export]
    macro_rules! compare {
        ($expr:expr, $value:expr) => {
            compare_float!($expr, $value, 1e-4)
        };
    }

    #[macro_export]
    macro_rules! compare_hs {
        ($a:expr, $b:expr) => {{
            compare!($a.hue, $b.hue);
            compare!($a.sat, $b.sat);
        }};
    }

    #[macro_export]
    macro_rules! compare_xy {
        ($expr:expr, $value:expr) => {
            let a = $expr;
            let b = $value;
            compare!(a.x, b.x);
            compare!(a.y, b.y);
        };
    }

    #[macro_export]
    macro_rules! compare_xy_quant {
        ($expr:expr, $value:expr) => {
            let a = $expr;
            let b = $value;
            compare_float!(a.x, b.x, 1e-3);
            compare_float!(a.y, b.y, 1e-3);
        };
    }

    #[macro_export]
    macro_rules! compare_rgb {
        ($a:expr, $b:expr) => {{
            eprintln!("Comparing r");
            compare!($a[0], $b[0]);
            eprintln!("Comparing g");
            compare!($a[1], $b[1]);
            eprintln!("Comparing b");
            compare!($a[2], $b[2]);
        }};
    }

    #[macro_export]
    macro_rules! compare_matrix {
        ($a:expr, $b:expr) => {
            zip($a, $b).for_each(|(a, b)| {
                compare!(a, b);
            });
        };
    }

    #[macro_export]
    macro_rules! compare_hsl_rgb {
        ($h:expr, $s:expr, $rgb:expr) => {{
            let sat = $s;
            compare_rgb!(XY::rgb_from_hsl(HS { hue: $h, sat }, 0.5), $rgb);
        }};
    }

    /// verify that `HUE_BRIDGE_V2_DEFAULT_SWVERSION` and
    /// `HUE_BRIDGE_V2_DEFAULT_APIVERSION` are synchronized
    #[test]
    fn default_version_match() {
        let ver = SwVersion::new(HUE_BRIDGE_V2_DEFAULT_SWVERSION, String::new());
        assert_eq!(
            HUE_BRIDGE_V2_DEFAULT_APIVERSION,
            ver.get_legacy_apiversion()
        );
    }

    #[test]
    fn bridge_id() {
        let mac = MacAddress::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let id = crate::bridge_id(mac);
        assert_eq!(id, "112233fffe445566");
    }

    #[test]
    fn bridge_id_raw() {
        let mac = MacAddress::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        let id = crate::bridge_id_raw(mac);
        assert_eq!(id, [0x11, 0x22, 0x33, 0xFF, 0xFE, 0x44, 0x55, 0x66]);
    }
}
