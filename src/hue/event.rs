use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::error::ApiResult;

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
    #[serde(default)]
    pub id_v1: Option<String>,
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
            id_v1: None,
        }
    }

    pub fn update<T: Serialize>(data: T, id_v1: Option<String>) -> ApiResult<Self> {
        Ok(Self {
            creationtime: Utc::now(),
            id: Uuid::new_v4(),
            event: Event::Update(Update {
                data: vec![serde_json::to_value(data)?],
            }),
            id_v1,
        })
    }

    pub fn delete<T: Serialize>(data: T) -> ApiResult<Self> {
        Ok(Self {
            creationtime: Utc::now(),
            id: Uuid::new_v4(),
            event: Event::Delete(Delete {
                data: vec![serde_json::to_value(data)?],
            }),
            id_v1: None,
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
