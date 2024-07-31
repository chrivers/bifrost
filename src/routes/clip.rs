use axum::extract::State;
use axum::{extract::Path, response::IntoResponse, routing::get, Json, Router};
use serde_json::Value;
use uuid::Uuid;

use crate::hue::v2::{ResourceType, V2Reply};
use crate::state::AppState;

async fn get_clipv2(State(state): State<AppState>) -> impl IntoResponse {
    Json(V2Reply {
        data: state.get_resources().await,
        errors: vec![],
    })
}

async fn get_clipv2_resource(
    State(state): State<AppState>,
    Path(rtype): Path<ResourceType>,
) -> impl IntoResponse {
    Json(V2Reply {
        data: state.get_resources_by_type(rtype).await,
        errors: vec![],
    })
}

async fn post_clipv2_resource(
    State(_state): State<AppState>,
    Path(rtype): Path<ResourceType>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("POST {rtype:?}: {req:?}");
}

async fn get_clipv2_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
) -> impl IntoResponse {
    if let Some(res) = state.get_resource(rtype, id).await {
        Json(V2Reply {
            data: vec![res],
            errors: vec![],
        })
    } else {
        Json(V2Reply {
            data: vec![],
            errors: vec!["glump".to_owned()],
        })
    }
}

async fn put_clipv2_resource_id(
    State(_state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("PUT {rtype:?}/{id}: {req:?}");
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_clipv2))
        .route(
            "/:resource",
            get(get_clipv2_resource).post(post_clipv2_resource),
        )
        .route(
            "/:resource/:id",
            get(get_clipv2_resource_id).put(put_clipv2_resource_id),
        )
}
