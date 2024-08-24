use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::error::ApiError;
use crate::hue::api::{RType, Resource, ResourceLink, V2Reply};
use crate::routes::clip::ApiV2Result;
use crate::server::appstate::AppState;

async fn get_root(State(state): State<AppState>) -> impl IntoResponse {
    V2Reply::list(state.res.lock().await.get_resources())
}

async fn get_resource(State(state): State<AppState>, Path(rtype): Path<RType>) -> ApiV2Result {
    V2Reply::list(state.res.lock().await.get_resources_by_type(rtype))
}

async fn post_resource(
    State(state): State<AppState>,
    Path(rtype): Path<RType>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("POST: {rtype:?} {}", serde_json::to_string(&req)?);

    let obj = Resource::from_value(rtype, req)?;

    let mut lock = state.res.lock().await;

    let rlink = ResourceLink::new(Uuid::new_v4(), obj.rtype());
    lock.add(&rlink, obj)?;
    drop(lock);

    V2Reply::ok(rlink)
}

async fn get_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
) -> ApiV2Result {
    V2Reply::ok(state.res.lock().await.get_resource(rtype, &id)?)
}

async fn put_resource_id(
    Path((rtype, id)): Path<(RType, Uuid)>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT {rtype:?}/{id}");
    log::debug!("json data\n{}", serde_json::to_string_pretty(&put)?);

    log::warn!("PUT {rtype:?}/{id}: state update not supported");

    Err(ApiError::UpdateUnsupported(rtype))
}

async fn delete_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
) -> ApiV2Result {
    log::info!("DELETE {rtype:?}/{id}");

    state.res.lock().await.get_resource(rtype, &id)?;

    Err(ApiError::DeleteDenied(id))?
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_root))
        .route("/:resource", get(get_resource))
        .route("/:resource", post(post_resource))
        .route("/:resource/:id", get(get_resource_id))
        .route("/:resource/:id", put(put_resource_id))
        .route("/:resource/:id", delete(delete_resource_id))
}
