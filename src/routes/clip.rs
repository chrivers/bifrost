use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use hyper::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::api::{
    GroupedLightUpdate, LightUpdate, RType, Resource, ResourceLink, Room, Scene, SceneRecall,
    SceneStatus, SceneStatusUpdate, SceneUpdate, V2Reply,
};
use crate::resource::AuxData;
use crate::state::AppState;
use crate::z2m::update::DeviceUpdate;

type ApiV2Result = ApiResult<Json<V2Reply<Value>>>;

impl<T: Serialize> V2Reply<T> {
    #[allow(clippy::unnecessary_wraps)]
    fn ok(obj: T) -> ApiV2Result {
        Ok(Json(V2Reply {
            data: vec![serde_json::to_value(obj)?],
            errors: vec![],
        }))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn list(data: Vec<T>) -> ApiV2Result {
        Ok(Json(V2Reply {
            data: data
                .into_iter()
                .map(|e| serde_json::to_value(e))
                .collect::<Result<_, _>>()?,
            errors: vec![],
        }))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let error_msg = format!("{self}");
        log::error!("Request failed: {error_msg}");
        let res = Json(V2Reply::<Value> {
            data: vec![],
            errors: vec![error_msg],
        });

        let status = match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Full(_) => StatusCode::INSUFFICIENT_STORAGE,
            Self::WrongType(_, _) => StatusCode::NOT_ACCEPTABLE,
            Self::DeleteDenied(_) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, res).into_response()
    }
}

async fn get_root(State(state): State<AppState>) -> impl IntoResponse {
    Json(V2Reply {
        data: state.res.lock().await.get_resources(),
        errors: vec![],
    })
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

    let link = match obj {
        Resource::Scene(scn) => {
            let room = lock.get::<Room>(&scn.group)?;

            let sid = lock.get_next_scene_id(&scn.group)?;
            println!("NEXT: {sid}");

            let payload = json!({
                "scene_store": {
                    "ID": sid,
                    "name": scn.metadata.name,
                }
            });

            let link_scene = RType::Scene.deterministic((scn.group.rid, sid));

            log::info!("New scene: {link_scene:?} ({})", scn.metadata.name);

            lock.aux_set(
                &link_scene,
                AuxData::new()
                    .with_topic(&scn.metadata.name)
                    .with_index(sid),
            );

            lock.z2m_send_set(&room.metadata.name, payload)?;

            lock.add(&link_scene, Resource::Scene(scn))?;
            drop(lock);

            link_scene
        }

        obj => {
            let rlink = ResourceLink::new(Uuid::new_v4(), obj.rtype());
            lock.add(&rlink, obj)?;
            drop(lock);

            rlink
        }
    };

    V2Reply::ok(link)
}

#[allow(clippy::option_if_let_else)]
async fn get_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
) -> ApiV2Result {
    V2Reply::ok(state.res.lock().await.get_resource(rtype, &id)?)
}

async fn put_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT {rtype:?}/{id}: {put:?}");

    let rlink = rtype.link_to(id);
    let mut lock = state.res.lock().await;
    let res = lock.get_resource(rtype, &id)?;
    match res.obj {
        Resource::Light(obj) => {
            let upd: LightUpdate = serde_json::from_value(put)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on.map(|on| on.on))
                .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 255.0))
                .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
                .with_color_xy(upd.color.map(|col| col.xy));

            lock.z2m_send_set(&obj.metadata.name, payload)?;
        }

        Resource::GroupedLight(obj) => {
            log::info!("PUT {rtype:?}/{id}: updating");

            let rr: Room = lock.get(&obj.owner)?;

            let upd: GroupedLightUpdate = serde_json::from_value(put)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on.map(|on| on.on))
                .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 255.0))
                .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
                .with_color_xy(upd.color.map(|col| col.xy));

            lock.z2m_send_set(&rr.metadata.name, payload)?;
        }

        Resource::Scene(obj) => {
            log::info!("PUT {rtype:?}/{id}: updating");

            let upd: SceneUpdate = serde_json::from_value(put)?;
            log::info!("{upd:#?}");

            if let Some(md) = upd.metadata {
                lock.update(&id, |scn: &mut Scene| {
                    if md.appdata.is_some() {
                        scn.metadata.appdata = md.appdata;
                    }
                    if md.image.is_some() {
                        scn.metadata.image = md.image;
                    }
                    scn.metadata.name = md.name;
                })?;
            }

            match upd.recall {
                Some(SceneRecall {
                    action: Some(SceneStatusUpdate::Active),
                    ..
                }) => {
                    let room = lock.get::<Room>(&obj.group)?;
                    for scn in room.services.iter().filter(|rl| rl.rtype == RType::Scene) {
                        if let Ok(scene) = lock.get::<Scene>(scn) {
                            if scene.status != Some(SceneStatus::Inactive) {
                                lock.update::<Scene>(&scn.rid, |scn| {
                                    scn.status = Some(SceneStatus::Inactive);
                                })?;
                            }
                        }
                    }
                    lock.update(&id, |scn: &mut Scene| {
                        scn.status = Some(SceneStatus::Static);
                    })?;

                    let aux = lock.aux_get(&rlink)?;

                    let topic = aux.topic.as_ref().ok_or(ApiError::AuxNotFound(rlink))?;
                    let payload =
                        json!({"scene_recall": aux.index, "_scene": rlink, "_room": obj.group});

                    lock.z2m_send_set(topic, payload)?;
                    drop(lock);
                }
                Some(recall) => {
                    log::error!("Scene recall type not supported: {recall:?}");
                }
                _ => {}
            }
        }
        _ => {
            log::warn!("PUT {rtype:?}/{id}: state update not supported");
        }
    }

    V2Reply::ok(rlink)
}

async fn delete_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
) -> ApiV2Result {
    log::info!("DELETE {rtype:?}/{id}");
    let link = rtype.link_to(id);

    let mut lock = state.res.lock().await;

    let res = lock.get_resource(rtype, &id)?;
    match res.obj {
        Resource::Scene(obj) => {
            let aux = lock.aux_get(&link)?;

            let topic = aux.topic.as_ref().ok_or(ApiError::NotFound(id))?;
            let index = aux.index.as_ref().ok_or(ApiError::NotFound(id))?;
            let payload = json!({
                "scene_remove": index,
            });

            lock.z2m_send_set(topic, payload)?;

            lock.update::<Room>(&obj.group.rid, |obj| {
                obj.services.retain(|svc| svc.rid != id);
            })?;

            lock.delete(&link)?;
            drop(lock);

            V2Reply::ok(link)
        }
        _ => Err(ApiError::DeleteDenied(id))?,
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_root))
        .route("/:resource", get(get_resource).post(post_resource))
        .route(
            "/:resource/:id",
            get(get_resource_id)
                .put(put_resource_id)
                .delete(delete_resource_id),
        )
}
