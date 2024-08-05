use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures::SinkExt;
use hyper::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::hue::v2::{
    GroupedLightUpdate, On, Resource, ResourceType, V2Reply,
};
use crate::state::AppState;
use crate::z2m::{self, api::DeviceUpdate};

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

async fn get_resource(
    State(state): State<AppState>,
    Path(rtype): Path<ResourceType>,
) -> ApiV2Result {
    V2Reply::list(state.get_resources_by_type(rtype).await)
}

async fn post_resource(
    State(state): State<AppState>,
    Path(rtype): Path<ResourceType>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    log::info!("POST: {rtype:?} {}", serde_json::to_string(&req)?);
    let obj = Resource::from_value(rtype, req);
    if obj.is_err() {
        log::error!("{:?}", obj);
    }

    let link = state.res.lock().await.add_resource(obj?)?;

    V2Reply::ok(link)
}

#[allow(clippy::option_if_let_else)]
async fn get_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
) -> ApiV2Result {
    V2Reply::ok(state.get_resource(rtype, &id).await?)
}

async fn put_resource_id(
    State(state): State<AppState>,
    Path((rtype, id)): Path<(ResourceType, Uuid)>,
    Json(put): Json<Value>,
) -> ApiV2Result {
    log::info!("PUT {rtype:?}/{id}: {put:?}");

    let res = state.get_resource(rtype, &id).await?;
    match res.obj {
        Resource::Light(obj) => {
            let upd: GroupedLightUpdate = serde_json::from_value(put)?;

            let mut payload = HashMap::new();

            match upd.on {
                Some(On { on: true }) => {
                    payload.insert("state".to_string(), json!("ON"));
                }
                Some(On { on: false }) => {
                    payload.insert("state".to_string(), json!("OFF"));
                }
                None => {}
            }

            if let Some(dim) = upd.dimming {
                payload.insert(
                    "brightness".to_string(),
                    json!(dim.brightness / 100.0 * 255.0),
                );
            }

            if let Some(col) = upd.color {
                payload.insert("color".to_string(), json!(col.xy));
            }

            if let Some(ct) = upd.color_temperature {
                payload.insert("color_temp".to_string(), json!(ct.mirek));
            }

            let api_req = z2m::api::Other {
                topic: format!("{}/set", obj.metadata.name),
                payload: serde_json::to_value(payload)?,
            };

            state
                .ws
                .lock()
                .await
                .send(Message::Text(serde_json::to_string(&api_req)?))
                .await?;
            log::info!("{api_req:#?}");
        }

        Resource::GroupedLight(obj) => {
            log::info!("PUT {rtype:?}/{id}: updating");

            let Resource::Room(rr) = state
                .get_resource(obj.owner.rtype, &obj.owner.rid)
                .await?
                .obj
            else {
                return Err(ApiError::NotFound(obj.owner.rid));
            };

            let upd: GroupedLightUpdate = serde_json::from_value(put)?;

            let payload = DeviceUpdate::default()
                .with_state(upd.on.map(|on| on.on))
                .with_brightness(upd.dimming.map(|dim| dim.brightness / 100.0 * 255.0))
                .with_color_temp(upd.color_temperature.map(|ct| ct.mirek))
                .with_color_xy(upd.color.map(|col| col.xy));

            let api_req = z2m::api::Other {
                topic: format!("{}/set", rr.metadata.name),
                payload: serde_json::to_value(payload)?,
            };

            state
                .ws
                .lock()
                .await
                .send(Message::Text(serde_json::to_string(&api_req)?))
                .await?;
            log::info!("{api_req:#?}");
        }
        _ => {
            log::warn!("PUT {rtype:?}/{id}: state update not supported");
        }
    }

    V2Reply::ok(state.get_resource(rtype, &id).await?)
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
        .route("/:resource", get(get_resource).post(post_resource))
        .route(
            "/:resource/:id",
            get(get_resource_id)
                .put(put_resource_id)
                .delete(delete_resource_id),
        )
}
