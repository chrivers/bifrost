use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Event {
    Add(Add),
    Update(Update),
    Delete(Delete),
    Error(Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventBlock {
    id: Uuid,
    #[serde(flatten)]
    event: Event,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Add {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Update {
    data: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delete {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {}
