#![allow(clippy::struct_excessive_bools)]

use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::types::XY;

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DeviceUpdate {
    pub state: Option<DeviceState>,
    pub brightness: Option<f64>,
    pub color_temp: Option<u32>,
    pub color_mode: Option<DeviceColorMode>,
    pub color: Option<DeviceColor>,
    pub linkquality: Option<u8>,
    pub color_options: Option<ColorOptions>,
    pub color_temp_startup: Option<f64>,
    pub level_config: Option<LevelConfig>,
    #[serde(default)]
    pub elapsed: Option<u64>,
    #[serde(default)]
    pub power_on_behavior: Option<PowerOnBehavior>,
    #[serde(default)]
    pub update: HashMap<String, Value>,
    pub update_available: Option<bool>,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeviceColor {
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    h: Option<f64>,
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    s: Option<f64>,

    pub hue: Option<f64>,
    pub saturation: Option<f64>,

    #[serde(flatten)]
    pub xy: XY,
}

impl DeviceColor {
    #[must_use]
    pub const fn xy(xy: XY) -> Self {
        Self {
            h: None,
            s: None,
            hue: None,
            saturation: None,
            xy,
        }
    }

    #[must_use]
    pub const fn hs(h: f64, s: f64) -> Self {
        Self {
            h: None,
            s: None,
            hue: Some(h),
            saturation: Some(s),
            xy: XY::new(0.0, 0.0),
        }
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub enum PowerOnBehavior {
    #[default]
    Unknown,

    #[serde(rename = "on")]
    On,

    #[serde(rename = "off")]
    Off,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ColorOptions {
    pub execute_if_off: bool,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LevelConfig {
    pub on_off_transition_time: Option<u16>,
    pub on_transition_time: Option<u16>,
    pub off_transition_time: Option<u16>,
    pub current_level_startup: Option<CurrentLevelStartup>,
    pub on_level: Option<OnLevel>,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CurrentLevelStartup {
    Previous,
    Minimum,
    #[serde(untagged)]
    Value(u8),
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OnLevel {
    Previous,
    #[serde(untagged)]
    Value(u8),
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DeviceColorMode {
    ColorTemp,
    Xy,
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceState {
    On,
    Off,
}

impl From<bool> for DeviceState {
    fn from(value: bool) -> Self {
        if value {
            Self::On
        } else {
            Self::Off
        }
    }
}

impl From<DeviceState> for bool {
    fn from(value: DeviceState) -> Self {
        match value {
            DeviceState::On => true,
            DeviceState::Off => false,
        }
    }
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
    definitions: Value,
    required: Vec<String>,
    properties: Value,
    #[serde(rename = "type")]
    _type: Value,
    #[serde(flatten)]
    bad: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    advanced: ConfigAdvanced,
    blocklist: Vec<Option<Value>>,
    device_options: ConfigDeviceOptions,
    devices: HashMap<String, Value>,
    external_converters: Vec<Option<Value>>,
    frontend: Value,
    groups: HashMap<String, GroupValue>,
    homeassistant: ConfigHomeassistant,
    map_options: Value,
    mqtt: Value,
    ota: Value,
    passlist: Vec<Option<Value>>,
    permit_join: bool,
    serial: ConfigSerial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    channel: i64,
    extended_pan_id: f64,
    pan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coordinator {
    ieee_address: IeeeAddress,
    meta: CoordinatorMeta,
    #[serde(rename = "type")]
    coordinator_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAdvanced {
    adapter_concurrent: Option<Value>,
    adapter_delay: Option<Value>,
    availability_blacklist: Vec<Value>,
    availability_blocklist: Vec<Value>,
    availability_passlist: Vec<Value>,
    availability_whitelist: Vec<Value>,
    cache_state: bool,
    cache_state_persistent: bool,
    cache_state_send_on_startup: bool,
    channel: i64,
    elapsed: bool,
    ext_pan_id: Vec<i64>,
    homeassistant_legacy_entity_attributes: bool,
    last_seen: String,
    legacy_api: bool,
    legacy_availability_payload: bool,
    log_debug_namespace_ignore: String,
    log_debug_to_mqtt_frontend: bool,
    log_directory: String,
    log_file: String,
    log_level: String,
    log_namespaced_levels: Value,
    log_output: Vec<String>,
    log_rotation: bool,
    log_symlink_current: bool,
    log_syslog: Value,
    output: String,
    pan_id: i64,
    report: bool,
    soft_reset_timeout: i64,
    timestamp_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorMeta {
    build: i64,
    ezsp: i64,
    major: i64,
    minor: i64,
    patch: i64,
    revision: String,
    special: i64,
    #[serde(rename = "type")]
    meta_type: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSerial {
    adapter: String,
    disable_led: bool,
    port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigHomeassistant {
    discovery_topic: String,
    legacy_entity_attributes: bool,
    legacy_triggers: bool,
    status_topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigDeviceOptions {
    legacy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupValue {
    devices: Vec<String>,
    friendly_name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    description: String,
    exposes: Vec<Expose>,
    model: String,
    options: Vec<Expose>,
    supports_ota: bool,
    vendor: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeBinary {
    access: u8,
    property: String,

    name: String,
    label: String,
    description: String,

    value_off: Value,
    value_on: Value,
    value_toggle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/* #[serde(deny_unknown_fields)] */
pub struct ExposeComposite {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeEnum {
    access: u8,
    property: String,

    name: String,
    label: String,
    description: Option<String>,

    category: Option<String>,
    values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeLight {
    features: Vec<Expose>,
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
    access: u8,
    property: String,
    name: String,
    label: String,

    description: Option<String>,

    unit: Option<String>,
    category: Option<String>,
    value_max: Option<i32>,
    value_min: Option<i32>,

    #[serde(default)]
    presets: Vec<Preset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposeSwitch {
    features: Vec<Expose>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    bindings: Vec<Binding>,
    clusters: Clusters,
    configured_reportings: Vec<ConfiguredReporting>,
    scenes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfiguredReporting {
    attribute: String,
    cluster: String,
    maximum_report_interval: i32,
    minimum_report_interval: i32,
    reportable_change: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    description: String,
    name: String,
    value: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    cluster: String,
    target: BindingTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BindingTarget {
    Endpoint(EndpointLink),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clusters {
    input: Vec<String>,
    output: Vec<String>,
}
