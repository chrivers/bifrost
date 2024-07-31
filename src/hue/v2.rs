use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipResourceType {
    BehaviorScript,
    Bridge,
    BridgeHome,
    Device,
    Entertainment,
    GeofenceClient,
    GroupedLight,
    Homekit,
    Light,
    Room,
    Scene,
    ZigbeeConnectivity,
    ZigbeeDeviceDiscovery,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BehaviorScript {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bridge {
    pub bridge_id: String,
    pub id: Uuid,
    pub owner: ResourceLink,
    pub time_zone: TimeZone,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeHome {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entertainment {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeofenceClient {}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupedLight {}

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
pub struct ColorTemperature {
    mirek: u32,
    mirek_schema: MirekSchema,
    mirek_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Dimming {
    brightness: f32,
    min_dim_level: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delta {}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightMetadata {
    archetype: String,
    name: String,
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
pub struct Room {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZigbeeConnectivity {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZigbeeDeviceDiscovery {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Resource {
    BehaviorScript(BehaviorScript),
    Bridge(Bridge),
    BridgeHome(BridgeHome),
    Device(Device),
    Entertainment(Entertainment),
    GeofenceClient(GeofenceClient),
    GroupedLight(GroupedLight),
    Light(Light),
    Room(Room),
    Scene(Scene),
    ZigbeeConnectivity(ZigbeeConnectivity),
    ZigbeeDeviceDiscovery(ZigbeeDeviceDiscovery),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct V2Reply<T> {
    pub data: Vec<T>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLink {
    pub rid: Uuid,
    pub rtype: ClipResourceType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeZone {
    pub time_zone: String,
}
