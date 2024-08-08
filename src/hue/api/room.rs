use serde::{Deserialize, Serialize};

use super::{Metadata, RType, ResourceLink};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    pub children: Vec<ResourceLink>,
    pub metadata: Metadata,
    #[serde(default)]
    pub services: Vec<ResourceLink>,
}

impl Room {
    #[must_use]
    pub fn group(&self) -> Option<&ResourceLink> {
        self.services
            .iter()
            .find(|rl| rl.rtype == RType::GroupedLight)
    }
}

#[derive(Copy, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RoomArchetypes {
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
