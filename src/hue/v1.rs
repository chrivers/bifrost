use std::{collections::HashMap, net::Ipv4Addr};

use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

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
            starterkitid: "".to_string(),
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    devicetype: String,
    generateclientkey: bool,
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
pub struct SoftwareUpdate2 {
    autoinstall: Value,
    bridge: Value,
    checkforupdate: bool,
    lastchange: String,
    state: String,
}

impl SoftwareUpdate2 {
    pub fn new() -> Self {
        Self {
            autoinstall: json!({ "on": true }),
            bridge: json!({
                "lastinstall": "2020-01-01T01:01:01",
                "state": "noupdates",
            }),
            checkforupdate: false,
            lastchange: "2020-01-01T01:01:01".to_string(),
            state: "noupdates".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Whitelist {
    pub create_date: String,
    pub last_use_date: String,
    pub name: String,
}

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
    #[serde(rename = "UTC")]
    pub utc: String,
    pub localtime: String,
    pub whitelist: HashMap<Uuid, Whitelist>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiGroup {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiLight {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResourceLink {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRule {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiScene {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSchedule {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSensor {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUserConfig {
    pub config: ApiConfig,
    pub groups: HashMap<u32, ApiGroup>,
    pub lights: HashMap<u32, ApiLight>,
    pub resourcelinks: HashMap<u32, ApiResourceLink>,
    pub rules: HashMap<u32, ApiRule>,
    pub scenes: HashMap<u32, ApiScene>,
    pub schedules: HashMap<u32, ApiSchedule>,
    pub sensors: HashMap<u32, ApiSensor>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            analyticsconsent: false,
            backup: Default::default(),
            short_config: Default::default(),
            dhcp: true,
            internetservices: Default::default(),
            linkbutton: Default::default(),
            portalconnection: ConnectionState::Disconnected,
            portalservices: Default::default(),
            portalstate: Default::default(),
            proxyaddress: "none".to_string(),
            proxyport: Default::default(),
            swupdate2: SoftwareUpdate2::new(),
            zigbeechannel: 25,
            ipaddress: Ipv4Addr::UNSPECIFIED,
            netmask: Ipv4Addr::UNSPECIFIED,
            gateway: Ipv4Addr::UNSPECIFIED,
            timezone: "Europe/London".to_string(),
            utc: "2020-01-01T01:01:01".to_string(),
            localtime: "2020-01-01T01:01:01".to_string(),
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
    pub fn new(total: u32, available: u32) -> Self {
        Self { total, available }
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
