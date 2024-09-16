use serde::{Deserialize, Serialize};

use crate::hue::api::{Metadata, RType, ResourceLink, Stub};
use crate::z2m;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Device {
    pub product_data: DeviceProductData,
    pub metadata: Metadata,
    pub services: Vec<ResourceLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usertest: Option<UserTest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identify: Option<Stub>,
}

impl Device {
    #[must_use]
    pub fn light_service(&self) -> Option<&ResourceLink> {
        self.services.iter().find(|rl| rl.rtype == RType::Light)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Identify {}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserTest {
    status: String,
    usertest: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceProductData {
    pub model_id: String,
    pub manufacturer_name: String,
    pub product_name: String,
    pub product_archetype: DeviceArchetype,
    pub certified: bool,
    pub software_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_platform_type: Option<String>,
}

impl DeviceProductData {
    const SIGNIFY_MANUFACTURER_NAME: &'static str = "Signify Netherlands B.V.";

    #[must_use]
    pub fn hue_bridge_v2() -> Self {
        Self {
            certified: true,
            manufacturer_name: Self::SIGNIFY_MANUFACTURER_NAME.to_string(),
            model_id: "BSB002".to_string(),
            product_archetype: DeviceArchetype::BridgeV2,
            product_name: "Hue Bridge".to_string(),
            software_version: "1.66.1966060010".to_string(),
            hardware_platform_type: None,
        }
    }

    #[must_use]
    pub fn guess_from_device(dev: &z2m::api::Device) -> Self {
        fn str_or_unknown(name: &Option<String>) -> String {
            name.clone().unwrap_or_else(|| String::from("<unknown>"))
        }

        let product_name = str_or_unknown(&dev.model_id);
        let model_id = str_or_unknown(&dev.definition.as_ref().map(|def| def.model.clone()));
        let manufacturer_name = str_or_unknown(&dev.manufacturer);
        let certified = manufacturer_name == Self::SIGNIFY_MANUFACTURER_NAME;
        let software_version = str_or_unknown(&dev.software_build_id);

        let product_archetype = DeviceArchetype::SpotBulb;

        Self {
            model_id,
            manufacturer_name,
            product_name,
            product_archetype,
            certified,
            software_version,
            hardware_platform_type: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DeviceArchetype {
    BridgeV2,
    UnknownArchetype,
    ClassicBulb,
    SultanBulb,
    FloodBulb,
    SpotBulb,
    CandleBulb,
    LusterBulb,
    PendantRound,
    PendantLong,
    CeilingRound,
    CeilingSquare,
    FloorShade,
    FloorLantern,
    TableShade,
    RecessedCeiling,
    RecessedFloor,
    SingleSpot,
    DoubleSpot,
    TableWash,
    WallLantern,
    WallShade,
    FlexibleLamp,
    GroundSpot,
    WallSpot,
    Plug,
    HueGo,
    HueLightstrip,
    HueIris,
    HueBloom,
    Bollard,
    WallWasher,
    HuePlay,
    VintageBulb,
    VintageCandleBulb,
    EllipseBulb,
    TriangleBulb,
    SmallGlobeBulb,
    LargeGlobeBulb,
    EdisonBulb,
    ChristmasTree,
    StringLight,
    HueCentris,
    HueLightstripTv,
    HueLightstripPc,
    HueTube,
    HueSigne,
    PendantSpot,
    CeilingHorizontal,
    CeilingTube,

    #[serde(untagged)]
    Other(String),
}
