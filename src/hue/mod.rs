pub mod date_format;
pub mod event;
pub mod scene_icons;
pub mod update;
pub mod api;
pub mod legacy_api;

pub const HUE_BRIDGE_V2_MODEL_ID: &str = "BSB002";

#[must_use]
pub fn best_guess_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "none".to_string())
}
