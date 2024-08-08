use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::hue::api::{ResourceLink, RoomArchetypes, SceneMetadata};
use crate::hue::best_guess_timezone;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bridge {
    pub bridge_id: String,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeHome {
    pub children: Vec<ResourceLink>,
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DollarRef {
    #[serde(rename = "$ref")]
    dref: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BehaviorScript {
    configuration_schema: DollarRef,
    description: String,
    max_number_instances: Option<u32>,
    metadata: Value,
    state_schema: DollarRef,
    supported_features: Vec<String>,
    trigger_schema: DollarRef,
    version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BehaviorInstance {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entertainment {
    equalizer: bool,
    owner: ResourceLink,
    proxy: bool,
    renderer: bool,
    renderer_reference: ResourceLink,
    segments: EntertainmentSegments,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntertainmentSegments {
    configurable: bool,
    max_segments: u32,
    segments: Vec<EntertainmentSegment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntertainmentSegment {
    length: u32,
    start: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeofenceClient {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Geolocation {
    is_configured: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Homekit {
    status: String,
    status_values: Vec<String>,
}

impl Default for Homekit {
    fn default() -> Self {
        Self {
            status: "unpaired".to_string(),
            status_values: vec![
                "pairing".to_string(),
                "paired".to_string(),
                "unpaired".to_string(),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Matter {
    has_qr_code: bool,
    max_fabrics: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicImage {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SmartScene {
    /* active_timeslot: { */
    /*     timeslot_id: 3, */
    /*     weekday: monday */
    /* }, */
    active_timeslot: Value,
    group: ResourceLink,
    metadata: SceneMetadata,
    state: String,
    transition_duration: u32,
    week_timeslots: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ZigbeeConnectivityStatus {
    ConnectivityIssue,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZigbeeConnectivity {
    mac_address: String,
    owner: ResourceLink,
    status: ZigbeeConnectivityStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZigbeeDeviceDiscovery {
    owner: ResourceLink,
    status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Zone {
    pub metadata: Metadata,
    pub children: Vec<ResourceLink>,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeZone {
    pub time_zone: String,
}

impl TimeZone {
    #[must_use]
    pub fn best_guess() -> Self {
        Self {
            time_zone: best_guess_timezone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub name: String,
    pub archetype: String,
}

impl Metadata {
    #[must_use]
    pub fn new(archetype: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            archetype: archetype.to_string(),
        }
    }

    #[must_use]
    pub fn room(archetype: RoomArchetypes, name: &str) -> Self {
        Self::new(json!(archetype).as_str().unwrap(), name)
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
