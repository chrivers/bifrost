use std::fmt::{self, Debug};
use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Resource;

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
