use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use itertools::Itertools;
use serde_json::{json, Value};

use crate::server::appstate::AppState;

async fn packages() -> Json<Value> {
    Json(json!([]))
}

async fn hardcoded() -> Json<Value> {
    Json(json!([{
        "Attributions": [],
        "Package": "bifrost",
        "SPDX-License-Identifiers": [
            "GPL-3.0"
        ],
        "SourceLinks": [
            "https://github.com/chrivers/bifrost",
        ],
        "Version": "0.9",
        "Website": "https://github.com/chrivers/bifrost",
        "licenses": {
            "GPL-3.0": "gpl-3.0.txt",
        }
    }]))
}

async fn license() -> impl IntoResponse {
    const LICENSE: &str = include_str!("../../LICENSE");

    let split = LICENSE
        .find("Preamble")
        .expect("License file must have preamble");

    /* a bit of string trickery to make license render nicely in hue app */
    format!(
        "{}{}",
        &LICENSE[..split]
            .split("\n\n ")
            .map(|s| s.replace("\n ", " "))
            .join("\n\n"),
        &LICENSE[split..]
            .split("\n\n  ")
            .map(|s| s.replace("\n    ", "\n").replace('\n', " "))
            .join("\n\n")
    )
}

async fn rust_packages() -> Json<Value> {
    Json(json!([]))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/packages.json", get(packages))
        .route("/hardcoded.json", get(hardcoded))
        .route("/rust-packages.json", get(rust_packages))
        .route("/gpl-3.0.txt", get(license))
}
