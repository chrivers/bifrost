use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, post, put},
    Json, Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::api::{
    RType, Resource, Scene, SceneStatus, SceneStatusUpdate, SceneUpdate, V2Reply,
};
use crate::model::state::AuxData;
use crate::routes::clip::ApiV2Result;
use crate::server::appstate::AppState;
use crate::z2m::request::ClientRequest;

async fn post_scene(
    State(state): State<AppState>,
    Json(req): Json<Value>,
) -> ApiResult<impl IntoResponse> {
    log::info!("POST: scene {}", serde_json::to_string(&req)?);

    let scene: Scene = serde_json::from_value(req)?;

    let mut lock = state.res.lock().await;

    let sid = lock.get_next_scene_id(&scene.group)?;

    let link_scene = RType::Scene.deterministic((scene.group.rid, sid));

    log::info!("New scene: {link_scene:?} ({})", scene.metadata.name);

    lock.aux_set(
        &link_scene,
        AuxData::new()
            .with_topic(&scene.metadata.name)
            .with_index(sid),
    );

    lock.z2m_request(ClientRequest::scene_store(
        scene.group,
        sid,
        scene.metadata.name.clone(),
    ))?;

    lock.add(&link_scene, Resource::Scene(scene))?;
    drop(lock);

    V2Reply::ok(link_scene)
}

async fn put_scene(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT scene/{id}");
    log::debug!("json data\n{}", serde_json::to_string_pretty(&put)?);

    let rlink = RType::Scene.link_to(id);
    let mut lock = state.res.lock().await;

    log::info!("PUT scene/{id}: updating");

    let upd: SceneUpdate = serde_json::from_value(put)?;

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

    let scene = lock.get::<Scene>(&rlink)?;

    if let Some(recall) = upd.recall {
        if recall.action == Some(SceneStatusUpdate::Active) {
            let scenes = lock.get_scenes_for_room(&scene.group.rid);
            for rid in scenes {
                lock.update(&rid, |scn: &mut Scene| {
                    if rid == id {
                        scn.status = Some(SceneStatus::Static);
                    } else {
                        scn.status = Some(SceneStatus::Inactive);
                    }
                })?;
            }

            lock.z2m_request(ClientRequest::scene_recall(rlink))?;
            drop(lock);
        } else {
            log::error!("Scene recall type not supported: {recall:?}");
        }
    }

    V2Reply::ok(rlink)
}

async fn delete_scene(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiV2Result {
    log::info!("DELETE scene/{id}");
    let link = RType::Scene.link_to(id);

    let lock = state.res.lock().await;
    let res = lock.get_resource(RType::Scene, &id)?;

    match res.obj {
        Resource::Scene(_) => {
            lock.z2m_request(ClientRequest::scene_remove(link))?;

            drop(lock);

            V2Reply::ok(link)
        }
        _ => Err(ApiError::DeleteDenied(id))?,
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(post_scene))
        .route("/:id", put(put_scene))
        .route("/:id", delete(delete_scene))
}
