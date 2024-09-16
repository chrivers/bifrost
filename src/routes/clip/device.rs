use axum::{
    extract::{Path, State},
    routing::put,
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::hue::api::{Device, DeviceUpdate, RType, V2Reply};
use crate::routes::clip::ApiV2Result;
use crate::server::appstate::AppState;

async fn put_device(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT device/{id}");
    log::debug!("json data\n{}", serde_json::to_string_pretty(&put)?);

    let rlink = RType::Device.link_to(id);

    let upd: DeviceUpdate = serde_json::from_value(put)?;

    state.res.lock().await.update::<Device>(&id, |obj| {
        if let Some(md) = upd.metadata {
            if let Some(name) = md.name {
                obj.metadata.name = name;
            }
            if let Some(archetype) = md.archetype {
                obj.metadata.archetype = archetype;
            }
        }
    })?;

    V2Reply::ok(rlink)
}

pub fn router() -> Router<AppState> {
    Router::new().route("/:id", put(put_device))
}
