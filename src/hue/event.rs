use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::ApiResult,
    hue::api::{self, ResourceLink},
};

use super::date_format;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Event {
    Add(Add),
    Update(Update),
    Delete(Delete),
    Error(Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventBlock {
    #[serde(with = "date_format::utc")]
    pub creationtime: DateTime<Utc>,
    pub id: Uuid,
    #[serde(flatten)]
    pub event: Event,
}

impl EventBlock {
    #[must_use]
    pub fn add(data: Value) -> Self {
        Self {
            creationtime: Utc::now(),
            id: Uuid::new_v4(),
            event: Event::Add(Add { data: vec![data] }),
        }
    }

    pub fn update(id: &Uuid, data: api::Update) -> ApiResult<Self> {
        Ok(Self {
            creationtime: Utc::now(),
            id: Uuid::new_v4(),
            event: Event::Update(Update {
                data: vec![serde_json::to_value(api::UpdateRecord::new(id, data))?],
            }),
        })
    }

    pub fn delete(link: &ResourceLink) -> ApiResult<Self> {
        Ok(Self {
            creationtime: Utc::now(),
            id: Uuid::new_v4(),
            event: Event::Delete(Delete {
                data: vec![json!({
                    "id": link.rid,
                    "id_v1": format!("/legacy/{}", link.rid.as_simple()),
                    "rtype": link.rtype,
                })],
            }),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Add {
    pub data: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Update {
    pub data: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Delete {
    pub data: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Error {}
