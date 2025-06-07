use std::collections::{BTreeMap, HashMap};

use axum::Router;
use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use bytes::Bytes;
use chrono::Utc;
use log::{info, warn};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::MutexGuard;

use bifrost_api::backend::BackendRequest;
use hue::api::{
    Device, Entertainment, EntertainmentConfiguration, EntertainmentConfigurationAction,
    EntertainmentConfigurationLocationsNew, EntertainmentConfigurationMetadata,
    EntertainmentConfigurationNew, EntertainmentConfigurationServiceLocationsNew,
    EntertainmentConfigurationType, EntertainmentConfigurationUpdate, GroupedLight,
    GroupedLightUpdate, Light, LightUpdate, RType, ResourceLink, Room, Scene, SceneActive,
    SceneStatus, SceneUpdate, V1Reply,
};
use hue::error::{HueApiV1Error, HueError, HueResult};
use hue::legacy_api::{
    ApiConfigUpdate, ApiGroup, ApiGroupAction, ApiGroupActionUpdate, ApiGroupClass, ApiGroupNew,
    ApiGroupState, ApiGroupType, ApiGroupUpdate2, ApiLight, ApiLightStateUpdate, ApiResourceType,
    ApiScene, ApiSceneAppData, ApiSceneType, ApiSceneVersion, ApiSensor, ApiUserConfig,
    Capabilities, HueApiResult, NewUser, NewUserReply,
};

use crate::error::{ApiError, ApiResult};
use crate::resource::Resources;
use crate::routes::auth::{STANDARD_APPLICATION_ID, STANDARD_CLIENT_KEY};
use crate::routes::clip::entertainment_configuration::{self, POSITIONS};
use crate::routes::extractor::Json;
use crate::routes::{ApiV1Error, ApiV1Result};
use crate::server::appstate::AppState;

async fn get_api_config(State(state): State<AppState>) -> Json<impl Serialize> {
    Json(state.api_short_config().await)
}

async fn post_api(bytes: Bytes) -> ApiV1Result<Json<impl Serialize>> {
    info!("post: {bytes:?}");
    let json: NewUser = serde_json::from_slice(&bytes)?;

    let res = NewUserReply {
        clientkey: if json.generateclientkey {
            Some(hex::encode_upper(STANDARD_CLIENT_KEY))
        } else {
            None
        },
        username: STANDARD_APPLICATION_ID.to_string(),
    };
    Ok(Json(vec![HueApiResult::Success(res)]))
}

fn get_lights(res: &MutexGuard<Resources>) -> ApiResult<HashMap<String, ApiLight>> {
    let mut lights = HashMap::new();

    for rr in res.get_resources_by_type(RType::Light) {
        let light: Light = rr.obj.try_into()?;
        let dev = res.get::<Device>(&light.owner)?;
        lights.insert(
            res.get_id_v1(rr.id)?,
            ApiLight::from_dev_and_light(&rr.id, dev, &light),
        );
    }

    Ok(lights)
}

fn get_groups(res: &MutexGuard<Resources>, group_0: bool) -> ApiResult<HashMap<String, ApiGroup>> {
    let mut rooms = HashMap::new();

    if group_0 {
        rooms.insert("0".into(), ApiGroup::make_group_0());
    }

    for rr in res.get_resources_by_type(RType::Room) {
        let room: Room = rr.obj.try_into()?;
        let uuid = room
            .services
            .iter()
            .find(|rl| rl.rtype == RType::GroupedLight)
            .ok_or(HueError::NotFound(rr.id))?;

        let glight = res.get::<GroupedLight>(uuid)?;
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

    for rr in res.get_resources_by_type(RType::EntertainmentConfiguration) {
        let entconf: EntertainmentConfiguration = rr.obj.try_into()?;

        let mut locations = BTreeMap::<String, Vec<f64>>::new();

        for sl in &entconf.locations.service_locations {
            let ent = res.get::<Entertainment>(&sl.service)?;
            let dev = res.get::<Device>(&ent.owner)?;
            let light_link = dev
                .light_service()
                .ok_or(HueError::NotFound(ent.owner.rid))?;

            let idx = res.get_id_v1_index(light_link.rid)?;
            locations.insert(
                idx.to_string(),
                vec![sl.position.x, sl.position.y, sl.position.z],
            );
        }

        let class = match entconf.configuration_type {
            EntertainmentConfigurationType::Screen => ApiGroupClass::TV,
            EntertainmentConfigurationType::Monitor => ApiGroupClass::Computer,
            EntertainmentConfigurationType::Music => ApiGroupClass::Music,
            // FIXME: what does Space3D map to?
            EntertainmentConfigurationType::Space3D | EntertainmentConfigurationType::Other => {
                ApiGroupClass::Other
            }
        };

        rooms.insert(
            res.get_id_v1(rr.id)?,
            ApiGroup {
                name: entconf.metadata.name.clone(),
                lights: locations.keys().cloned().collect(),
                locations: json!(locations),
                action: ApiGroupAction::default(),
                class,
                group_type: ApiGroupType::Entertainment,
                recycle: false,
                sensors: vec![],
                state: ApiGroupState::default(),
                stream: json!({
                    "active": entconf.active_streamer.is_some(),
                    "owner": entconf.active_streamer.map(|_st| STANDARD_APPLICATION_ID.to_string()),
                    "proxymode": "auto",
                    "proxynode": "/bridge"
                }),
            },
        );
    }

    Ok(rooms)
}

pub fn get_scene(res: &Resources, owner: String, scene: &Scene) -> ApiV1Result<ApiScene> {
    let lights = scene
        .actions
        .iter()
        .map(|sae| res.get_id_v1(sae.target.rid))
        .collect::<HueResult<_>>()?;

    let lightstates = scene
        .actions
        .iter()
        .map(|sae| {
            Ok((
                res.get_id_v1(sae.target.rid)?,
                ApiLightStateUpdate::from(sae.action.clone()),
            ))
        })
        .collect::<ApiV1Result<_>>()?;

    let room_id = res.get_id_v1_index(scene.group.rid)?;

    Ok(ApiScene {
        name: scene.metadata.name.clone(),
        scene_type: ApiSceneType::GroupScene,
        lights,
        lightstates,
        owner,
        recycle: false,
        locked: false,
        /* Some clients (e.g. Hue Essentials) require .appdata */
        appdata: ApiSceneAppData {
            data: Some(format!("xxxxx_r{room_id}")),
            version: Some(1),
        },
        picture: String::new(),
        lastupdated: Utc::now(),
        version: ApiSceneVersion::V2 as u32,
        image: scene.metadata.image.map(|rl| rl.rid),
        group: Some(room_id.to_string()),
    })
}

fn get_scenes(owner: &str, res: &MutexGuard<Resources>) -> ApiV1Result<HashMap<String, ApiScene>> {
    let mut scenes = HashMap::new();

    for rr in res.get_resources_by_type(RType::Scene) {
        let scene = &rr.obj.try_into()?;

        scenes.insert(
            res.get_id_v1(rr.id)?,
            get_scene(res, owner.to_string(), scene)?,
        );
    }

    Ok(scenes)
}

#[allow(clippy::zero_sized_map_values)]
async fn get_api_user(
    state: State<AppState>,
    Path(username): Path<String>,
) -> ApiV1Result<Json<impl Serialize>> {
    let lock = state.res.lock().await;

    Ok(Json(ApiUserConfig {
        config: state.api_config(username.clone()).await?,
        groups: get_groups(&lock, false)?,
        lights: get_lights(&lock)?,
        resourcelinks: HashMap::new(),
        rules: HashMap::new(),
        scenes: get_scenes(&username, &lock)?,
        schedules: HashMap::new(),
        sensors: HashMap::from([(1, ApiSensor::builtin_daylight_sensor())]),
    }))
}

async fn get_api_user_resource(
    State(state): State<AppState>,
    Path((username, artype)): Path<(String, ApiResourceType)>,
) -> ApiV1Result<Json<Value>> {
    let lock = &state.res.lock().await;
    match artype {
        ApiResourceType::Config => Ok(Json(json!(state.api_config(username).await?))),
        ApiResourceType::Lights => Ok(Json(json!(get_lights(lock)?))),
        ApiResourceType::Groups => Ok(Json(json!(get_groups(lock, false)?))),
        ApiResourceType::Scenes => Ok(Json(json!(get_scenes(&username, lock)?))),
        ApiResourceType::Resourcelinks
        | ApiResourceType::Rules
        | ApiResourceType::Schedules
        | ApiResourceType::Sensors => Ok(Json(json!({}))),
        ApiResourceType::Capabilities => Ok(Json(json!(Capabilities::new()))),
    }
}

fn lights_v1_to_ec_locations(
    lights: &[String],
    res: &Resources,
) -> ApiResult<EntertainmentConfigurationLocationsNew> {
    let mut service_locations = vec![];

    let mut positions = POSITIONS.iter().cycle();

    for id in lights {
        let light_uuid = res.from_id_v1(id.parse().map_err(ApiError::ParseIntError)?)?;
        let light = res.get_id::<Light>(light_uuid)?;
        let device = res.get::<Device>(&light.owner)?;

        // FIXME: not the best error mapping
        let ent_svc = device
            .entertainment_service()
            .ok_or(HueError::NotFound(light_uuid))?;

        service_locations.push(EntertainmentConfigurationServiceLocationsNew {
            positions: vec![positions.next().unwrap().clone()],
            service: *ent_svc,
        });
    }

    Ok(EntertainmentConfigurationLocationsNew { service_locations })
}

async fn post_api_user_resource(
    state: State<AppState>,
    Path((_username, resource)): Path<(String, ApiResourceType)>,
    Json(req): Json<Value>,
) -> ApiV1Result<Json<Value>> {
    // FIXME: these are copied from entertainment_configuration

    // We only know how to create entertainment groups
    let ApiResourceType::Groups = resource else {
        warn!("POST v1 user resource unsupported");
        warn!("Request: {req:?}");
        return Err(ApiV1Error::V1CreateUnsupported(resource));
    };

    let group_create: ApiGroupNew = serde_json::from_value(req)?;
    info!("Create group request: {group_create:?}");

    if group_create.group_type != ApiGroupType::Entertainment {
        return Err(ApiV1Error::V1CreateUnsupported(resource));
    }

    let lock = state.res.lock().await;

    let locations = lights_v1_to_ec_locations(&group_create.lights, &lock)?;

    let ecnew = EntertainmentConfigurationNew {
        configuration_type: EntertainmentConfigurationType::Screen,
        metadata: EntertainmentConfigurationMetadata {
            name: group_create
                .name
                .unwrap_or_else(|| String::from("Entertainment area")),
        },
        stream_proxy: None,
        locations,
    };

    log::debug!("Converted to V2 create request: {ecnew:?}");
    drop(lock);

    let mut resp =
        entertainment_configuration::post_resource(&state, serde_json::to_value(ecnew)?).await?;

    // FIXME: ugly unpacking/repacking of post_resource result
    if let Some(data) = resp.0.data.pop() {
        let rlink: ResourceLink = serde_json::from_value(data)?;

        let id = state.res.lock().await.get_id_v1_index(rlink.rid)?;

        let response = json!([{"success": {"id": id}}]);

        log::info!("Success: created {id} ({})", rlink.rid);
        Ok(Json(response))
    } else {
        Err(ApiV1Error::V1CreateUnsupported(resource))
    }
}

async fn put_api_user_resource(
    State(state): State<AppState>,
    Path((_username, resource)): Path<(String, ApiResourceType)>,
    Json(req): Json<Value>,
) -> ApiV1Result<Json<impl Serialize>> {
    let result = match resource {
        ApiResourceType::Config => {
            let upd: ApiConfigUpdate = serde_json::from_value(req.clone())?;
            let mut config = (*state.config()).clone();

            if let Some(timezone) = upd.timezone {
                config.bridge.timezone = timezone;
            }

            // todo: persist config
            state.replace_config(config);

            vec![HueApiResult::Success(req)]
        }
        resource => {
            warn!("PUT v1 user resource {resource:?} {req:?}");
            vec![HueApiResult::Success(req)]
        }
    };
    Ok(Json(result))
}

#[allow(clippy::significant_drop_tightening)]
async fn get_api_user_resource_id(
    State(state): State<AppState>,
    Path((username, resource, id)): Path<(String, ApiResourceType, u32)>,
) -> ApiV1Result<Json<impl Serialize>> {
    log::debug!("GET v1 username={username} resource={resource:?} id={id}");
    let result = match resource {
        ApiResourceType::Lights => {
            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Light);
            let light = lock.get::<Light>(&link)?;
            let dev = lock.get::<Device>(&light.owner)?;

            json!(ApiLight::from_dev_and_light(&uuid, dev, light))
        }
        ApiResourceType::Scenes => {
            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Scene);
            let scene = lock.get::<Scene>(&link)?;

            json!(get_scene(&lock, username, scene)?)
        }
        ApiResourceType::Groups => {
            let lock = state.res.lock().await;
            let groups = get_groups(&lock, true)?;
            let group = groups
                .get(&id.to_string())
                .ok_or(HueError::V1NotFound(id))?;

            json!(group)
        }
        _ => Err(HueError::V1NotFound(id))?,
    };

    Ok(Json(result))
}

#[allow(clippy::significant_drop_tightening, clippy::single_match)]
async fn put_api_user_resource_id(
    State(state): State<AppState>,
    Path((username, artype, id)): Path<(String, ApiResourceType, u32)>,
    Json(req): Json<Value>,
) -> ApiV1Result<Json<Value>> {
    log::debug!("PUT v1 username={username} resource={artype:?} id={id}");
    log::debug!("JSON: {req:?}");
    match artype {
        ApiResourceType::Groups => {
            let upd: ApiGroupUpdate2 = serde_json::from_value(req)?;

            let mut v1res = V1Reply::for_group(id);

            let mut ecupd = EntertainmentConfigurationUpdate::new();

            let lock = state.res.lock().await;

            let uuid = lock.from_id_v1(id)?;

            ecupd.action = upd.stream.map(|stream| {
                if stream.active {
                    EntertainmentConfigurationAction::Start
                } else {
                    EntertainmentConfigurationAction::Stop
                }
            });

            if let Some(lights) = &upd.lights {
                ecupd.locations = Some(lights_v1_to_ec_locations(lights, &lock)?.into());
            }

            drop(lock);

            let rlink = RType::EntertainmentConfiguration.link_to(uuid);

            let resp = entertainment_configuration::put_resource_id(
                &state,
                rlink,
                serde_json::to_value(&ecupd)?,
            )
            .await?;

            if !resp.0.errors.is_empty() {
                Err(HueApiV1Error::BridgeInternalError)?;
            }

            if let Some(stream) = &upd.stream {
                v1res = v1res.add("stream/active", stream.active)?;
            }

            Ok(Json(v1res.json()))
        }
        ApiResourceType::Config
        | ApiResourceType::Lights
        | ApiResourceType::Resourcelinks
        | ApiResourceType::Rules
        | ApiResourceType::Scenes
        | ApiResourceType::Schedules
        | ApiResourceType::Sensors
        | ApiResourceType::Capabilities => Err(ApiV1Error::V1CreateUnsupported(artype)),
    }
}

async fn put_api_user_resource_id_path(
    State(state): State<AppState>,
    Path((_username, artype, id, path)): Path<(String, ApiResourceType, u32, String)>,
    Json(req): Json<Value>,
) -> ApiV1Result<Json<Value>> {
    match artype {
        ApiResourceType::Lights => {
            log::debug!("req: {}", serde_json::to_string_pretty(&req)?);
            if path != "state" {
                return Err(HueError::V1NotFound(id))?;
            }

            let lock = state.res.lock().await;
            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Light);
            let updv1: ApiLightStateUpdate = serde_json::from_value(req)?;

            let upd = LightUpdate::from(&updv1);

            lock.backend_request(BackendRequest::LightUpdate(link, upd))?;
            drop(lock);

            let reply = V1Reply::for_light(id, &path).with_light_state_update(&updv1)?;

            Ok(Json(reply.json()))
        }

        /* handle groups, exceot for group 0 ("all groups") */
        ApiResourceType::Groups if id != 0 => {
            if path != "action" {
                return Err(HueError::V1NotFound(id))?;
            }

            let lock = state.res.lock().await;

            let uuid = lock.from_id_v1(id)?;
            let link = ResourceLink::new(uuid, RType::Room);

            let room: &Room = lock.get(&link)?;
            let glight = room.grouped_light_service().unwrap();

            let updv1: ApiGroupActionUpdate = serde_json::from_value(req)?;

            let reply = match updv1 {
                ApiGroupActionUpdate::LightUpdate(upd) => {
                    let updv2 = GroupedLightUpdate::from(&upd);

                    lock.backend_request(BackendRequest::GroupedLightUpdate(*glight, updv2))?;
                    drop(lock);

                    V1Reply::for_group_path(id, &path).with_light_state_update(&upd)?
                }
                ApiGroupActionUpdate::GroupUpdate(upd) => {
                    let scene_id = upd.scene.parse().map_err(ApiError::ParseIntError)?;
                    let scene_uuid = lock.from_id_v1(scene_id)?;
                    let rlink = RType::Scene.link_to(scene_uuid);
                    let updv2 = SceneUpdate::new().with_recall_action(Some(SceneStatus {
                        active: SceneActive::Static,
                        last_recall: None,
                    }));
                    lock.backend_request(BackendRequest::SceneUpdate(rlink, updv2))?;
                    drop(lock);

                    V1Reply::for_group_path(id, &path).add("scene", upd.scene)?
                }
            };

            Ok(Json(reply.json()))
        }

        /* handle group 0 ("all groups") */
        ApiResourceType::Groups => {
            if path != "action" {
                return Err(HueError::V1NotFound(id))?;
            }

            let lock = state.res.lock().await;

            let updv1: ApiGroupActionUpdate = serde_json::from_value(req)?;

            let reply = match updv1 {
                ApiGroupActionUpdate::LightUpdate(upd) => {
                    let updv2 = GroupedLightUpdate::from(&upd);

                    for res in lock.get_resources_by_type(RType::GroupedLight) {
                        let link = RType::GroupedLight.link_to(res.id);
                        let req = BackendRequest::GroupedLightUpdate(link, updv2.clone());
                        lock.backend_request(req)?;
                    }

                    drop(lock);

                    V1Reply::for_group_path(id, &path).with_light_state_update(&upd)?
                }
                ApiGroupActionUpdate::GroupUpdate(_api_group_update) => {
                    return Err(HueError::V1NotFound(id))?;
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
        | ApiResourceType::Capabilities => Err(ApiV1Error::V1CreateUnsupported(artype)),
    }
}

/// This generates a workaround necessary for iConnectHue (iPhone app)
///
/// For some reason, iConnectHue has been observed to try the endpoint GET /api/newUser,
/// even though this does not seem to ever have been a valid hue endpoint.
///
/// 2025-01-24: This response has been confirmed to work by Alexa and Peter Miller on discord.
pub async fn workaround_iconnect_hue() -> ApiV1Result<()> {
    Err(HueApiV1Error::UnauthorizedUser)?
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(post_api))
        .route("/config", get(get_api_config))
        .route("/nouser/config", get(get_api_config))
        .route("/newUser", get(workaround_iconnect_hue))
        .route("/{user}", get(get_api_user))
        .route("/{user}/{rtype}", get(get_api_user_resource))
        .route("/{user}/{rtype}", post(post_api_user_resource))
        .route("/{user}/{rtype}", put(put_api_user_resource))
        .route("/{user}/{rtype}/{id}", get(get_api_user_resource_id))
        .route("/{user}/{rtype}/{id}", put(put_api_user_resource_id))
        .route(
            "/{user}/{rtype}/{id}/{key}",
            put(put_api_user_resource_id_path),
        )
}
