#![allow(clippy::struct_excessive_bools)]

use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::hue::api::MirekSchema;
use crate::z2m::serde_util;

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
    pub topic: Option<String>,
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
pub struct GroupLink {
    pub id: u32,
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
    #[serde(default)]
    pub required: Vec<String>,
    pub properties: Value,
    #[serde(rename = "type")]
    pub config_type: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub advanced: ConfigAdvanced,
    #[serde(default)]
    pub availability: Value,
    pub blocklist: Vec<Option<Value>>,
    pub device_options: ConfigDeviceOptions,
    pub devices: HashMap<String, Value>,
    pub external_converters: Vec<Option<Value>>,
    pub frontend: Value,
    pub groups: HashMap<String, GroupValue>,
    #[serde(with = "serde_util::struct_or_false")]
    pub homeassistant: Option<ConfigHomeassistant>,
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
    pub extended_pan_id: Value,
    pub pan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub homeassistant_legacy_entity_attributes: Option<bool>,
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
    pub adapter: Option<String>,
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

#[allow(clippy::pub_underscore_fields)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub description: Option<String>,
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
    pub supported: Option<bool>,
    #[serde(rename = "type")]
    pub device_type: String,

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
pub enum Expose {
    Binary(ExposeBinary),
    Composite(ExposeComposite),
    Enum(ExposeEnum),
    Light(ExposeLight),
    List(Value),
    Lock(ExposeLock),
    Numeric(ExposeNumeric),
    Switch(ExposeSwitch),

    /* FIXME: Not modelled yet */
    Text(Value),
    Cover(Value),
    Fan(Value),
    Climate(Value),
}

impl Expose {
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Binary(obj) => Some(obj.name.as_str()),
            Self::Composite(obj) => Some(obj.name.as_str()),
            Self::Enum(obj) => Some(obj.name.as_str()),
            Self::Numeric(obj) => Some(obj.name.as_str()),
            Self::Light(_)
            | Self::List(_)
            | Self::Switch(_)
            | Self::Lock(_)
            | Self::Text(_)
            | Self::Cover(_)
            | Self::Fan(_)
            | Self::Climate(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeBinary {
    pub access: u8,
    pub property: String,

    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,

    pub value_off: Value,
    pub value_on: Value,
    pub value_toggle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeComposite {
    pub access: Option<u8>,
    pub property: String,

    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,

    pub category: Option<String>,

    #[serde(default)]
    pub features: Vec<Expose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeEnum {
    pub access: u8,
    pub property: String,

    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,

    pub category: Option<String>,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeLight {
    pub features: Vec<Expose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeLock {
    pub features: Vec<Expose>,
    pub label: Option<String>,
}

impl ExposeLight {
    #[must_use]
    pub fn feature(&self, name: &str) -> Option<&Expose> {
        self.features.iter().find(|exp| exp.name() == Some(name))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeList {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeNumeric {
    pub access: u8,
    pub property: String,
    pub name: String,
    pub label: Option<String>,

    pub description: Option<String>,

    pub unit: Option<String>,
    pub category: Option<String>,
    pub value_max: Option<f64>,
    pub value_min: Option<f64>,
    pub value_step: Option<f64>,

    #[serde(default)]
    pub presets: Vec<Preset>,
}

impl ExposeNumeric {
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn extract_mirek_schema(&self) -> Option<MirekSchema> {
        if self.unit.as_deref() == Some("mired") {
            if let (Some(min), Some(max)) = (self.value_min, self.value_max) {
                return Some(MirekSchema {
                    mirek_minimum: min as u32,
                    mirek_maximum: max as u32,
                });
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeSwitch {
    pub features: Vec<Expose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default)]
    pub reportable_change: Value,
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
    Group(GroupLink),
    Endpoint(EndpointLink),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clusters {
    pub input: Vec<String>,
    pub output: Vec<String>,
}
