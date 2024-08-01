#![warn(
    clippy::all,
    clippy::correctness,
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::perf,
    clippy::style
)]
#![allow(
    clippy::cargo_common_metadata,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::if_not_else,
    clippy::inline_always,
    clippy::many_single_char_names,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
    clippy::similar_names,
    clippy::future_not_send
)]


use std::net::Ipv4Addr;
use std::{net::SocketAddr, time::Duration};

use axum::body::Body;
use axum::extract::Request;
use axum::routing::get;
use axum::ServiceExt;
use axum::{response::Response, Router};
use axum_server::service::MakeService;
use axum_server::tls_rustls::RustlsConfig;

use hyper::body::Incoming;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

pub mod hue;

mod config;
mod mdns;
mod mqtt;
mod routes;
mod state;

use state::AppState;

fn trace_layer_on_response(response: &Response<Body>, latency: Duration, span: &Span) {
    span.record(
        "latency",
        tracing::field::display(format!("{}μs", latency.as_micros())),
    );
    span.record("status", tracing::field::display(response.status()));
}

fn router(appstate: AppState) -> Router<()> {
    Router::new()
        .nest("/api", routes::api::router())
        .nest("/clip/v2/resource", routes::clip::router())
        .route(
            "/eventstream/clip/v2",
            get(routes::eventstream::get_clip_v2),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request| {
                    info_span!(
                        "http",
                        method = ?request.method(),
                        uri = ?request.uri(),
                        status = tracing::field::Empty,
                        /* latency = tracing::field::Empty, */
                    )
                })
                .on_response(trace_layer_on_response),
        )
        .with_state(appstate)
}

async fn http_server(listen_addr: Ipv4Addr, svc: impl MakeService<SocketAddr, Request<Incoming>>) {
    let addr = SocketAddr::from((listen_addr, 80));
    log::info!("http listening on {}", addr);

    axum_server::bind(addr).serve(svc).await.unwrap();
}

async fn https_server(listen_addr: Ipv4Addr, svc: impl MakeService<SocketAddr, Request<Incoming>>) {
    let config = RustlsConfig::from_pem_file("cert.pem", "cert.pem")
        .await
        .unwrap();

    let addr = SocketAddr::from((listen_addr, 443));
    log::info!("https listening on {}", addr);

    axum_server::bind_rustls(addr, config)
        .serve(svc)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    colog::init();

    let config = config::parse("config.yaml").unwrap();

    let mut appstate = AppState::new(config);
    appstate.init().await;

    log::info!("Serving mac [{}]", appstate.mac());

    let _mdns = mdns::register_mdns(&appstate);

    let ip = appstate.ip();

    let normalized = NormalizePathLayer::trim_trailing_slash().layer(router(appstate));
    let svc = ServiceExt::<Request>::into_make_service(normalized);

    let http = tokio::spawn(http_server(ip, svc.clone()));
    let https = tokio::spawn(https_server(ip, svc));

    let _ = tokio::join!(http, https);
}
