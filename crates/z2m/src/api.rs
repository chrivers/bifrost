#![allow(clippy::struct_excessive_bools)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawMessage {
    pub topic: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "topic", content = "payload")]
pub enum Message {
    #[serde(rename = "bridge/info")]
    BridgeInfo(Box<BridgeInfo>),

    #[serde(rename = "bridge/state")]
    BridgeState(Value),

    #[serde(rename = "bridge/event")]
    BridgeEvent(BridgeEvent),

    #[serde(rename = "bridge/devices")]
    BridgeDevices(BridgeDevices),

    #[serde(rename = "bridge/groups")]
    BridgeGroups(BridgeGroups),

    #[serde(rename = "bridge/logging")]
    BridgeLogging(BridgeLogging),

    #[serde(rename = "bridge/definitions")]
    BridgeDefinitions(Value),

    #[serde(rename = "bridge/extensions")]
    BridgeExtensions(Value),

    #[serde(rename = "bridge/converters")]
    BridgeConverters(Value),

    #[serde(rename = "bridge/response/options")]
    BridgeOptions(Value),

    #[serde(rename = "bridge/response/touchlink/scan")]
    BridgeTouchlinkScan(Value),

    #[serde(rename = "bridge/response/permit_join")]
    BridgePermitJoin(Value),

    #[serde(rename = "bridge/response/networkmap")]
    BridgeNetworkmap(Value),

    #[serde(rename = "bridge/config")]
    BridgeConfig(Value),

    #[serde(rename = "bridge/response/group/add")]
    BridgeResponseGroupAdd(Response<GroupAdd>),

    #[serde(rename = "bridge/response/group/remove")]
    BridgeResponseGroupRemove(Response<GroupRemove>),

    #[serde(rename = "bridge/response/group/rename")]
    BridgeResponseGroupRename(Response<GroupRename>),

    #[serde(rename = "bridge/response/group/options")]
    BridgeResponseGroupOptions(Response<GroupOptions>),

    #[serde(rename = "bridge/response/group/members/add")]
    BridgeGroupMembersAdd(Response<GroupMemberChange>),

    #[serde(rename = "bridge/response/group/members/remove")]
    BridgeGroupMembersRemove(Response<GroupMemberChange>),

    #[serde(rename = "bridge/response/device/remove")]
    BridgeDeviceRemove(Response<DeviceRemoveResponse>),

    #[serde(rename = "bridge/response/device/options")]
    BridgeDeviceOptions(Value),

    #[serde(rename = "bridge/response/device/configure_reporting")]
    BridgeDeviceConfigureReporting(Value),

    #[serde(rename = "bridge/response/device/ota_update/check")]
    BridgeDeviceOtaUpdateCheck(Value),

    #[serde(rename = "bridge/health")]
    BridgeHealth(BridgeHealth),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum Endpoint {
    #[default]
    Default,
    #[serde(untagged)]
    Name(String),
    #[serde(untagged)]
    Number(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMemberChange {
    pub device: String,
    pub group: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<Endpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_disable_reporting: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PermitJoin {
    pub time: u32,
    pub device: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceRemove {
    pub id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceRemoveResponse {
    pub id: String,
    pub block: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Response<T> {
    Ok {
        data: T,
        #[serde(default)]
        transaction: Option<Value>,
    },
    Error {
        error: Value,
        #[serde(default)]
        transaction: Option<Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAdd {
    pub id: Option<u32>,
    pub friendly_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRemove {
    pub id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberAddRemove {
    pub device: String,
    pub endpoint: u8,
    pub group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupRename {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupOptions {
    pub from: Value,
    pub to: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRename {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub homeassistant_rename: bool,
}

#[derive(Serialize, Deserialize, Clone, Hash, Debug, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Online,
    Offline,
}

#[derive(Serialize, Deserialize, Clone, Hash)]
#[serde(transparent)]
pub struct IeeeAddress(#[serde(deserialize_with = "ieee_address")] u64);

impl Debug for IeeeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IeeeAddress({:016x})", self.0)
    }
}

impl Display for IeeeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

fn ieee_address<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s: &str = Deserialize::deserialize(deserializer)?;
    let num = u64::from_str_radix(s.trim_start_matches("0x"), 16).map_err(Error::custom)?;
    Ok(num)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BridgeOnlineState {
    Online,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeState {
    pub state: BridgeOnlineState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeEvent {
    /* FIXME: needs proper mapping */
    /* See: <zigbee2mqtt>/lib/extension/bridge.ts */
    pub data: Value,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BridgeLogging {
    pub level: String,
    pub message: String,
    pub topic: Option<String>,
}

type BridgeGroups = Vec<Group>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub friendly_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub id: u32,
    pub members: Vec<GroupMember>,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupMember {
    pub endpoint: u32,
    pub ieee_address: IeeeAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointLink {
    pub endpoint: u32,
    pub ieee_address: IeeeAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupLink {
    pub id: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeInfo {
    pub commit: String,
    pub config: Config,
    pub config_schema: BridgeConfigSchema,
    pub coordinator: Coordinator,
    pub log_level: String,
    pub network: Network,
    pub permit_join: bool,
    pub restart_required: bool,
    pub version: String,
    pub zigbee_herdsman: Version,
    pub zigbee_herdsman_converters: Version,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfigSchema {
    pub definitions: Value,
    #[serde(default)]
    pub required: Vec<String>,
    pub properties: Value,
    #[serde(rename = "type")]
    pub config_type: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub advanced: ConfigAdvanced,
    #[serde(default)]
    pub availability: Value,
    #[serde(default)]
    pub version: Value,
    pub blocklist: Vec<Option<Value>>,
    pub device_options: Value,
    pub devices: HashMap<String, Value>,
    #[serde(default)]
    pub external_converters: Vec<Option<Value>>,
    pub frontend: Value,
    pub groups: HashMap<String, GroupValue>,
    #[serde(with = "crate::serde_util::struct_or_false")]
    pub homeassistant: Option<ConfigHomeassistant>,
    pub map_options: Value,
    pub mqtt: Value,
    pub ota: Value,
    pub passlist: Vec<Option<Value>>,
    pub serial: ConfigSerial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub channel: i64,
    pub extended_pan_id: Value,
    pub pan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinator {
    pub ieee_address: IeeeAddress,
    /* stict parsing disabled for now, format too volatile between versions */
    /* pub meta: CoordinatorMeta, */
    pub meta: Value,
    #[serde(rename = "type")]
    pub coordinator_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAdvanced {
    pub adapter_concurrent: Option<Value>,
    pub adapter_delay: Option<Value>,
    pub cache_state: bool,
    pub cache_state_persistent: bool,
    pub cache_state_send_on_startup: bool,
    pub channel: i64,
    pub elapsed: bool,
    pub ext_pan_id: Vec<i64>,
    pub homeassistant_legacy_entity_attributes: Option<bool>,
    pub last_seen: String,
    pub log_debug_namespace_ignore: String,
    pub log_debug_to_mqtt_frontend: bool,
    pub log_directory: String,
    pub log_file: String,
    pub log_level: String,
    pub log_namespaced_levels: Value,
    pub log_output: Vec<String>,
    pub log_rotation: bool,
    pub log_symlink_current: bool,
    pub log_syslog: Value,
    pub output: String,
    pub pan_id: i64,
    pub timestamp_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorMeta {
    pub build: i64,
    pub ezsp: i64,
    pub major: i64,
    pub minor: i64,
    pub patch: i64,
    pub revision: String,
    pub special: i64,
    #[serde(rename = "type")]
    pub meta_type: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSerial {
    pub adapter: Option<String>,
    pub disable_led: bool,
    #[serde(default)]
    pub port: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigHomeassistant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub experimental_event_entities: Option<Value>,
    #[serde(default)]
    pub legacy_action_sensor: Option<Value>,
    pub discovery_topic: String,
    pub status_topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupValue {
    #[serde(default)]
    pub devices: Vec<String>,
    pub friendly_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum PowerSource {
    #[serde(rename = "Unknown")]
    #[default]
    Unknown = 0,
    #[serde(rename = "Mains (single phase)")]
    MainsSinglePhase = 1,
    #[serde(rename = "Mains (3 phase)")]
    MainsThreePhase = 2,
    #[serde(rename = "Battery")]
    Battery = 3,
    #[serde(rename = "DC Source")]
    DcSource = 4,
    #[serde(rename = "Emergency mains constantly powered")]
    EmergencyMainsConstantly = 5,
    #[serde(rename = "Emergency mains and transfer switch")]
    EmergencyMainsAndTransferSwitch = 6,
}

pub type BridgeDevices = Vec<Device>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    Coordinator,
    Router,
    EndDevice,
    Unknown,
    GreenPower,
}

#[allow(clippy::pub_underscore_fields)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub description: Option<String>,
    pub date_code: Option<String>,
    pub definition: Option<DeviceDefinition>,
    pub disabled: bool,
    pub endpoints: HashMap<String, DeviceEndpoint>,
    pub friendly_name: String,
    pub ieee_address: IeeeAddress,
    pub interview_completed: bool,
    pub interviewing: bool,
    pub manufacturer: Option<String>,
    pub model_id: Option<String>,
    pub network_address: u16,
    #[serde(default)]
    pub power_source: PowerSource,
    pub software_build_id: Option<String>,
    pub supported: Option<bool>,
    #[serde(rename = "type")]
    pub device_type: DeviceType,

    /* all other fields */
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default, flatten)]
    pub __: HashMap<String, Value>,
}

impl Device {
    #[must_use]
    pub fn exposes(&self) -> &[Expose] {
        self.definition.as_ref().map_or(&[], |def| &def.exposes)
    }

    #[must_use]
    pub fn expose_light(&self) -> Option<&ExposeLight> {
        self.exposes().iter().find_map(|exp| {
            if let Expose::Light(light) = exp {
                Some(light)
            } else {
                None
            }
        })
    }

    #[must_use]
    pub fn expose_gradient(&self) -> Option<&ExposeList> {
        self.exposes().iter().find_map(|exp| {
            if let Expose::List(grad) = exp {
                if grad
                    .base
                    .property
                    .as_ref()
                    .is_some_and(|prop| prop == "gradient")
                {
                    Some(grad)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    #[must_use]
    pub fn expose_action(&self) -> bool {
        self.exposes().iter().any(|exp| {
            if let Expose::Enum(ExposeEnum { base, .. }) = exp {
                base.name.as_deref() == Some("action")
            } else {
                false
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDefinition {
    pub model: String,
    pub vendor: String,
    pub description: String,
    pub exposes: Vec<Expose>,
    pub supports_ota: bool,
    pub options: Vec<Expose>,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Expose {
    Binary(ExposeBinary),
    Composite(ExposeComposite),
    Enum(ExposeEnum),
    Light(ExposeLight),
    Lock(ExposeLock),
    Numeric(ExposeNumeric),
    Switch(ExposeSwitch),
    List(ExposeList),

    /* FIXME: Not modelled yet */
    Text(ExposeGeneric),
    Cover(ExposeGeneric),
    Fan(ExposeGeneric),
    Climate(ExposeGeneric),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeGeneric {
    #[serde(flatten)]
    pub base: ExposeBase,
    #[serde(flatten)]
    pub other: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExposeCategory {
    Config,
    Diagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeBase {
    pub name: Option<String>,
    pub label: Option<String>,
    #[serde(default)]
    pub access: u8,
    pub endpoint: Option<String>,
    pub property: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub features: Vec<Expose>,
    pub category: Option<ExposeCategory>,
}

impl Expose {
    #[must_use]
    pub const fn base(&self) -> &ExposeBase {
        #[allow(clippy::match_same_arms)]
        match self {
            Self::Binary(exp) => &exp.base,
            Self::Composite(exp) => &exp.base,
            Self::Enum(exp) => &exp.base,
            Self::Light(exp) => &exp.base,
            Self::List(exp) => &exp.base,
            Self::Lock(exp) => &exp.base,
            Self::Numeric(exp) => &exp.base,
            Self::Switch(exp) => &exp.base,
            Self::Text(exp) => &exp.base,
            Self::Cover(exp) => &exp.base,
            Self::Fan(exp) => &exp.base,
            Self::Climate(exp) => &exp.base,
        }
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.base().name.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeBinary {
    #[serde(flatten)]
    pub base: ExposeBase,
    pub value_off: Value,
    pub value_on: Value,
    pub value_toggle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeComposite {
    #[serde(flatten)]
    pub base: ExposeBase,
    // FIXME
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeEnum {
    #[serde(flatten)]
    pub base: ExposeBase,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeLight {
    #[serde(flatten)]
    pub base: ExposeBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeLock {
    #[serde(flatten)]
    pub base: ExposeBase,
}

impl ExposeLight {
    #[must_use]
    pub fn feature(&self, name: &str) -> Option<&Expose> {
        self.base
            .features
            .iter()
            .find(|exp| exp.name() == Some(name))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeList {
    #[serde(flatten)]
    pub base: ExposeBase,
    pub item_type: Box<Expose>,
    #[serde(default)]
    pub length_min: Option<u32>,
    #[serde(default)]
    pub length_max: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeNumeric {
    #[serde(flatten)]
    pub base: ExposeBase,

    pub unit: Option<String>,
    pub value_max: Option<f64>,
    pub value_min: Option<f64>,
    pub value_step: Option<f64>,

    #[serde(default)]
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeSwitch {
    #[serde(flatten)]
    pub base: ExposeBase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEndpoint {
    pub bindings: Vec<DeviceEndpointBinding>,
    pub configured_reportings: Vec<DeviceEndpointConfiguredReporting>,
    pub clusters: DeviceEndpointClusters,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEndpointConfiguredReporting {
    pub attribute: Value,
    pub cluster: String,
    pub maximum_report_interval: i64,
    pub minimum_report_interval: i64,
    #[serde(default)]
    pub reportable_change: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub description: String,
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEndpointBinding {
    pub cluster: String,
    pub target: DeviceEndpointBindingTarget,
}

// NOTE: definition diverges from z2m, but is more strict
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DeviceEndpointBindingTarget {
    Group(GroupLink),
    Endpoint(EndpointLink),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEndpointClusters {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealth {
    pub devices: Option<HashMap<String, BridgeHealthDevice>>,
    pub mqtt: Option<BridgeHealthMqtt>,
    pub os: Option<BridgeHealthOs>,
    pub process: Option<BridgeHealthProcess>,
    pub response_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealthDevice {
    pub leave_count: Option<i64>,
    pub messages: Option<i64>,
    pub messages_per_sec: Option<f64>,
    pub network_address_changes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealthMqtt {
    pub connected: bool,
    pub published: Option<i64>,
    pub queued: Option<i64>,
    pub received: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealthOs {
    pub load_average: Option<Vec<f64>>,
    pub memory_percent: Option<f64>,
    pub memory_used_mb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealthProcess {
    pub memory_percent: Option<f64>,
    pub memory_used_mb: Option<f64>,
    pub uptime_sec: Option<f64>,
}
