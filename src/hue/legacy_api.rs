use std::{collections::HashMap, net::Ipv4Addr};

use chrono::{DateTime, Local, Utc};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::hue::{api, best_guess_timezone};

use super::date_format;

#[derive(Debug, Serialize, Deserialize)]
pub struct HueError {
    #[serde(rename = "type")]
    typ: u32,
    address: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HueResult<T> {
    Success(T),
    Error(HueError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiShortConfig {
    pub apiversion: String,
    pub bridgeid: String,
    pub datastoreversion: String,
    pub factorynew: bool,
    pub mac: MacAddress,
    pub modelid: String,
    pub name: String,
    pub replacesbridgeid: Option<String>,
    pub starterkitid: String,
    pub swversion: String,
}

impl Default for ApiShortConfig {
    fn default() -> Self {
        Self {
            apiversion: "1.65.0".to_string(),
            bridgeid: "0000000000000000".to_string(),
            datastoreversion: "163".to_string(),
            factorynew: false,
            mac: MacAddress::default(),
            modelid: crate::hue::HUE_BRIDGE_V2_MODEL_ID.to_string(),
            name: "Bifrost Bridge".to_string(),
            replacesbridgeid: None,
            starterkitid: String::new(),
            swversion: "1965111030".to_string(),
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
    devicetype: String,
    generateclientkey: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUserReply {
    pub username: Uuid,
    pub clientkey: Uuid,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected,
    Disconnected,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
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

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DeviceTypes {
    bridge: bool,
    lights: Vec<Value>,
    sensors: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwUpdate {
    #[serde(with = "date_format::utc")]
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoftwareUpdate2 {
    autoinstall: Value,
    bridge: SwUpdate,
    checkforupdate: bool,
    #[serde(with = "date_format::utc")]
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
    #[serde(with = "date_format::utc")]
    pub create_date: DateTime<Utc>,
    #[serde(with = "date_format::utc")]
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
    pub timezone: String,
    #[serde(with = "date_format::utc", rename = "UTC")]
    pub utc: DateTime<Utc>,
    #[serde(with = "date_format::local")]
    pub localtime: DateTime<Local>,
    pub whitelist: HashMap<Uuid, Whitelist>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiEffect {
    None,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiAlert {
    None,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroupAction {
    on: bool,
    bri: u32,
    hue: u32,
    sat: u32,
    effect: ApiEffect,
    xy: [f64; 2],
    ct: u32,
    alert: ApiAlert,
    colormode: LightColorMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ApiGroupType {
    Room,
    LightGroup,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroup {
    name: String,
    lights: Vec<String>,
    action: ApiGroupAction,

    #[serde(rename = "type")]
    group_type: ApiGroupType,
    class: String,
}

impl ApiGroup {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn from_lights_and_room(
        glight: api::GroupedLight,
        lights: &[(Uuid, api::Light)],
        room: api::Room,
    ) -> Self {
        Self {
            name: room.metadata.name,
            lights: lights.iter().map(|l| format!("{}", l.0.simple())).collect(),
            action: ApiGroupAction {
                on: glight.on.is_some_and(|on| on.on),
                bri: glight
                    .dimming
                    .map(|dim| (dim.brightness * 2.54) as u32)
                    .unwrap_or_default(),
                hue: 0,
                sat: 0,
                effect: ApiEffect::None,
                xy: [0.0, 0.0],
                ct: 0,
                alert: ApiAlert::None,
                colormode: LightColorMode::Xy,
            },
            class: "Bedroom".to_string(),
            group_type: ApiGroupType::Room,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroupState {
    pub all_on: bool,
    pub any_on: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightColorMode {
    Ct,
    Xy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLightState {
    on: bool,
    bri: u32,
    hue: u32,
    sat: u32,
    effect: String,
    xy: [f64; 2],
    ct: u32,
    alert: String,
    colormode: LightColorMode,
    mode: String,
    reachable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLightStateUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xy: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ct: Option<u32>,
}

impl From<api::SceneAction> for ApiLightStateUpdate {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn from(action: api::SceneAction) -> Self {
        Self {
            on: action.on.map(|on| on.on),
            bri: action.dimming.map(|dim| (dim.brightness * 2.54) as u32),
            xy: action.color.map(|col| col.xy.into()),
            ct: action.color_temperature.map(|ct| ct.mirek),
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
    swconfigid: String,
    productid: String,
}

impl ApiLight {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    #[must_use]
    pub fn from_dev_and_light(uuid: &Uuid, dev: api::Device, light: api::Light) -> Self {
        let colormode = if light.color.is_some() {
            LightColorMode::Xy
        } else {
            LightColorMode::Ct
        };

        Self {
            state: ApiLightState {
                on: light.on.on,
                bri: light
                    .dimming
                    .map(|dim| (dim.brightness * 2.54) as u32)
                    .unwrap_or_default(),
                hue: 0,
                sat: 0,
                effect: String::new(),
                xy: light.color.map(|col| col.xy.into()).unwrap_or_default(),
                ct: light
                    .color_temperature
                    .and_then(|ct| ct.mirek)
                    .unwrap_or_default(),
                alert: String::new(),
                colormode,
                mode: "homeautomation".to_string(),
                reachable: true,
            },
            swupdate: SwUpdate::default(),
            name: light.metadata.name,
            modelid: dev.product_data.product_name,
            manufacturername: dev.product_data.manufacturer_name,
            productname: "Hue color spot".to_string(),
            productid: dev.product_data.model_id,
            capabilities: json!({
                "certified": true,
                "control": {
                    "colorgamut": [
                        [0.6915, 0.3083 ],
                        [0.17,   0.7    ],
                        [0.1532, 0.0475 ],
                    ],
                    "colorgamuttype": "C",
                    "ct": {
                        "max": 500,
                        "min": 153
                    },
                    "maxlumen": 300,
                    "mindimlevel": 200
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
            uniqueid: uuid.as_simple().to_string(),
            swversion: dev.product_data.software_version,
            swconfigid: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResourceLink {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRule {}

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
pub struct ApiScene {
    name: String,
    #[serde(rename = "type")]
    scene_type: ApiSceneType,
    lights: Vec<String>,
    lightstates: HashMap<String, ApiLightStateUpdate>,
    owner: Uuid,
    recycle: bool,
    locked: bool,
    appdata: Value,
    picture: String,
    #[serde(with = "date_format::utc")]
    lastupdated: DateTime<Utc>,
    version: u32,
    image: Option<Uuid>,
    group: String,
}

impl ApiScene {
    #[must_use]
    pub fn from_scene(owner: Uuid, scene: api::Scene) -> Self {
        let lights = scene
            .actions
            .iter()
            .map(|sae| sae.target.rid.as_simple().to_string())
            .collect();

        let lightstates = scene
            .actions
            .iter()
            .map(|sae| {
                (
                    sae.target.rid.as_simple().to_string(),
                    ApiLightStateUpdate::from(sae.action.clone()),
                )
            })
            .collect();

        Self {
            name: scene.metadata.name,
            scene_type: ApiSceneType::GroupScene,
            lights,
            lightstates,
            owner,
            recycle: false,
            locked: false,
            appdata: json!({}),
            picture: String::new(),
            lastupdated: Utc::now(),
            version: ApiSceneVersion::V2 as u32,
            image: scene.metadata.image.map(|rl| rl.rid),
            group: scene.group.rid.as_simple().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSchedule {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSensor {}

#[allow(clippy::zero_sized_map_values)]
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
            portalservices: Default::default(),
            portalstate: PortalState::default(),
            proxyaddress: "none".to_string(),
            proxyport: Default::default(),
            swupdate2: SoftwareUpdate2::new(),
            zigbeechannel: 25,
            ipaddress: Ipv4Addr::UNSPECIFIED,
            netmask: Ipv4Addr::UNSPECIFIED,
            gateway: Ipv4Addr::UNSPECIFIED,
            timezone: best_guess_timezone(),
            utc: Utc::now(),
            localtime: Local::now(),
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
            lights: Capacity::new(63, 60),
            sensors: SensorsCapacity {
                available: 240,
                total: 250,
                clip: Capacity::new(250, 240),
                zll: Capacity::new(64, 63),
                zgp: Capacity::new(64, 63),
            },
            groups: Capacity::new(64, 60),
            schedules: Capacity::new(100, 95),
            rules: RulesCapacity {
                available: 233,
                total: 255,
                conditions: Capacity::new(1500, 1451),
                actions: Capacity::new(1000, 954),
            },
            resourcelinks: Capacity::new(64, 59),
            streaming: StreamingCapacity {
                available: 1,
                total: 1,
                channels: 20,
            },
            timezones: json!({
                "values": [
                    "CET",
                    "UTC",
                    "GMT",
                    "Europe/Copenhagen",
                ],
            }),
        }
    }
}
