use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::server::appstate::AppState;

async fn packages() -> Json<Value> {
    Json(json!([]))
}

async fn hardcoded() -> Json<Value> {
    Json(json!([]))
}

async fn rust_packages() -> Json<Value> {
    Json(json!([]))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/packages.json", get(packages))
        .route("/hardcoded.json", get(hardcoded))
        .route("/rust-packages.json", get(rust_packages))
}
