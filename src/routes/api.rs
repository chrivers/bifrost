use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use log::warn;
use serde_json::{json, Value};
use tracing::info;
use uuid::Uuid;

use crate::hue::v1::{ApiResourceType, ApiUserConfig, HueResult, NewUser, NewUserReply};
use crate::state::AppState;

async fn get_api() -> impl IntoResponse {
    "yep"
}

async fn get_api_config(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.api_short_config())
}

async fn post_api(Json(j): Json<NewUser>) -> impl IntoResponse {
    info!("post: {j:?}");
    let res = NewUserReply {
        clientkey: Uuid::new_v4(),
        username: Uuid::new_v4(),
    };
    Json(vec![HueResult::Success(res)])
}

async fn get_api_user(state: State<AppState>, Path(_username): Path<String>) -> impl IntoResponse {
    Json(ApiUserConfig {
        config: state.api_config(),
        groups: HashMap::new(),
        lights: HashMap::new(),
        resourcelinks: HashMap::new(),
        rules: HashMap::new(),
        scenes: HashMap::new(),
        schedules: HashMap::new(),
        sensors: HashMap::new(),
    })
}

async fn get_api_user_resource(
    State(state): State<AppState>,
    Path((_username, resource)): Path<(String, ApiResourceType)>,
) -> Json<Value> {
    /* info!("user {username} resource {resource:?}"); */
    match resource {
        ApiResourceType::Config => Json(json!(state.api_config())),
        ApiResourceType::Groups => Json(json!({})),
        ApiResourceType::Lights => Json(json!({})),
        ApiResourceType::Resourcelinks => Json(json!({})),
        ApiResourceType::Rules => Json(json!({})),
        ApiResourceType::Scenes => Json(json!({})),
        ApiResourceType::Schedules => Json(json!({})),
        ApiResourceType::Sensors => Json(json!({})),
    }
}

async fn put_api_user_resource(
    Path((_username, _resource)): Path<(String, String)>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    warn!("PUT v1 user resource {req:?}");
    //Json(format!("user {username} resource {resource}"))
    Json(vec![HueResult::Success(req)])
}

async fn get_api_user_resource_id(
    Path((username, resource, id)): Path<(String, String, String)>,
) -> impl IntoResponse {
    warn!("GET v1 user resource id");
    format!("user {username} resource {resource} id {id}")
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_api).post(post_api))
        .route("/config", get(get_api_config))
        .route("/:username", get(get_api_user))
        .route(
            "/:username/:resource",
            get(get_api_user_resource).put(put_api_user_resource),
        )
        .route("/:username/:resource/:id", get(get_api_user_resource_id))
}
