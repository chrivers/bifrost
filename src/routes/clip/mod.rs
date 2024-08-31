pub mod generic;
pub mod grouped_light;
pub mod light;
pub mod scene;

use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;

use crate::error::ApiResult;
use crate::hue::api::V2Reply;
use crate::server::appstate::AppState;

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

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/scene", scene::router())
        .nest("/light", light::router())
        .nest("/grouped_light", grouped_light::router())
        .nest("/", generic::router())
}
