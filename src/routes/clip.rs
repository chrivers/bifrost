use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::hue::v2::{Resource, ResourceType, Room, Scene, V2Reply};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

type ApiV2Result = ApiResult<Json<V2Reply<Value>>>;

impl<T: Serialize> V2Reply<T> {
    #[allow(clippy::unnecessary_wraps)]
    fn ok(obj: T) -> ApiV2Result {
        Ok(Json(V2Reply {
            data: vec![serde_json::to_value(obj).unwrap()],
            errors: vec![],
        }))
    }

    #[allow(clippy::unnecessary_wraps)]
    fn list(data: Vec<T>) -> ApiV2Result {
        Ok(Json(V2Reply {
            data: data
                .into_iter()
                .map(|e| serde_json::to_value(e).unwrap())
                .collect(),
            errors: vec![],
        }))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        Json(V2Reply::<Value> {
            data: vec![],
            errors: vec![format!("{}", self)],
        })
        .into_response()
    }
}

async fn get_root(State(state): State<AppState>) -> impl IntoResponse {
    Json(V2Reply {
        data: state.get_resources().await,
        errors: vec![],
    })
}

async fn get_resource(State(state): State<AppState>, Path(rtype): Path<ResourceType>) -> ApiV2Result {
    V2Reply::list(state.get_resources_by_type(rtype).await)
}

async fn post_resource(
    State(_state): State<AppState>,
    Path(rtype): Path<ResourceType>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("POST {rtype:?}: {req:?}");
}

async fn get_scene(State(state): State<AppState>) -> ApiV2Result {
    V2Reply::list(state.get_resources_by_type(ResourceType::Scene).await)
}

async fn get_room(State(state): State<AppState>) -> ApiV2Result {
    V2Reply::list(state.get_resources_by_type(ResourceType::Room).await)
}

async fn post_scene(State(state): State<AppState>, Json(req): Json<Value>) -> ApiV2Result {
    log::info!("POST scene: {}", serde_json::to_string(&req).unwrap());
    let scn = serde_json::from_value::<Scene>(req);
    println!("{:?}", &scn);
    let scene = scn.unwrap();

    log::info!("POST scene: {scene:#?}");
    let link = state.res.lock().await.add_scene(scene);

    V2Reply::ok(link)
}

async fn post_room(State(state): State<AppState>, Json(req): Json<Value>) -> ApiV2Result {
    log::info!("POST room: {}", serde_json::to_string(&req).unwrap());
    let scn = serde_json::from_value::<Room>(req);
    println!("{:?}", &scn);
    let mut room = scn.unwrap();
    room.id_v1 = Some("/rooms/v2".to_string());

    log::info!("POST room: {room:#?}");
    let link = state.res.lock().await.add_room(room);

    V2Reply::ok(link)
}

#[allow(clippy::option_if_let_else)]
async fn get_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
) -> ApiV2Result {
    if let Some(res) = state.get_resource(rtype, id).await {
        V2Reply::ok(res)
    } else {
        Err(ApiError::Fail("foo"))
    }
}

async fn put_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
    Json(req): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT {rtype:?}/{id}: {req:?}");

    V2Reply::ok(
        state
            .get_resource(rtype, id)
            .await
            .ok_or(ApiError::Fail("nope"))?,
    )
}

async fn delete_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
) -> ApiV2Result {
    log::info!("DELETE {rtype:?}/{id}");
    let link = rtype.link_to(id);
    state.res.lock().await.delete(&link)?;

    V2Reply::ok(link)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_root))
        .route("/scene", get(get_scene).post(post_scene))
        .route("/room", get(get_room).post(post_room))
        .route("/:resource", get(get_resource).post(post_resource))
        .route(
            "/:resource/:id",
            get(get_resource_id)
                .put(put_resource_id)
                .delete(delete_resource_id),
        )
}
