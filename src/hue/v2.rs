use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use uuid::Uuid;

use crate::hue::best_guess_timezone;

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    BehaviorScript,
    BehaviorInstance,
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
    Zone,
}

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl ResourceType {
    #[must_use]
    pub fn link_to(self, rid: Uuid) -> ResourceLink {
        ResourceLink {
            rid: Uuid::from_u128_le(rid.to_u128_le() ^ (hash(&self) as u128)),
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
pub struct BehaviorInstance {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Bridge {
    pub bridge_id: String,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeHome {
    pub children: Vec<ResourceLink>,
    pub id_v1: Option<String>,
    pub services: Vec<ResourceLink>,
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
    pub alert: Value,
    pub color: LightColor,
    pub color_temperature: ColorTemperature,
    pub color_temperature_delta: Value,
    pub dimming: Dimming,
    pub dimming_delta: Value,
    pub dynamics: Value,
    pub id_v1: Option<String>,
    pub on: On,
    pub owner: ResourceLink,
    pub signaling: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupedLightUpdate {
    pub on: Option<On>,
    pub dimming: Option<Dimming>,
    pub color: Option<ColorUpdate>,
    pub color_temp: Option<f64>,
    pub color_temperature: Option<ColorTemperatureUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorUpdate {
    pub xy: XY,
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
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorGamut {
    red: XY,
    green: XY,
    blue: XY,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightColor {
    pub gamut: Option<ColorGamut>,
    pub gamut_type: Option<String>,
    pub xy: XY,
}

impl LightColor {
    #[must_use]
    pub fn dummy() -> Self {
        Self {
            gamut: Some(ColorGamut {
                red: XY {
                    x: 0.681_235,
                    y: 0.318_186,
                },
                green: XY {
                    x: 0.391_898,
                    y: 0.525_033,
                },
                blue: XY {
                    x: 0.150_241,
                    y: 0.027_116,
                },
            }),
            gamut_type: Some("Other".to_string()),
            xy: XY { x: 0.4573, y: 0.41 },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MirekSchema {
    mirek_minimum: u32,
    mirek_maximum: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperatureUpdate {
    pub mirek: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColorTemperature {
    pub mirek: u32,
    pub mirek_schema: MirekSchema,
    pub mirek_valid: bool,
}

impl ColorTemperature {
    #[must_use]
    pub const fn dummy() -> Self {
        Self {
            mirek_schema: MirekSchema {
                mirek_maximum: 454,
                mirek_minimum: 250,
            },
            mirek_valid: true,
            mirek: 366,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dimming {
    pub brightness: f64,
    pub min_dim_level: Option<f64>,
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
    pub on: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    pub id_v1: Option<String>,
    pub alert: Value,
    pub color: LightColor,
    pub color_temperature: ColorTemperature,
    pub color_temperature_delta: Delta,
    pub dimming: Dimming,
    pub dimming_delta: Delta,
    pub dynamics: Value,
    pub effects: Value,
    pub identify: Value,
    pub metadata: Metadata,
    pub mode: String,
    pub on: On,
    pub owner: ResourceLink,
    pub powerup: Value,
    pub signaling: Value,
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
            color: LightColor::dummy(),
            color_temperature: ColorTemperature::dummy(),
            color_temperature_delta: Delta {},
            dimming: Dimming {
                brightness: 100.0,
                min_dim_level: Some(0.2),
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
                    "color": {
                        "xy": XY { x: 0.4573, y: 0.41 },
                    },
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublicImage {}

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
pub struct Zone {
    id_v1: Option<String>,
    pub metadata: Metadata,
    pub children: Vec<ResourceLink>,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    BehaviorScript(BehaviorScript),
    BehaviorInstance(BehaviorInstance),
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
    PublicImage(PublicImage),
    Room(Room),
    Scene(Scene),
    SmartScene(SmartScene),
    ZigbeeConnectivity(ZigbeeConnectivity),
    ZigbeeDeviceDiscovery(ZigbeeDeviceDiscovery),
    Zone(Zone),
}

impl Resource {
    #[must_use]
    pub const fn rtype(&self) -> ResourceType {
        match self {
            Self::BehaviorScript(_) => ResourceType::BehaviorScript,
            Self::BehaviorInstance(_) => ResourceType::BehaviorInstance,
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
            Self::PublicImage(_) => ResourceType::PublicImage,
            Self::Room(_) => ResourceType::Room,
            Self::Scene(_) => ResourceType::Scene,
            Self::SmartScene(_) => ResourceType::SmartScene,
            Self::ZigbeeConnectivity(_) => ResourceType::ZigbeeConnectivity,
            Self::ZigbeeDeviceDiscovery(_) => ResourceType::ZigbeeDeviceDiscovery,
            Self::Zone(_) => ResourceType::Zone,
        }
    }

    pub fn from_value(rtype: ResourceType, obj: Value) -> Result<Self, serde_json::Error> {
        let res = match rtype {
            ResourceType::BehaviorScript => Self::BehaviorScript(from_value(obj)?),
            ResourceType::BehaviorInstance => Self::BehaviorInstance(from_value(obj)?),
            ResourceType::Bridge => Self::Bridge(from_value(obj)?),
            ResourceType::BridgeHome => Self::BridgeHome(from_value(obj)?),
            ResourceType::Device => Self::Device(from_value(obj)?),
            ResourceType::Entertainment => Self::Entertainment(from_value(obj)?),
            ResourceType::GeofenceClient => Self::GeofenceClient(from_value(obj)?),
            ResourceType::Geolocation => Self::Geolocation(from_value(obj)?),
            ResourceType::GroupedLight => Self::GroupedLight(from_value(obj)?),
            ResourceType::Homekit => Self::Homekit(from_value(obj)?),
            ResourceType::Light => Self::Light(from_value(obj)?),
            ResourceType::Matter => Self::Matter(from_value(obj)?),
            ResourceType::PublicImage => Self::PublicImage(from_value(obj)?),
            ResourceType::Room => Self::Room(from_value(obj)?),
            ResourceType::Scene => Self::Scene(from_value(obj)?),
            ResourceType::SmartScene => Self::SmartScene(from_value(obj)?),
            ResourceType::ZigbeeConnectivity => Self::ZigbeeConnectivity(from_value(obj)?),
            ResourceType::ZigbeeDeviceDiscovery => Self::ZigbeeDeviceDiscovery(from_value(obj)?),
            ResourceType::Zone => Self::Zone(from_value(obj)?),
        };
        Ok(res)
    }

    pub fn assign_id_v1(&mut self, index: u32) -> bool {
        match self {
            Self::BridgeHome(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::Entertainment(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::GroupedLight(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::Light(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::Room(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::Scene(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::ZigbeeConnectivity(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }
            Self::Zone(obj) => {
                obj.id_v1 = Some(format!("/foo/{index}"));
                true
            }

            Self::BehaviorScript(_)
            | Self::BehaviorInstance(_)
            | Self::Bridge(_)
            | Self::Device(_)
            | Self::GeofenceClient(_)
            | Self::Geolocation(_)
            | Self::Homekit(_)
            | Self::Matter(_)
            | Self::PublicImage(_)
            | Self::SmartScene(_)
            | Self::ZigbeeDeviceDiscovery(_) => false,
        }
    }

    pub fn get_id_v1(&mut self) -> &Option<String> {
        match self {
            Self::BridgeHome(obj) => &obj.id_v1,
            Self::Entertainment(obj) => &obj.id_v1,
            Self::GroupedLight(obj) => &obj.id_v1,
            Self::Light(obj) => &obj.id_v1,
            Self::Room(obj) => &obj.id_v1,
            Self::Scene(obj) => &obj.id_v1,
            Self::ZigbeeConnectivity(obj) => &obj.id_v1,
            Self::Zone(obj) => &obj.id_v1,

            Self::BehaviorScript(_)
            | Self::BehaviorInstance(_)
            | Self::Bridge(_)
            | Self::Device(_)
            | Self::GeofenceClient(_)
            | Self::Geolocation(_)
            | Self::Homekit(_)
            | Self::Matter(_)
            | Self::PublicImage(_)
            | Self::SmartScene(_)
            | Self::ZigbeeDeviceDiscovery(_) => &None,
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
    pub fn random(rtype: ResourceType) -> Self {
        Self {
            rid: Uuid::new_v4(),
            rtype,
        }
    }

    #[must_use]
    pub const fn to(rid: Uuid, res: &Resource) -> Self {
        Self {
            rid,
            rtype: res.rtype(),
        }
    }

    #[must_use]
    pub fn for_type(&self, rtype: ResourceType) -> Self {
        let rid = Uuid::from_u128_le(
            self.rid.to_u128_le() ^ (hash(&self.rtype) as u128) ^ (hash(&rtype) as u128),
        );
        Self { rid, rtype }
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
