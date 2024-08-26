use serde::{Deserialize, Serialize};

use crate::hue::api::{RType, ResourceLink};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomMetadata {
    pub name: String,
    pub archetype: RoomArchetype,
    pub hidden: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub children: Vec<ResourceLink>,
    pub metadata: RoomMetadata,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

impl Room {
    #[must_use]
    pub fn grouped_light_service(&self) -> Option<&ResourceLink> {
        self.services
            .iter()
            .find(|rl| rl.rtype == RType::GroupedLight)
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RoomArchetype {
    LivingRoom,
    Kitchen,
    Dining,
    Bedroom,
    KidsBedroom,
    Bathroom,
    Nursery,
    Recreation,
    Office,
    Gym,
    Hallway,
    Toilet,
    FrontDoor,
    Garage,
    Terrace,
    Garden,
    Driveway,
    Carport,
    Home,
    Downstairs,
    Upstairs,
    TopFloor,
    Attic,
    GuestRoom,
    Staircase,
    Lounge,
    ManCave,
    Computer,
    Studio,
    Music,
    Tv,
    Reading,
    Closet,
    Storage,
    LaundryRoom,
    Balcony,
    Porch,
    Barbecue,
    Pool,
    Other,
}

impl RoomMetadata {
    #[must_use]
    pub fn new(archetype: RoomArchetype, name: &str) -> Self {
        Self {
            archetype,
            name: name.to_string(),
            hidden: false
        }
    }
}
