use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
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
    Room,
    Scene,
    SmartScene,
    ZigbeeConnectivity,
    ZigbeeDeviceDiscovery,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DollarRef {
    #[serde(rename = "$ref")]
    dref: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Bridge {
    pub bridge_id: String,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeHome {
    children: Vec<ResourceLink>,
    id_v1: String,
    services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub product_data: DeviceProductData,
    pub metadata: Metadata,
    pub identify: Value,
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceProductData {
    pub model_id: String,
    pub manufacturer_name: String,
    pub product_name: String,
    pub product_archetype: String,
    pub certified: bool,
    pub software_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entertainment {
    equalizer: bool,
    id_v1: String,
    owner: ResourceLink,
    proxy: bool,
    renderer: bool,
    renderer_reference: ResourceLink,
    segments: EntertainmentSegments,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntertainmentSegments {
    configurable: bool,
    max_segments: u32,
    segments: Vec<EntertainmentSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntertainmentSegment {
    length: u32,
    start: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeofenceClient {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Geolocation {
    is_configured: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupedLight {
    alert: Value,
    color: Value,
    color_temperature: Value,
    color_temperature_delta: Value,
    dimming: Value,
    dimming_delta: Value,
    dynamics: Value,
    id_v1: String,
    on: On,
    owner: ResourceLink,
    signaling: Value,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct XY {
    x: f32,
    y: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColorGamut {
    red: XY,
    green: XY,
    blue: XY,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightColor {
    gamut: ColorGamut,
    gamut_type: String,
    xy: XY,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MirekSchema {
    mirek_minimum: u32,
    mirek_maximum: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColorTemperatureUpdate {
    mirek: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColorTemperature {
    mirek: u32,
    mirek_schema: MirekSchema,
    mirek_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Dimming {
    brightness: f64,
    min_dim_level: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DimmingUpdate {
    brightness: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delta {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    archetype: String,
}

impl Metadata {
    pub fn new(archetype: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            archetype: archetype.to_string(),
        }
    }

    pub fn room(archetype: RoomArchetypes, name: &str) -> Self {
        Self::new(json!(archetype).as_str().unwrap(), name)
    }

    pub fn hue_bridge(name: &str) -> Self {
        Self::new("bridge_v2", name)
    }

    pub fn spot_bulb(name: &str) -> Self {
        Self::new("spot_bulb", name)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct On {
    on: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Light {
    id: Uuid,
    id_v1: String,
    /* alert: { */
    /*     action_values: [ */
    /*         breathe */
    /*     ] */
    /* }, */
    alert: Value,
    color: LightColor,
    color_temperature: ColorTemperature,
    color_temperature_delta: Delta,
    dimming: Dimming,
    dimming_delta: Delta,
    /* dynamics: { */
    /*     speed: 0, */
    /*     speed_valid: false, */
    /*     status: none, */
    /*     status_values: [ */
    /*         none, */
    /*         dynamic_palette */
    /*     ] */
    /* }, */
    /* effects: { */
    /*     effect_values: [ */
    /*         no_effect, */
    /*         candle, */
    /*         fire, */
    /*         prism */
    /*     ], */
    /*     status: no_effect, */
    /*     status_values: [ */
    /*         no_effect, */
    /*         candle, */
    /*         fire, */
    /*         prism */
    /*     ] */
    /* }, */
    /* identify: {}, */
    metadata: LightMetadata,
    /* mode: normal, */
    /* on: { */
    /*     on: true */
    /* }, */
    owner: ResourceLink,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Matter {
    has_qr_code: bool,
    max_fabrics: u32,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub children: Vec<ResourceLink>,
    pub id_v1: String,
    pub metadata: Metadata,
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneAction {
    color_temperature: ColorTemperatureUpdate,
    dimming: DimmingUpdate,
    on: On,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneActionElement {
    action: SceneAction,
    target: ResourceLink,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    appdata: Option<String>,
    image: ResourceLink,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneStatus {
    active: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {
    actions: Vec<SceneActionElement>,
    auto_dynamic: bool,
    group: ResourceLink,
    id_v1: String,
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
    recall: Value,
    speed: f64,
    status: SceneStatus,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZigbeeConnectivityStatus {
    ConnectivityIssue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZigbeeConnectivity {
    id_v1: String,
    mac_address: String,
    owner: ResourceLink,
    status: ZigbeeConnectivityStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZigbeeDeviceDiscovery {
    owner: ResourceLink,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub fn rtype(&self) -> ResourceType {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLink {
    pub rid: Uuid,
    pub rtype: ResourceType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeZone {
    pub time_zone: String,
}
