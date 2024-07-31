use axum::extract::State;
use axum::{extract::Path, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;

use crate::hue::v2::{ClipResourceType, V2Reply};
use crate::state::AppState;

async fn get_clipv2(State(state): State<AppState>) -> impl IntoResponse {
    let bridge = state.get_bridge();

    Json(V2Reply {
        data: vec![bridge],
        errors: vec![],
    })
}

async fn get_clipv2_resource(
    State(state): State<AppState>,
    Path(resource): Path<ClipResourceType>,
) -> impl IntoResponse {
    match resource {
        ClipResourceType::Bridge => {
            let bridge = state.get_bridge();
            Json(json!(V2Reply {
                data: vec![bridge],
                errors: vec![],
            }))
        }
        _ => Json(json!("nope")),
    }
}

async fn get_clipv2_resource_id(
    Path((resource, _id)): Path<(ClipResourceType, String)>,
) -> impl IntoResponse {
    match resource {
        ClipResourceType::Bridge => Json(json!({})),
        _ => Json(json!("nope")),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_clipv2))
        .route("/:resource", get(get_clipv2_resource))
        .route("/:resource/:id", get(get_clipv2_resource_id))
}
