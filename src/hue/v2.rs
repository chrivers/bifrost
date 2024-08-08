use std::fmt::{self, Debug};
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::{
    hue::{
        best_guess_timezone,
        update::{ColorTemperatureUpdate, DimmingUpdate},
    },
    types::XY,
    z2m::update::DeviceColorMode,
};

#[derive(Copy, Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RType {
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

fn hash<T: Hash + ?Sized>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

impl RType {
    #[must_use]
    pub const fn link_to(self, rid: Uuid) -> ResourceLink {
        ResourceLink { rid, rtype: self }
    }

    #[must_use]
    pub fn deterministic(self, data: impl Hash) -> ResourceLink {
        /* hash resource type (i.e., self) */
        let h1 = hash(&self);

        /* hash data */
        let h2 = hash(&data);

        /* use resulting bytes for uuid seed */
        let seed: &[u8] = &[h1.to_le_bytes(), h2.to_le_bytes()].concat();

        let rid = Uuid::new_v5(&Uuid::NAMESPACE_OID, seed);

        self.link_to(rid)
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
    pub services: Vec<ResourceLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub product_data: DeviceProductData,
    pub metadata: Metadata,
    pub identify: Value,
    pub services: Vec<ResourceLink>,
}

impl Device {
    #[must_use]
    pub fn light(&self) -> Option<&ResourceLink> {
        self.services.iter().find(|rl| rl.rtype == RType::Light)
    }
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
    pub on: On,
    pub owner: ResourceLink,
    pub signaling: Value,
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

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct On {
    pub on: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Light {
    /* This field does not exist in the hue api, but we need it to keep track of
     * last-used color mode for a light. */
    #[serde(skip, default)]
    pub color_mode: Option<DeviceColorMode>,

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
    pub fn new(owner: ResourceLink) -> Self {
        Self {
            alert: json!({"action_values": ["breathe"]}),
            color_mode: None,
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
    pub metadata: Metadata,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

impl Room {
    #[must_use]
    pub fn group(&self) -> Option<&ResourceLink> {
        self.services
            .iter()
            .find(|rl| rl.rtype == RType::GroupedLight)
    }
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
    pub appdata: Option<String>,
    pub image: Option<ResourceLink>,
    pub name: String,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
pub struct SceneStatus {
    pub active: SceneRecallAction,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecallAction {
    Inactive,
    Active,
    DynamicPalette,
    Static,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    pub actions: Vec<SceneActionElement>,
    #[serde(default)]
    pub auto_dynamic: bool,
    pub group: ResourceLink,
    pub metadata: SceneMetadata,
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
    pub palette: Value,
    pub speed: f64,
    pub status: Option<SceneStatus>,
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
    pub const fn rtype(&self) -> RType {
        match self {
            Self::BehaviorScript(_) => RType::BehaviorScript,
            Self::BehaviorInstance(_) => RType::BehaviorInstance,
            Self::Bridge(_) => RType::Bridge,
            Self::BridgeHome(_) => RType::BridgeHome,
            Self::Device(_) => RType::Device,
            Self::Entertainment(_) => RType::Entertainment,
            Self::GeofenceClient(_) => RType::GeofenceClient,
            Self::Geolocation(_) => RType::Geolocation,
            Self::GroupedLight(_) => RType::GroupedLight,
            Self::Homekit(_) => RType::Homekit,
            Self::Light(_) => RType::Light,
            Self::Matter(_) => RType::Matter,
            Self::PublicImage(_) => RType::PublicImage,
            Self::Room(_) => RType::Room,
            Self::Scene(_) => RType::Scene,
            Self::SmartScene(_) => RType::SmartScene,
            Self::ZigbeeConnectivity(_) => RType::ZigbeeConnectivity,
            Self::ZigbeeDeviceDiscovery(_) => RType::ZigbeeDeviceDiscovery,
            Self::Zone(_) => RType::Zone,
        }
    }

    pub fn from_value(rtype: RType, obj: Value) -> ApiResult<Self> {
        let res = match rtype {
            RType::BehaviorScript => Self::BehaviorScript(from_value(obj)?),
            RType::BehaviorInstance => Self::BehaviorInstance(from_value(obj)?),
            RType::Bridge => Self::Bridge(from_value(obj)?),
            RType::BridgeHome => Self::BridgeHome(from_value(obj)?),
            RType::Device => Self::Device(from_value(obj)?),
            RType::Entertainment => Self::Entertainment(from_value(obj)?),
            RType::GeofenceClient => Self::GeofenceClient(from_value(obj)?),
            RType::Geolocation => Self::Geolocation(from_value(obj)?),
            RType::GroupedLight => Self::GroupedLight(from_value(obj)?),
            RType::Homekit => Self::Homekit(from_value(obj)?),
            RType::Light => Self::Light(from_value(obj)?),
            RType::Matter => Self::Matter(from_value(obj)?),
            RType::PublicImage => Self::PublicImage(from_value(obj)?),
            RType::Room => Self::Room(from_value(obj)?),
            RType::Scene => Self::Scene(from_value(obj)?),
            RType::SmartScene => Self::SmartScene(from_value(obj)?),
            RType::ZigbeeConnectivity => Self::ZigbeeConnectivity(from_value(obj)?),
            RType::ZigbeeDeviceDiscovery => Self::ZigbeeDeviceDiscovery(from_value(obj)?),
            RType::Zone => Self::Zone(from_value(obj)?),
        };
        Ok(res)
    }
}

#[macro_export]
macro_rules! resource_conversion_impl {
    ( $name:ident ) => {
        impl<'a> TryFrom<&'a mut Resource> for &'a mut $name {
            type Error = ApiError;

            fn try_from(value: &'a mut Resource) -> Result<Self, Self::Error> {
                if let Resource::$name(obj) = value {
                    Ok(obj)
                } else {
                    Err(ApiError::WrongType(RType::Light, value.rtype()))
                }
            }
        }

        impl TryFrom<Resource> for $name {
            type Error = ApiError;

            fn try_from(value: Resource) -> Result<Self, Self::Error> {
                if let Resource::$name(obj) = value {
                    Ok(obj)
                } else {
                    Err(ApiError::WrongType(RType::Light, value.rtype()))
                }
            }
        }

        impl From<$name> for Resource {
            fn from(value: $name) -> Self {
                Resource::$name(value)
            }
        }
    };
}

resource_conversion_impl!(BehaviorScript);
resource_conversion_impl!(BehaviorInstance);
resource_conversion_impl!(Bridge);
resource_conversion_impl!(BridgeHome);
resource_conversion_impl!(Device);
resource_conversion_impl!(Entertainment);
resource_conversion_impl!(GeofenceClient);
resource_conversion_impl!(Geolocation);
resource_conversion_impl!(GroupedLight);
resource_conversion_impl!(Homekit);
resource_conversion_impl!(Light);
resource_conversion_impl!(Matter);
resource_conversion_impl!(PublicImage);
resource_conversion_impl!(Room);
resource_conversion_impl!(Scene);
resource_conversion_impl!(SmartScene);
resource_conversion_impl!(ZigbeeConnectivity);
resource_conversion_impl!(ZigbeeDeviceDiscovery);
resource_conversion_impl!(Zone);

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub id: Uuid,
    id_v1: String,
    #[serde(flatten)]
    pub obj: Resource,
}

impl ResourceRecord {
    #[must_use]
    pub fn from_ref((id, obj): (&Uuid, &Resource)) -> Self {
        Self {
            id: *id,
            id_v1: format!("/legacy/{}", id.as_simple()),
            obj: obj.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Reply<T> {
    pub data: Vec<T>,
    pub errors: Vec<String>,
}

#[derive(Copy, Hash, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ResourceLink {
    pub rid: Uuid,
    pub rtype: RType,
}

impl ResourceLink {
    #[must_use]
    pub const fn new(rid: Uuid, rtype: RType) -> Self {
        Self { rid, rtype }
    }
}

impl Debug for ResourceLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rtype = format!("{:?}", self.rtype).to_lowercase();
        let rid = self.rid;
        write!(f, "{rtype}/{rid}")
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
