use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::hue::best_guess_timezone;

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    BehaviorScript,
    Bridge,
    BridgeHome,
    Device,
    Entertainment,
    GeofenceClient,
    Geolocation,
    GroupedLight,
    Homekit,
    Light,
    Matter,
    PublicImage,
    Room,
    Scene,
    SmartScene,
    ZigbeeConnectivity,
    ZigbeeDeviceDiscovery,
}

impl ResourceType {
    #[must_use]
    pub fn link(self) -> ResourceLink {
        ResourceLink {
            rid: Uuid::new_v4(),
            rtype: self,
        }
    }

    #[must_use]
    pub fn link_to(self, rid: Uuid) -> ResourceLink {
        ResourceLink {
            rid,
            rtype: self,
        }
    }
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
pub struct Bridge {
    pub bridge_id: String,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeHome {
    children: Vec<ResourceLink>,
    id_v1: Option<String>,
    services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub product_data: DeviceProductData,
    pub metadata: Metadata,
    pub identify: Value,
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceProductData {
    pub model_id: String,
    pub manufacturer_name: String,
    pub product_name: String,
    pub product_archetype: String,
    pub certified: bool,
    pub software_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entertainment {
    equalizer: bool,
    id_v1: Option<String>,
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
pub struct GroupedLight {
    alert: Value,
    color: Value,
    color_temperature: Value,
    color_temperature_delta: Value,
    dimming: Value,
    dimming_delta: Value,
    dynamics: Value,
    id_v1: Option<String>,
    on: On,
    owner: ResourceLink,
    signaling: Value,
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
pub struct XY {
    x: f32,
    y: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorGamut {
    red: XY,
    green: XY,
    blue: XY,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightColor {
    gamut: Option<ColorGamut>,
    gamut_type: Option<String>,
    xy: XY,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MirekSchema {
    mirek_minimum: u32,
    mirek_maximum: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperatureUpdate {
    mirek: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperature {
    mirek: u32,
    mirek_schema: MirekSchema,
    mirek_valid: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dimming {
    brightness: f64,
    min_dim_level: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DimmingUpdate {
    brightness: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Delta {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub name: String,
    archetype: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct On {
    on: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    id_v1: Option<String>,
    alert: Value,
    color: LightColor,
    color_temperature: ColorTemperature,
    color_temperature_delta: Delta,
    dimming: Dimming,
    dimming_delta: Delta,
    dynamics: Value,
    effects: Value,
    identify: Value,
    metadata: Metadata,
    mode: String,
    on: On,
    owner: ResourceLink,
    powerup: Value,
    signaling: Value,
    /* powerup: { */
    /*     color: { */
    /*         color_temperature: { */
    /*             mirek: 366 */
    /*         }, */
    /*         mode: color_temperature */
    /*     }, */
    /*     configured: true, */
    /*     dimming: { */
    /*         dimming: { */
    /*             brightness: 100 */
    /*         }, */
    /*         mode: dimming */
    /*     }, */
    /*     on: { */
    /*         mode: on, */
    /*         on: { */
    /*             on: true */
    /*         } */
    /*     }, */
    /*     preset: safety */
    /* }, */
    /* signaling: { */
    /*     signal_values: [ */
    /*         no_signal, */
    /*         on_off, */
    /*         on_off_color, */
    /*         alternating */
    /*     ] */
    /* }, */
}

impl Light {
    #[must_use]
    pub fn new(id: u32, owner: ResourceLink) -> Self {
        Self {
            id_v1: Some(format!("/lights/{id}")),
            alert: json!({"action_values": ["breathe"]}),
            color: LightColor {
                gamut: Some(ColorGamut {
                    red: XY {
                        x: 0.6915,
                        y: 0.3083,
                    },
                    green: XY { x: 0.17, y: 0.7 },
                    blue: XY {
                        x: 0.1532,
                        y: 0.0475,
                    },
                }),
                gamut_type: Some("C".to_string()),
                xy: XY { x: 0.4573, y: 0.41 },
            },
            color_temperature: ColorTemperature {
                mirek_schema: MirekSchema {
                    mirek_maximum: 500,
                    mirek_minimum: 153,
                },
                mirek_valid: true,
                mirek: 366,
            },
            color_temperature_delta: Delta {},
            dimming: Dimming {
                brightness: 100.0,
                min_dim_level: 0.2,
            },
            dimming_delta: Delta {},
            dynamics: json!({
                "speed": 0,
                "speed_valid": false,
                "status": "none",
                "status_values": [
                    "none",
                    "dynamic_palette",
                ]
            }),
            effects: json!({
                "effect_values": [
                    "no_effect",
                    "candle",
                    "fire",
                    "prism"
                ],
                "status": "no_effect",
                "status_values": [
                    "no_effect",
                    "candle",
                    "fire",
                    "prism"
                ]
            }),
            identify: json!({}),
            mode: "normal".to_string(),
            on: On { on: true },
            metadata: Metadata {
                archetype: "spot_bulb".to_string(),
                name: "Light 1".to_string(),
            },
            owner,
            powerup: json!({
                "color": {
                    "color_temperature": {
                        "mirek": 366
                    },
                    "mode": "color_temperature"
                },
                "configured": true,
                "dimming": {
                    "dimming": {
                        "brightness": 100
                    },
                    "mode": "dimming"
                },
                "on": {
                    "mode": "on",
                    "on": {
                        "on": true
                    }
                },
                "preset": "safety"
            }),
            signaling: json!({
                "signal_values": [
                    "no_signal",
                    "on_off",
                    "on_off_color",
                    "alternating"
                ]
            }),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Matter {
    has_qr_code: bool,
    max_fabrics: u32,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RoomArchetypes {
    LivingRoom,
    Kitchen,
    Dining,
    Bedroom,
    KidsBedroom,
    Bathroom,
    Nursery,
    Recreation,
    Office,
    Gym,
    Hallway,
    Toilet,
    FrontDoor,
    Garage,
    Terrace,
    Garden,
    Driveway,
    Carport,
    Home,
    Downstairs,
    Upstairs,
    TopFloor,
    Attic,
    GuestRoom,
    Staircase,
    Lounge,
    ManCave,
    Computer,
    Studio,
    Music,
    Tv,
    Reading,
    Closet,
    Storage,
    LaundryRoom,
    Balcony,
    Porch,
    Barbecue,
    Pool,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub children: Vec<ResourceLink>,
    pub id_v1: Option<String>,
    pub metadata: Metadata,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneAction {
    color: Option<LightColor>,
    color_temperature: Option<ColorTemperatureUpdate>,
    dimming: DimmingUpdate,
    on: On,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneActionElement {
    action: SceneAction,
    target: ResourceLink,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    appdata: Option<String>,
    image: Option<ResourceLink>,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneStatus {
    active: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    actions: Vec<SceneActionElement>,
    #[serde(default)]
    auto_dynamic: bool,
    group: ResourceLink,
    id_v1: Option<String>,
    metadata: SceneMetadata,
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
    palette: Value,
    recall: Option<Value>,
    speed: f64,
    status: Option<SceneStatus>,
}

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
    id_v1: Option<String>,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    BehaviorScript(BehaviorScript),
    Bridge(Bridge),
    BridgeHome(BridgeHome),
    Device(Device),
    Entertainment(Entertainment),
    GeofenceClient(GeofenceClient),
    Geolocation(Geolocation),
    GroupedLight(GroupedLight),
    Homekit(Homekit),
    Light(Light),
    Matter(Matter),
    Room(Room),
    Scene(Scene),
    SmartScene(SmartScene),
    ZigbeeConnectivity(ZigbeeConnectivity),
    ZigbeeDeviceDiscovery(ZigbeeDeviceDiscovery),
}

impl Resource {
    #[must_use]
    pub const fn rtype(&self) -> ResourceType {
        match self {
            Self::BehaviorScript(_) => ResourceType::BehaviorScript,
            Self::Bridge(_) => ResourceType::Bridge,
            Self::BridgeHome(_) => ResourceType::BridgeHome,
            Self::Device(_) => ResourceType::Device,
            Self::Entertainment(_) => ResourceType::Entertainment,
            Self::GeofenceClient(_) => ResourceType::GeofenceClient,
            Self::Geolocation(_) => ResourceType::Geolocation,
            Self::GroupedLight(_) => ResourceType::GroupedLight,
            Self::Homekit(_) => ResourceType::Homekit,
            Self::Light(_) => ResourceType::Light,
            Self::Matter(_) => ResourceType::Matter,
            Self::Room(_) => ResourceType::Room,
            Self::Scene(_) => ResourceType::Scene,
            Self::SmartScene(_) => ResourceType::SmartScene,
            Self::ZigbeeConnectivity(_) => ResourceType::ZigbeeConnectivity,
            Self::ZigbeeDeviceDiscovery(_) => ResourceType::ZigbeeDeviceDiscovery,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceRecord {
    id: Uuid,
    #[serde(flatten)]
    pub obj: Resource,
}

impl ResourceRecord {
    #[must_use]
    pub fn from_ref((id, obj): (&Uuid, &Resource)) -> Self {
        Self {
            id: *id,
            obj: obj.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Reply<T> {
    pub data: Vec<T>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceLink {
    pub rid: Uuid,
    pub rtype: ResourceType,
}

impl ResourceLink {
    #[must_use]
    pub const fn new(rid: Uuid, rtype: ResourceType) -> Self {
        Self { rid, rtype }
    }

    #[must_use]
    pub const fn to(rid: Uuid, res: &Resource) -> Self {
        Self {
            rid,
            rtype: res.rtype(),
        }
    }
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

impl DeviceProductData {
    #[must_use]
    pub fn hue_color_spot() -> Self {
        Self {
            model_id: "LCG002".to_string(),
            manufacturer_name: "Signify Netherlands B.V.".to_string(),
            product_name: "Hue color spot".to_string(),
            product_archetype: "spot_bulb".to_string(),
            certified: true,
            software_version: "1.104.2".to_string(),
        }
    }

    #[must_use]
    pub fn hue_bridge_v2() -> Self {
        Self {
            certified: true,
            manufacturer_name: "Signify Netherlands B.V.".to_string(),
            model_id: "BSB002".to_string(),
            product_archetype: "bridge_v2".to_string(),
            product_name: "Hue Bridge".to_string(),
            software_version: "1.60.1960149090".to_string(),
        }
    }
}
