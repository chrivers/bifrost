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

use crate::hue::api::{Device, GroupedLight, Light, RType, ResourceLink, Room, Scene, V1Reply};
use crate::hue::legacy_api::{
    ApiGroup, ApiLight, ApiLightStateUpdate, ApiResourceType, ApiScene, ApiUserConfig,
    Capabilities, HueResult, NewUser, NewUserReply,
};
use crate::resource::Resources;
use crate::server::appstate::AppState;
use crate::z2m::request::ClientRequest;
use crate::z2m::update::DeviceUpdate;
use crate::{
    error::{ApiError, ApiResult},
    hue::legacy_api::ApiGroupActionUpdate,
};

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
            res.get_id_v1(rr.id)?,
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
        let lights: Vec<String> = room
            .children
            .iter()
            .filter_map(|rl| res.get(rl).ok())
            .filter_map(Device::light_service)
            .filter_map(|rl| res.get_id_v1(rl.rid).ok())
            .collect();

        rooms.insert(
            res.get_id_v1(rr.id)?,
            ApiGroup::from_lights_and_room(glight, lights, room),
        );
    }

    Ok(rooms)
}

fn get_scenes(owner: &Uuid, res: &MutexGuard<Resources>) -> ApiResult<HashMap<String, ApiScene>> {
    let mut scenes = HashMap::new();

    for rr in res.get_resources_by_type(RType::Scene) {
        let scene: Scene = rr.obj.try_into()?;

        scenes.insert(
            res.get_id_v1(rr.id)?,
            ApiScene::from_scene(res, *owner, scene)?,
        );
    }

    Ok(scenes)
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

async fn post_api_user_resource(
    Path((_username, resource)): Path<(Uuid, ApiResourceType)>,
    Json(req): Json<Value>,
) -> ApiResult<Json<Value>> {
    warn!("POST v1 user resource unsupported");
    warn!("Request: {req:?}");
    Err(ApiError::V1CreateUnsupported(resource))
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
    Path((username, resource, id)): Path<(Uuid, ApiResourceType, u32)>,
) -> ApiResult<impl IntoResponse> {
    log::debug!("GET v1 username={username} resource={resource:?} id={id}");
    match resource {
        ApiResourceType::Lights => {
            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Light);
            let light = lock.get::<Light>(&link)?;
            let dev = lock.get::<Device>(&light.owner)?.clone();
            Ok(Json(json!(ApiLight::from_dev_and_light(
                &uuid,
                dev,
                light.clone(),
            ))))
        }
        ApiResourceType::Scenes => {
            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Scene);
            let scene = lock.get::<Scene>(&link)?.clone();
            Ok(Json(json!(ApiScene::from_scene(&lock, username, scene)?)))
        }
        ApiResourceType::Groups => {
            let lock = state.res.lock().await;
            let groups = get_groups(&lock)?;
            let group = groups
                .get(&id.to_string())
                .ok_or(ApiError::V1NotFound(id))?;
            Ok(Json(json!(group)))
        }
        _ => Err(ApiError::V1NotFound(id)),
    }
}

async fn put_api_user_resource_id(
    State(state): State<AppState>,
    Path((_username, resource, id, path)): Path<(String, ApiResourceType, u32, String)>,
    Json(req): Json<Value>,
) -> ApiResult<Json<Value>> {
    match resource {
        ApiResourceType::Lights => {
            log::debug!("req: {}", serde_json::to_string_pretty(&req)?);
            if path != "state" {
                return Err(ApiError::V1NotFound(id))?;
            }

            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Light);
            let upd: ApiLightStateUpdate = serde_json::from_value(req)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on)
                .with_brightness(upd.bri.map(f64::from))
                .with_color_xy(upd.xy.map(Into::into))
                .with_color_temp(upd.ct);

            lock.z2m_request(ClientRequest::light_update(link, payload))?;
            drop(lock);

            let reply = V1Reply::for_light(id, &path).with_light_state_update(&upd)?;

            Ok(Json(reply.json()))
        }
        ApiResourceType::Groups => {
            log::debug!("req: {}", serde_json::to_string_pretty(&req)?);
            if path != "action" {
                return Err(ApiError::V1NotFound(id))?;
            }

            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Room);
            let room: &Room = lock.get(&link)?;
            let glight = room.grouped_light_service().unwrap();

            let upd: ApiGroupActionUpdate = serde_json::from_value(req)?;

            let reply = match upd {
                ApiGroupActionUpdate::LightUpdate(upd) => {
                    let payload = DeviceUpdate::default()
                        .with_state(upd.on)
                        .with_brightness(upd.bri.map(f64::from))
                        .with_color_xy(upd.xy.map(Into::into))
                        .with_color_temp(upd.ct);

                    lock.z2m_request(ClientRequest::group_update(*glight, payload))?;
                    drop(lock);

                    V1Reply::for_group(id, &path).with_light_state_update(&upd)?
                }
                ApiGroupActionUpdate::GroupUpdate(upd) => {
                    let scene_id = upd.scene.parse()?;
                    let scene_uuid = lock.from_id_v1(scene_id)?;
                    let rlink = RType::Scene.link_to(scene_uuid);
                    lock.z2m_request(ClientRequest::scene_recall(rlink))?;
                    drop(lock);

                    V1Reply::for_group(id, &path).add("scene", upd.scene)?
                }
            };

            Ok(Json(reply.json()))
        }
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
        .route("/:user", get(get_api_user))
        .route("/:user/:rtype", get(get_api_user_resource))
        .route("/:user/:rtype", post(post_api_user_resource))
        .route("/:user/:rtype", put(put_api_user_resource))
        .route("/:user/:rtype/:id", get(get_api_user_resource_id))
        .route("/:user/:rtype/:id/:key", put(put_api_user_resource_id))
}
