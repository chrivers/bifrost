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

use crate::hue::v2::{RType, Resource, SceneRecallAction, SceneStatus, V2Reply};
use crate::state::AppState;
use crate::z2m::update::DeviceUpdate;
use crate::{
    error::{ApiError, ApiResult},
    hue::v2::ResourceLink,
};
use crate::{
    hue::update::{GroupedLightUpdate, SceneRecall, SceneUpdate},
    resource::AuxData,
};

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
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(V2Reply::<Value> {
                data: vec![],
                errors: vec![error_msg],
            }),
        )
            .into_response()
    }
}

async fn get_root(State(state): State<AppState>) -> impl IntoResponse {
    Json(V2Reply {
        data: state.get_resources().await,
        errors: vec![],
    })
}

async fn get_resource(State(state): State<AppState>, Path(rtype): Path<RType>) -> ApiV2Result {
    V2Reply::list(state.get_resources_by_type(rtype).await)
}

async fn post_resource(
    State(state): State<AppState>,
    Path(rtype): Path<RType>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("POST: {rtype:?} {}", serde_json::to_string(&req)?);
    let obj = Resource::from_value(rtype, req);
    if obj.is_err() {
        log::error!("{:?}", obj);
    }

    let mut lock = state.res.lock().await;

    let link = match obj? {
        Resource::Scene(scn) => {
            let room = lock.get_room(&scn.group.rid)?;

            let sid = lock.get_next_scene_id(&scn.group)?;
            println!("NEXT: {sid}");

            let payload = json!({
                "scene_store": {
                    "ID": sid,
                    "name": scn.metadata.name,
                }
            });

            let link_scene = RType::Scene.deterministic((scn.group.rid, sid));

            lock.aux.insert(
                link_scene.rid,
                AuxData::new()
                    .with_topic(&scn.metadata.name)
                    .with_index(sid),
            );

            state.send_set(&room.metadata.name, payload).await?;

            link_scene
        }

        obj => lock.add_resource(obj)?,
    };

    V2Reply::ok(link)
}

#[allow(clippy::option_if_let_else)]
async fn get_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
) -> ApiV2Result {
    V2Reply::ok(state.get_resource(rtype, &id).await?)
}

async fn put_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(RType, Uuid)>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT {rtype:?}/{id}: {put:?}");

    let res = state.get_resource(rtype, &id).await?;
    match res.obj {
        Resource::Light(obj) => {
            let upd: GroupedLightUpdate = serde_json::from_value(put)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on.map(|on| on.on))
                .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 255.0))
                .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
                .with_color_xy(upd.color.map(|col| col.xy));

            state.send_set(&obj.metadata.name, payload).await?;
        }

        Resource::GroupedLight(obj) => {
            log::info!("PUT {rtype:?}/{id}: updating");

            let Resource::Room(rr) = state.get_link(&obj.owner).await?.obj else {
                return Err(ApiError::NotFound(obj.owner.rid));
            };

            let upd: GroupedLightUpdate = serde_json::from_value(put)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on.map(|on| on.on))
                .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 255.0))
                .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
                .with_color_xy(upd.color.map(|col| col.xy));

            state.send_set(&rr.metadata.name, payload).await?;
        }

        Resource::Scene(_obj) => {
            log::info!("PUT {rtype:?}/{id}: updating");

            let upd: SceneUpdate = serde_json::from_value(put)?;
            log::info!("{upd:#?}");

            match upd.recall {
                Some(SceneRecall {
                    action: Some(SceneRecallAction::Active),
                    ..
                }) => {
                    let mut lock = state.res.lock().await;

                    lock.update_scene(&id, |scn| {
                        scn.status = Some(SceneStatus {
                            active: SceneRecallAction::Static,
                        });
                    })?;

                    let aux = lock.aux.get(&id).ok_or(ApiError::NotFound(id))?;

                    let topic = aux.topic.as_ref().ok_or(ApiError::NotFound(id))?;
                    let payload = json!({"scene_recall": aux.index});

                    state.send_set(topic, payload).await?;
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

    V2Reply::ok(ResourceLink::new(id, rtype))
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
        Resource::Scene(_obj) => {
            let aux = lock.aux.get(&id).ok_or(ApiError::NotFound(id))?;

            let topic = aux.topic.as_ref().ok_or(ApiError::NotFound(id))?;
            let index = aux.index.as_ref().ok_or(ApiError::NotFound(id))?;
            let payload = json!({
                "scene_remove": index,
            });

            state.send_set(topic, payload).await?;

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
