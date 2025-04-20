use axum::{
    extract::{Path, State},
    routing::put,
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use hue::api::BehaviorInstance;
use hue::api::{BehaviorInstanceUpdate, RType};

use crate::routes::clip::{ApiV2Result, V2Reply};
use crate::server::appstate::AppState;

async fn put_behavior_instance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT behavior_instance/{id}");
    log::debug!("json data\n{}", serde_json::to_string_pretty(&put)?);

    let rlink = RType::BehaviorInstance.link_to(id);

    log::info!("PUT behavior_instance/{id}: updating");

    let upd: BehaviorInstanceUpdate = serde_json::from_value(put)?;

    state
        .res
        .lock()
        .await
        .update::<BehaviorInstance>(&id, |bi| *bi += upd)?;

    V2Reply::ok(rlink)
}

pub fn router() -> Router<AppState> {
    Router::new().route("/{id}", put(put_behavior_instance))
}
