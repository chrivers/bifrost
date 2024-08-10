#![allow(clippy::struct_excessive_bools)]

use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
#[serde(tag = "topic", content = "payload")]
pub enum Message {
    #[serde(rename = "bridge/info")]
    BridgeInfo(BridgeInfo),

    #[serde(rename = "bridge/state")]
    BridgeState(BridgeState),

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

    #[serde(untagged)]
    Other(Other),
}

#[derive(Serialize, Deserialize, Clone, Hash)]
#[serde(transparent)]
pub struct IeeeAddress(#[serde(deserialize_with = "ieee_address")] u64);

impl Debug for IeeeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IeeeAddress({:016x})", self.0)
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
#[serde(deny_unknown_fields)]
pub struct Other {
    pub topic: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BridgeOnlineState {
    Online,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BridgeState {
    pub state: BridgeOnlineState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BridgeEvent {
    /* FIXME: needs proper mapping */
    /* See: <zigbee2mqtt>/lib/extension/bridge.ts */
    pub data: Value,
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct BridgeLogging {
    pub level: String,
    pub message: String,
}

type BridgeGroups = Vec<Group>;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub friendly_name: String,
    pub id: u32,
    pub members: Vec<EndpointLink>,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EndpointLink {
    pub endpoint: u32,
    pub ieee_address: IeeeAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct BridgeConfigSchema {
    pub definitions: Value,
    pub required: Vec<String>,
    pub properties: Value,
    #[serde(rename = "type")]
    pub config_type: Value,
    #[serde(flatten)]
    pub bad: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub advanced: ConfigAdvanced,
    pub blocklist: Vec<Option<Value>>,
    pub device_options: ConfigDeviceOptions,
    pub devices: HashMap<String, Value>,
    pub external_converters: Vec<Option<Value>>,
    pub frontend: Value,
    pub groups: HashMap<String, GroupValue>,
    pub homeassistant: ConfigHomeassistant,
    pub map_options: Value,
    pub mqtt: Value,
    pub ota: Value,
    pub passlist: Vec<Option<Value>>,
    pub permit_join: bool,
    pub serial: ConfigSerial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub channel: i64,
    pub extended_pan_id: f64,
    pub pan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coordinator {
    pub ieee_address: IeeeAddress,
    pub meta: CoordinatorMeta,
    #[serde(rename = "type")]
    pub coordinator_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAdvanced {
    pub adapter_concurrent: Option<Value>,
    pub adapter_delay: Option<Value>,
    pub availability_blacklist: Vec<Value>,
    pub availability_blocklist: Vec<Value>,
    pub availability_passlist: Vec<Value>,
    pub availability_whitelist: Vec<Value>,
    pub cache_state: bool,
    pub cache_state_persistent: bool,
    pub cache_state_send_on_startup: bool,
    pub channel: i64,
    pub elapsed: bool,
    pub ext_pan_id: Vec<i64>,
    pub homeassistant_legacy_entity_attributes: bool,
    pub last_seen: String,
    pub legacy_api: bool,
    pub legacy_availability_payload: bool,
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
    pub report: bool,
    pub soft_reset_timeout: i64,
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
    pub adapter: String,
    pub disable_led: bool,
    pub port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigHomeassistant {
    pub discovery_topic: String,
    pub legacy_entity_attributes: bool,
    pub legacy_triggers: bool,
    pub status_topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDeviceOptions {
    pub legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupValue {
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

type BridgeDevices = Vec<Device>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub date_code: Option<String>,
    pub definition: Option<Definition>,
    pub disabled: bool,
    pub endpoints: HashMap<String, Endpoint>,
    pub friendly_name: String,
    pub ieee_address: IeeeAddress,
    pub interview_completed: bool,
    pub interviewing: bool,
    pub manufacturer: Option<String>,
    pub model_id: Option<String>,
    pub network_address: i64,
    #[serde(default)]
    pub power_source: PowerSource,
    pub software_build_id: Option<String>,
    pub supported: bool,
    #[serde(rename = "type")]
    pub device_type: String,
}

impl Device {
    #[must_use]
    pub fn exposes(&self) -> &[Expose] {
        self.definition.as_ref().map_or(&[], |def| &def.exposes)
    }

    #[must_use]
    pub fn expose_light(&self) -> Option<&ExposeLight> {
        self.exposes()
            .iter()
            .find_map(|exp| {
                if let Expose::Light(light) = exp {
                    Some(light)
                } else {
                    None
                }
            })
    }

    #[must_use]
    pub fn expose_action(&self) -> bool {
        self.exposes().iter().any(|exp| {
            if let Expose::Enum(ExposeEnum { name, .. }) = exp {
                name == "action"
            } else {
                false
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub description: String,
    pub exposes: Vec<Expose>,
    pub model: String,
    pub options: Vec<Expose>,
    pub supports_ota: bool,
    pub vendor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[serde(deny_unknown_fields)]
pub enum Expose {
    Binary(ExposeBinary),
    Composite(ExposeComposite),
    Enum(ExposeEnum),
    Light(ExposeLight),
    List(Value),
    Numeric(ExposeNumeric),
    Switch(ExposeSwitch),
}

impl Expose {
    pub fn name(&self) -> Option<&str> {
        match self {
            Expose::Binary(obj) => Some(obj.name.as_str()),
            Expose::Composite(_) => None,
            Expose::Enum(obj) => Some(obj.name.as_str()),
            Expose::Light(_) => None,
            Expose::List(_) => None,
            Expose::Numeric(obj) => Some(obj.name.as_str()),
            Expose::Switch(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeBinary {
    pub access: u8,
    pub property: String,

    pub name: String,
    pub label: String,
    pub description: String,

    pub value_off: Value,
    pub value_on: Value,
    pub value_toggle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/* #[serde(deny_unknown_fields)] */
pub struct ExposeComposite {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeEnum {
    pub access: u8,
    pub property: String,

    pub name: String,
    pub label: String,
    pub description: Option<String>,

    pub category: Option<String>,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeLight {
    pub features: Vec<Expose>,
}

impl ExposeLight {
    pub fn feature(&self, name: &str) -> Option<&Expose> {
        self.features.iter().find(|exp| exp.name() == Some(name))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeList {
    #[serde(flatten)]
    __unknown: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeNumeric {
    pub access: u8,
    pub property: String,
    pub name: String,
    pub label: String,

    pub description: Option<String>,

    pub unit: Option<String>,
    pub category: Option<String>,
    pub value_max: Option<i32>,
    pub value_min: Option<i32>,

    #[serde(default)]
    pub presets: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeSwitch {
    pub features: Vec<Expose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub bindings: Vec<Binding>,
    pub clusters: Clusters,
    pub configured_reportings: Vec<ConfiguredReporting>,
    pub scenes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfiguredReporting {
    pub attribute: String,
    pub cluster: String,
    pub maximum_report_interval: i32,
    pub minimum_report_interval: i32,
    pub reportable_change: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub description: String,
    pub name: String,
    pub value: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub cluster: String,
    pub target: BindingTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BindingTarget {
    Endpoint(EndpointLink),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clusters {
    pub input: Vec<String>,
    pub output: Vec<String>,
}
