use std::{collections::HashMap, net::Ipv4Addr};

use chrono::{DateTime, Local, NaiveDateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::api;
use crate::api::{ColorGamut, DeviceProductData};
use crate::date_format;
use crate::hs::RawHS;

#[cfg(feature = "mac")]
use crate::version::SwVersion;
#[cfg(feature = "mac")]
use mac_address::MacAddress;

#[derive(Debug, Serialize, Deserialize)]
pub struct HueError {
    #[serde(rename = "type")]
    typ: u32,
    address: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HueApiResult<T> {
    Success(T),
    Error(HueError),
}

#[cfg(feature = "mac")]
pub fn serialize_lower_case_mac<S>(mac: &MacAddress, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let m = mac.bytes();
    let addr = format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        m[0], m[1], m[2], m[3], m[4], m[5]
    );
    serializer.serialize_str(&addr)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiShortConfig {
    pub apiversion: String,
    pub bridgeid: String,
    pub datastoreversion: String,
    pub factorynew: bool,
    #[cfg(feature = "mac")]
    #[serde(serialize_with = "serialize_lower_case_mac")]
    pub mac: MacAddress,
    #[cfg(not(feature = "mac"))]
    pub mac: String,
    pub modelid: String,
    pub name: String,
    pub replacesbridgeid: Option<String>,
    pub starterkitid: String,
    pub swversion: String,
}

impl Default for ApiShortConfig {
    #[allow(clippy::default_trait_access)]
    fn default() -> Self {
        Self {
            apiversion: crate::HUE_BRIDGE_V2_DEFAULT_APIVERSION.to_string(),
            bridgeid: "0000000000000000".to_string(),
            datastoreversion: "176".to_string(),
            factorynew: false,
            mac: Default::default(),
            modelid: crate::HUE_BRIDGE_V2_MODEL_ID.to_string(),
            name: "Bifrost Bridge".to_string(),
            replacesbridgeid: None,
            starterkitid: String::new(),
            swversion: crate::HUE_BRIDGE_V2_DEFAULT_SWVERSION.to_string(),
        }
    }
}

#[cfg(feature = "mac")]
impl ApiShortConfig {
    #[must_use]
    pub fn from_mac_and_version(mac: MacAddress, version: &SwVersion) -> Self {
        Self {
            bridgeid: crate::bridge_id(mac).to_uppercase(),
            apiversion: version.get_legacy_apiversion(),
            swversion: version.get_legacy_swversion(),
            mac,
            ..Self::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiResourceType {
    Config,
    Groups,
    Lights,
    Resourcelinks,
    Rules,
    Scenes,
    Schedules,
    Sensors,
    Capabilities,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub devicetype: String,
    #[serde(default)]
    pub generateclientkey: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUserReply {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clientkey: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected,
    #[default]
    Disconnected,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiInternetServices {
    pub internet: ConnectionState,
    pub remoteaccess: ConnectionState,
    pub swupdate: ConnectionState,
    pub time: ConnectionState,
}

impl Default for ApiInternetServices {
    fn default() -> Self {
        Self {
            internet: ConnectionState::Connected,
            remoteaccess: ConnectionState::Connected,
            swupdate: ConnectionState::Connected,
            time: ConnectionState::Connected,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PortalState {
    communication: ConnectionState,
    incoming: bool,
    outgoing: bool,
    signedon: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiBackup {
    pub errorcode: u32,
    pub status: String,
}

impl Default for ApiBackup {
    fn default() -> Self {
        Self {
            errorcode: 0,
            status: "idle".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwUpdate {
    #[serde(with = "date_format::legacy_utc")]
    lastinstall: DateTime<Utc>,
    state: SwUpdateState,
}

impl Default for SwUpdate {
    fn default() -> Self {
        Self {
            lastinstall: Utc::now(),
            state: SwUpdateState::NoUpdates,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwUpdateState {
    NoUpdates,
    Transferring,
    ReadyToInstall,
    AnyReadyToInstall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoftwareUpdate2 {
    autoinstall: Value,
    bridge: SwUpdate,
    checkforupdate: bool,
    #[serde(with = "date_format::legacy_utc")]
    lastchange: DateTime<Utc>,
    state: SwUpdateState,
}

impl SoftwareUpdate2 {
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            autoinstall: json!({ "on": true, "updatetime": "T14:00:00" }),
            bridge: SwUpdate {
                lastinstall: Utc::now(),
                state: SwUpdateState::NoUpdates,
            },
            checkforupdate: false,
            lastchange: Utc::now(),
            state: SwUpdateState::NoUpdates,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Whitelist {
    #[serde(with = "date_format::legacy_utc", rename = "create date")]
    pub create_date: DateTime<Utc>,
    #[serde(with = "date_format::legacy_utc", rename = "last use date")]
    pub last_use_date: DateTime<Utc>,
    pub name: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfig {
    pub analyticsconsent: bool,
    pub backup: ApiBackup,
    #[serde(flatten)]
    pub short_config: ApiShortConfig,
    pub dhcp: bool,
    pub internetservices: ApiInternetServices,
    pub linkbutton: bool,
    pub portalconnection: ConnectionState,
    pub portalservices: bool,
    pub portalstate: PortalState,
    pub proxyaddress: String,
    pub proxyport: u16,
    pub swupdate2: SoftwareUpdate2,
    pub zigbeechannel: u8,
    pub ipaddress: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub gateway: Ipv4Addr,
    pub timezone: Tz,
    #[serde(with = "date_format::legacy_utc", rename = "UTC")]
    pub utc: DateTime<Utc>,
    #[serde(with = "date_format::legacy_naive")]
    pub localtime: NaiveDateTime,
    pub whitelist: HashMap<String, Whitelist>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApiConfigUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<Tz>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiEffect {
    #[default]
    None,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiAlert {
    #[default]
    None,
    Select,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApiGroupAction {
    pub on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hue: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sat: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<ApiEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xy: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ct: Option<u16>,
    pub alert: ApiAlert,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colormode: Option<LightColorMode>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum ApiGroupType {
    Entertainment,
    #[default]
    LightGroup,
    Room,
    Zone,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum ApiGroupClass {
    #[serde(rename = "Living room")]
    LivingRoom,
    Kitchen,
    Dining,
    Bedroom,
    #[serde(rename = "Kids bedroom")]
    KidsBedroom,
    Bathroom,
    Nursery,
    Recreation,
    Office,
    Gym,
    Hallway,
    Toilet,
    #[serde(rename = "Front door")]
    FrontDoor,
    Garage,
    Terrace,
    Garden,
    Driveway,
    Carport,
    #[default]
    Other,

    Home,
    Downstairs,
    Upstairs,
    #[serde(rename = "Top floor")]
    TopFloor,
    Attic,
    #[serde(rename = "Guest room")]
    GuestRoom,
    Staircase,
    Lounge,
    #[serde(rename = "Man cave")]
    ManCave,
    Computer,
    Studio,
    Music,
    TV,
    Reading,
    Closet,
    Storage,
    #[serde(rename = "Laundry room")]
    LaundryRoom,
    Balcony,
    Porch,
    Barbecue,
    Pool,
    Free,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroup {
    pub name: String,
    pub lights: Vec<String>,
    pub action: ApiGroupAction,

    #[serde(rename = "type")]
    pub group_type: ApiGroupType,
    pub class: ApiGroupClass,
    pub recycle: bool,
    pub sensors: Vec<Value>,
    pub state: ApiGroupState,
    #[serde(skip_serializing_if = "Value::is_null", default)]
    pub stream: Value,
    #[serde(skip_serializing_if = "Value::is_null", default)]
    pub locations: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroupNew {
    pub name: Option<String>,
    #[serde(default, rename = "type")]
    pub group_type: ApiGroupType,
    #[serde(default)]
    pub class: ApiGroupClass,
    pub lights: Vec<String>,
}

impl ApiGroup {
    #[must_use]
    pub fn make_group_0() -> Self {
        Self {
            name: "Group 0".into(),
            lights: vec![],
            action: ApiGroupAction::default(),
            group_type: ApiGroupType::LightGroup,
            class: ApiGroupClass::default(),
            recycle: false,
            sensors: vec![],
            state: ApiGroupState::default(),
            stream: Value::Null,
            locations: Value::Null,
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn from_lights_and_room(
        glight: &api::GroupedLight,
        lights: Vec<String>,
        room: api::Room,
    ) -> Self {
        Self {
            name: room.metadata.name,
            lights,
            action: ApiGroupAction {
                on: glight.on.is_some_and(|on| on.on),
                bri: glight.dimming.map(|dim| (dim.brightness * 2.54) as u32),
                hue: None,
                sat: None,
                effect: None,
                xy: None,
                ct: None,
                alert: ApiAlert::None,
                colormode: None,
            },
            class: ApiGroupClass::default(),
            group_type: ApiGroupType::Room,
            recycle: false,
            sensors: vec![],
            state: ApiGroupState::default(),
            stream: Value::Null,
            locations: Value::Null,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApiGroupState {
    pub all_on: bool,
    pub any_on: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightColorMode {
    Ct,
    Xy,
    Hs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLightState {
    on: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    bri: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hue: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sat: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    xy: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ct: Option<u16>,
    alert: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    colormode: Option<LightColorMode>,
    mode: String,
    reachable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLightStateUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xy: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ct: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    pub hs: Option<RawHS>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitiontime: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroupUpdate {
    pub scene: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Active {
    pub active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroupUpdate2 {
    pub lights: Option<Vec<String>>,
    pub name: Option<String>,
    pub stream: Option<Active>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiGroupActionUpdate {
    GroupUpdate(ApiGroupUpdate),
    LightUpdate(ApiLightStateUpdate),
}

impl From<api::SceneAction> for ApiLightStateUpdate {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from(action: api::SceneAction) -> Self {
        Self {
            on: action.on.map(|on| on.on),
            bri: action.dimming.map(|dim| (dim.brightness * 2.54) as u8),
            xy: action.color.map(|col| col.xy.into()),
            ct: action.color_temperature.and_then(|ct| ct.mirek),
            hs: None,
            transitiontime: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLight {
    state: ApiLightState,
    swupdate: SwUpdate,
    #[serde(rename = "type")]
    light_type: String,
    name: String,
    modelid: String,
    manufacturername: String,
    productname: String,
    capabilities: Value,
    config: Value,
    uniqueid: String,
    swversion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    swconfigid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    productid: Option<String>,
}

impl ApiLight {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn from_dev_and_light(uuid: &Uuid, dev: &api::Device, light: &api::Light) -> Self {
        let colormode = if light.color.is_some() {
            LightColorMode::Xy
        } else {
            LightColorMode::Ct
        };

        let product_data = dev.product_data.clone();

        Self {
            state: ApiLightState {
                on: light.on.on,
                bri: light
                    .dimming
                    .map(|dim| ((dim.brightness * 2.54) as u32).max(1)),
                hue: None,
                sat: None,
                effect: Some("none".into()),
                xy: light.color.clone().map(|col| col.xy.into()),
                ct: light.color_temperature.clone().and_then(|ct| ct.mirek),
                alert: "select".into(),
                colormode: Some(colormode),
                mode: "homeautomation".to_string(),
                reachable: true,
            },
            swupdate: SwUpdate::default(),
            name: light.metadata.name.clone(),
            modelid: product_data.model_id,
            manufacturername: product_data.manufacturer_name,
            productname: product_data.product_name,
            productid: product_data.hardware_platform_type,

            capabilities: json!({
                "certified": true,
                "control": {
                    "colorgamut": [
                        [ColorGamut::GAMUT_C.red.x,   ColorGamut::GAMUT_C.red.y  ],
                        [ColorGamut::GAMUT_C.green.x, ColorGamut::GAMUT_C.green.y],
                        [ColorGamut::GAMUT_C.blue.x,  ColorGamut::GAMUT_C.blue.y ],
                    ],
                    "colorgamuttype": "C",
                    "ct": {
                        "max": 500,
                        "min": 153
                    },
                    "maxlumen": 800,
                    "mindimlevel": 10
                },
                "streaming": {
                    "proxy": true,
                    "renderer": true
                }
            }),
            config: json!({
                "archetype": "spotbulb",
                "function": "mixed",
                "direction": "downwards",
                "startup": {
                    "mode": "safety",
                    "configured": true
                }
            }),
            light_type: "Extended color light".to_string(),

            /* FIXME: Should have form "00:11:22:33:44:55:66:77-0b" */
            uniqueid: uuid.as_simple().to_string(),

            swversion: product_data.software_version,

            /* FIXME: Should have form "9012C6FD" */
            swconfigid: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResourceLink {
    #[serde(rename = "type")]
    pub link_type: String,
    pub name: String,
    pub description: String,
    pub classid: u32,
    pub owner: Uuid,
    pub recycle: bool,
    pub links: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRule {
    pub name: String,
    pub recycle: bool,
    pub status: String,
    pub conditions: Vec<Value>,
    pub actions: Vec<Value>,
    pub owner: Uuid,
    pub timestriggered: u32,
    #[serde(with = "date_format::legacy_utc")]
    pub created: DateTime<Utc>,
    pub lasttriggered: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ApiSceneType {
    LightScene,
    GroupScene,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ApiSceneVersion {
    V2 = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSceneAppData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiScene {
    pub name: String,
    #[serde(rename = "type")]
    pub scene_type: ApiSceneType,
    pub lights: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub lightstates: HashMap<String, ApiLightStateUpdate>,
    pub owner: String,
    pub recycle: bool,
    pub locked: bool,
    pub appdata: ApiSceneAppData,
    pub picture: String,
    #[serde(with = "date_format::legacy_utc")]
    pub lastupdated: DateTime<Utc>,
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSchedule {
    pub recycle: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autodelete: Option<bool>,
    pub description: String,
    pub command: Value,
    #[serde(with = "date_format::legacy_utc")]
    pub created: DateTime<Utc>,
    #[serde(
        with = "date_format::legacy_utc_opt",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub starttime: Option<DateTime<Utc>>,
    pub time: String,
    pub localtime: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSensor {
    #[serde(rename = "type")]
    pub sensor_type: String,
    pub config: Value,
    pub name: String,
    pub state: Value,
    pub manufacturername: String,
    pub modelid: String,
    pub swversion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swupdate: Option<SwUpdate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniqueid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diversityid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub productname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycle: Option<bool>,
    #[serde(skip_serializing_if = "Value::is_null", default)]
    pub capabilities: Value,
}

impl ApiSensor {
    #[must_use]
    pub fn builtin_daylight_sensor() -> Self {
        Self {
            config: json!({
                "configured": false,
                "on": true,
                "sunriseoffset": 30,
                "sunsetoffset": -30
            }),
            manufacturername: DeviceProductData::SIGNIFY_MANUFACTURER_NAME.to_string(),
            modelid: "PHDL00".to_string(),
            name: "Daylight".to_string(),
            state: json!({
                "daylight": Value::Null,
                "lastupdated": "none",
            }),
            swversion: "1.0".to_string(),
            sensor_type: "Daylight".to_string(),
            swupdate: None,
            uniqueid: None,
            diversityid: None,
            productname: None,
            recycle: None,
            capabilities: Value::Null,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUserConfig {
    pub config: ApiConfig,
    pub groups: HashMap<String, ApiGroup>,
    pub lights: HashMap<String, ApiLight>,
    pub resourcelinks: HashMap<u32, ApiResourceLink>,
    pub rules: HashMap<u32, ApiRule>,
    pub scenes: HashMap<String, ApiScene>,
    pub schedules: HashMap<u32, ApiSchedule>,
    pub sensors: HashMap<u32, ApiSensor>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            analyticsconsent: false,
            backup: ApiBackup::default(),
            short_config: ApiShortConfig::default(),
            dhcp: true,
            internetservices: ApiInternetServices::default(),
            linkbutton: Default::default(),
            portalconnection: ConnectionState::Disconnected,
            portalservices: true,
            portalstate: PortalState::default(),
            proxyaddress: "none".to_string(),
            proxyport: Default::default(),
            swupdate2: SoftwareUpdate2::new(),
            zigbeechannel: 25,
            ipaddress: Ipv4Addr::UNSPECIFIED,
            netmask: Ipv4Addr::UNSPECIFIED,
            gateway: Ipv4Addr::UNSPECIFIED,
            timezone: Tz::UTC,
            utc: Utc::now(),
            localtime: Local::now().naive_local(),
            whitelist: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Capacity {
    pub available: u32,
    pub total: u32,
}

impl Capacity {
    #[must_use]
    pub const fn new(total: u32, available: u32) -> Self {
        Self { available, total }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SensorsCapacity {
    pub available: u32,
    pub total: u32,
    pub clip: Capacity,
    pub zll: Capacity,
    pub zgp: Capacity,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ScenesCapacity {
    pub available: u32,
    pub total: u32,
    pub lightstates: Capacity,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RulesCapacity {
    pub available: u32,
    pub total: u32,
    pub conditions: Capacity,
    pub actions: Capacity,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SceneCapacity {
    #[serde(flatten)]
    pub scenes: Capacity,
    pub lightstates: Capacity,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StreamingCapacity {
    pub available: u32,
    pub total: u32,
    pub channels: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Capabilities {
    pub lights: Capacity,
    pub sensors: SensorsCapacity,
    pub groups: Capacity,
    pub scenes: SceneCapacity,
    pub schedules: Capacity,
    pub rules: RulesCapacity,
    pub resourcelinks: Capacity,
    pub streaming: StreamingCapacity,
    pub timezones: Value,
}

impl Capabilities {
    #[must_use]
    pub fn new() -> Self {
        Self {
            lights: Capacity::new(63, 62),
            sensors: SensorsCapacity {
                available: 249,
                total: 250,
                clip: Capacity::new(250, 249),
                zll: Capacity::new(64, 64),
                zgp: Capacity::new(64, 64),
            },
            groups: Capacity::new(64, 60),
            scenes: SceneCapacity {
                scenes: Capacity::new(200, 175),
                lightstates: Capacity::new(12600, 11025),
            },
            schedules: Capacity::new(100, 100),
            rules: RulesCapacity {
                available: 250,
                total: 250,
                conditions: Capacity::new(1500, 1500),
                actions: Capacity::new(1000, 1000),
            },
            resourcelinks: Capacity::new(64, 64),
            streaming: StreamingCapacity {
                available: 1,
                total: 1,
                channels: 20,
            },
            timezones: json!({
                "values":
                 chrono_tz::TZ_VARIANTS.iter().collect::<Vec<_>>(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "mac")]
    #[test]
    fn serialize_lower_case_mac() {
        use mac_address::MacAddress;

        use crate::legacy_api::serialize_lower_case_mac;

        let mac = MacAddress::new([0x01, 0x02, 0x03, 0xAA, 0xBB, 0xCC]);
        let mut res = vec![];
        let mut ser = serde_json::Serializer::new(&mut res);

        serialize_lower_case_mac(&mac, &mut ser).unwrap();

        assert_eq!(res, b"\"01:02:03:aa:bb:cc\"");
    }
}
