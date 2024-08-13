use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use log::{info, warn};
use serde_json::{json, Value};
use tokio::sync::MutexGuard;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::api::{
    Device, GroupedLight, Light, RType, ResourceLink, Room, Scene, V1ReplyBuilder,
};
use crate::hue::legacy_api::{
    ApiGroup, ApiLight, ApiLightStateUpdate, ApiResourceType, ApiScene, ApiUserConfig,
    Capabilities, HueResult, NewUser, NewUserReply,
};
use crate::resource::Resources;
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

fn get_lights(res: &MutexGuard<Resources>) -> ApiResult<HashMap<String, ApiLight>> {
    let mut lights = HashMap::new();

    for rr in res.get_resources_by_type(RType::Light) {
        let light: Light = rr.obj.try_into()?;
        let dev = res.get::<Device>(&light.owner)?.clone();
        lights.insert(
            rr.id.simple().to_string(),
            ApiLight::from_dev_and_light(&rr.id, dev, light),
        );
    }

    Ok(lights)
}

fn get_groups(res: &MutexGuard<Resources>) -> ApiResult<HashMap<String, ApiGroup>> {
    let mut rooms = HashMap::new();

    for rr in res.get_resources_by_type(RType::Room) {
        let room: Room = rr.obj.try_into()?;
        let uuid = room
            .services
            .iter()
            .find(|rl| rl.rtype == RType::GroupedLight)
            .ok_or(ApiError::NotFound(rr.id))?;

        let glight = res.get::<GroupedLight>(uuid)?.clone();
        let lights: Vec<(Uuid, Light)> = room
            .children
            .iter()
            .filter_map(|rl| res.get(rl).ok())
            .filter_map(Device::light_service)
            .filter_map(|rl| Some((rl.rid, res.get::<Light>(rl).ok()?.clone())))
            .collect();

        rooms.insert(
            rr.id.simple().to_string(),
            ApiGroup::from_lights_and_room(glight, &lights, room),
        );
    }

    Ok(rooms)
}

fn get_scenes(owner: &Uuid, res: &MutexGuard<Resources>) -> ApiResult<HashMap<String, ApiScene>> {
    let mut rooms = HashMap::new();

    for rr in res.get_resources_by_type(RType::Scene) {
        let scene: Scene = rr.obj.try_into()?;

        rooms.insert(
            rr.id.simple().to_string(),
            ApiScene::from_scene(*owner, scene),
        );
    }

    Ok(rooms)
}

#[allow(clippy::zero_sized_map_values)]
async fn get_api_user(state: State<AppState>, Path(username): Path<Uuid>) -> impl IntoResponse {
    Json(ApiUserConfig {
        config: state.api_config(username),
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
    Path((username, resource)): Path<(Uuid, ApiResourceType)>,
) -> Json<Value> {
    /* info!("user {username} resource {resource:?}"); */
    match resource {
        ApiResourceType::Config => Json(json!(state.api_config(username))),
        ApiResourceType::Groups
        | ApiResourceType::Lights
        | ApiResourceType::Resourcelinks
        | ApiResourceType::Rules
        | ApiResourceType::Scenes
        | ApiResourceType::Schedules
        | ApiResourceType::Sensors => Json(json!({})),
        ApiResourceType::Capabilities => Json(json!(Capabilities::new())),
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
