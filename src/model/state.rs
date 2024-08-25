use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    hue::api::{Resource, ResourceLink},
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AuxData {
    pub topic: Option<String>,
    pub index: Option<u32>,
}

impl AuxData {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_topic(self, topic: &str) -> Self {
        Self {
            topic: Some(topic.to_string()),
            ..self
        }
    }

    #[must_use]
    pub fn with_index(self, index: u32) -> Self {
        Self {
            index: Some(index),
            ..self
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdMap {
    forward: BTreeMap<Uuid, u32>,
    reverse: BTreeMap<u32, Uuid>,
    #[serde(skip)]
    next_id: u32,
}

impl IdMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn find_next_id(&mut self) -> u32 {
        while self.reverse.contains_key(&self.next_id) {
            self.next_id += 1;
        }
        self.next_id
    }

    pub fn add(&mut self, uuid: Uuid) -> u32 {
        if let Some(id) = self.forward.get(&uuid).copied() {
            return id;
        }

        let id = self.find_next_id();

        self.forward.insert(uuid, id);
        self.reverse.insert(id, uuid);

        id
    }

    #[must_use]
    pub fn id(&self, uuid: &Uuid) -> Option<u32> {
        self.forward.get(uuid).copied()
    }

    #[must_use]
    pub fn uuid(&self, id: &u32) -> Option<Uuid> {
        self.reverse.get(id).copied()
    }

    pub fn remove(&mut self, uuid: &Uuid) {
        if let Some(id) = self.forward.remove(uuid) {
            self.reverse.remove(&id);
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct State {
    aux: BTreeMap<Uuid, AuxData>,
    id_v1: IdMap,
    pub res: BTreeMap<Uuid, Resource>,
}

impl State {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn try_aux_get(&self, id: &Uuid) -> Option<&AuxData> {
        self.aux.get(id)
    }

    pub fn aux_get(&self, link: &ResourceLink) -> ApiResult<&AuxData> {
        self.try_aux_get(&link.rid)
            .ok_or_else(|| ApiError::AuxNotFound(*link))
    }

    pub fn aux_set(&mut self, link: &ResourceLink, aux: AuxData) {
        self.aux.insert(link.rid, aux);
    }

    #[must_use]
    pub fn try_get(&self, id: &Uuid) -> Option<&Resource> {
        self.res.get(id)
    }

    pub fn get(&self, id: &Uuid) -> ApiResult<&Resource> {
        self.try_get(id).ok_or_else(|| ApiError::NotFound(*id))
    }

    pub fn get_mut(&mut self, id: &Uuid) -> ApiResult<&mut Resource> {
        self.res.get_mut(id).ok_or_else(|| ApiError::NotFound(*id))
    }

    pub fn insert(&mut self, key: Uuid, value: Resource) {
        self.res.insert(key, value);
        self.id_v1.add(key);
    }

    pub fn remove(&mut self, id: &Uuid) -> ApiResult<()> {
        self.aux.remove(id);
        self.id_v1.remove(id);
        self.res.remove(id).ok_or_else(|| ApiError::NotFound(*id))?;
        Ok(())
    }

    #[must_use]
    pub fn id_v1(&self, uuid: &Uuid) -> Option<u32> {
        self.id_v1.id(uuid)
    }

    #[must_use]
    pub fn from_id_v1(&self, id: &u32) -> Option<Uuid> {
        self.id_v1.uuid(id)
    }
}
