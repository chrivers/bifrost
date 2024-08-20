use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put},
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
use crate::z2m::request::ClientRequest;
use crate::z2m::update::DeviceUpdate;

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
async fn get_api_user(
    state: State<AppState>,
    Path(username): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let lock = state.res.lock().await;

    Ok(Json(ApiUserConfig {
        config: state.api_config(username),
        groups: get_groups(&lock)?,
        lights: get_lights(&lock)?,
        resourcelinks: HashMap::new(),
        rules: HashMap::new(),
        scenes: get_scenes(&username, &lock)?,
        schedules: HashMap::new(),
        sensors: HashMap::new(),
    }))
}

async fn get_api_user_resource(
    State(state): State<AppState>,
    Path((username, resource)): Path<(Uuid, ApiResourceType)>,
) -> ApiResult<Json<Value>> {
    let lock = &state.res.lock().await;
    match resource {
        ApiResourceType::Config => Ok(Json(json!(state.api_config(username)))),
        ApiResourceType::Lights => Ok(Json(json!(get_lights(lock)?))),
        ApiResourceType::Groups => Ok(Json(json!(get_groups(lock)?))),
        ApiResourceType::Scenes => Ok(Json(json!(get_scenes(&username, lock)?))),
        ApiResourceType::Resourcelinks
        | ApiResourceType::Rules
        | ApiResourceType::Schedules
        | ApiResourceType::Sensors => Ok(Json(json!({}))),
        ApiResourceType::Capabilities => Ok(Json(json!(Capabilities::new()))),
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

#[allow(clippy::significant_drop_tightening)]
async fn get_api_user_resource_id(
    State(state): State<AppState>,
    Path((username, resource, id)): Path<(Uuid, ApiResourceType, Uuid)>,
) -> ApiResult<impl IntoResponse> {
    warn!("GET v1 user resource id");
    match resource {
        ApiResourceType::Lights => {
            let lock = state.res.lock().await;
            let link = ResourceLink::new(id, RType::Light);
            let light = lock.get::<Light>(&link)?;
            let dev = lock.get::<Device>(&light.owner)?.clone();
            Ok(Json(json!(ApiLight::from_dev_and_light(
                &id,
                dev,
                light.clone(),
            ))))
        }
        ApiResourceType::Scenes => {
            let lock = state.res.lock().await;
            let link = ResourceLink::new(id, RType::Scene);
            let scene = lock.get::<Scene>(&link)?.clone();
            Ok(Json(json!(ApiScene::from_scene(username, scene))))
        }
        _ => Err(ApiError::NotFound(id)),
    }
}

async fn put_api_user_resource_id_state(
    State(state): State<AppState>,
    Path((_username, resource, id, path)): Path<(String, ApiResourceType, Uuid, String)>,
    Json(req): Json<Value>,
) -> ApiResult<Json<Value>> {
    match resource {
        ApiResourceType::Lights => {
            println!("req: {req:#?}");
            match path.as_str() {
                "state" => {
                    let lock = state.res.lock().await;
                    let link = ResourceLink::new(id, RType::Light);
                    let upd: ApiLightStateUpdate = serde_json::from_value(req)?;

                    let payload = DeviceUpdate::default()
                        .with_state(upd.on)
                        .with_brightness(upd.bri.map(f64::from))
                        .with_color_xy(upd.xy.map(Into::into))
                        .with_color_temp(upd.ct);

                    lock.z2m_request(ClientRequest::light_update(link, payload))?;
                    drop(lock);

                    let reply = V1ReplyBuilder::new(format!("/lights/{}/{path}", id.as_simple()))
                        .add_option("on", upd.on)?
                        .add_option("bri", upd.bri)?
                        .add_option("xy", upd.xy)?
                        .add_option("ct", upd.ct)?;

                    Ok(Json(reply.json()))
                }
                _ => Err(ApiError::NotFound(id)),
            }
        }
        ApiResourceType::Groups => match path.as_str() {
            "action" => {
                let lock = state.res.lock().await;
                let link = ResourceLink::new(id, RType::Room);
                let room: &Room = lock.get(&link)?;
                let glight = room.grouped_light_service().unwrap();

                let upd: ApiLightStateUpdate = serde_json::from_value(req)?;

                let payload = DeviceUpdate::default()
                    .with_state(upd.on)
                    .with_brightness(upd.bri.map(f64::from))
                    .with_color_xy(upd.xy.map(Into::into))
                    .with_color_temp(upd.ct);

                lock.z2m_request(ClientRequest::group_update(*glight, payload))?;
                drop(lock);

                let reply = V1ReplyBuilder::new(format!("/groups/{}/{path}", id.as_simple()))
                    .add_option("on", upd.on)?
                    .add_option("bri", upd.bri)?
                    .add_option("xy", upd.xy)?
                    .add_option("ct", upd.ct)?;

                Ok(Json(reply.json()))
            }
            _ => Err(ApiError::NotFound(id)),
        },
        ApiResourceType::Config
        | ApiResourceType::Resourcelinks
        | ApiResourceType::Rules
        | ApiResourceType::Scenes
        | ApiResourceType::Schedules
        | ApiResourceType::Sensors
        | ApiResourceType::Capabilities => Err(ApiError::V1CreateUnsupported(resource)),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(post_api))
        .route("/config", get(get_api_config))
        .route("/:username", get(get_api_user))
        .route(
            "/:username/:resource",
            get(get_api_user_resource).put(put_api_user_resource),
        )
        .route("/:username/:resource/:id", get(get_api_user_resource_id))
        .route(
            "/:username/:resource/:id/:state",
            put(put_api_user_resource_id_state),
        )
}
