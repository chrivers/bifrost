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

use std::fs::File;
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
use tokio::task::JoinSet;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use bifrost::config;
use bifrost::error::ApiResult;
use bifrost::mdns;
use bifrost::routes;
use bifrost::state::AppState;
use bifrost::z2m;

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

async fn http_server(
    listen_addr: Ipv4Addr,
    svc: impl MakeService<SocketAddr, Request<Incoming>>,
) -> ApiResult<()> {
    let addr = SocketAddr::from((listen_addr, 80));
    log::info!("http listening on {}", addr);

    axum_server::bind(addr).serve(svc).await?;

    Ok(())
}

async fn https_server(
    listen_addr: Ipv4Addr,
    svc: impl MakeService<SocketAddr, Request<Incoming>>,
) -> ApiResult<()> {
    let config = RustlsConfig::from_pem_file("cert.pem", "cert.pem").await?;

    let addr = SocketAddr::from((listen_addr, 443));
    log::info!("https listening on {}", addr);

    axum_server::bind_rustls(addr, config).serve(svc).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    colog::init();

    let config = config::parse("config.yaml")?;

    let appstate = AppState::new(config);
    if let Ok(fd) = File::open("state.yaml") {
        appstate.res.lock().await.load(fd)?;
    } else {
        appstate.res.lock().await.init(&appstate.bridge_id())?;
    }

    log::info!("Serving mac [{}]", appstate.mac());

    let _mdns = mdns::register_mdns(&appstate);

    let ip = appstate.ip();

    let normalized = NormalizePathLayer::trim_trailing_slash().layer(router(appstate.clone()));
    let svc = ServiceExt::<Request>::into_make_service(normalized);

    let mut tasks = JoinSet::new();

    tasks.spawn(http_server(ip, svc.clone()));
    tasks.spawn(https_server(ip, svc));

    for (name, server) in &appstate.z2m_config().servers {
        log::info!("Connecting to [{name}]: {}", server.url);
        let client = z2m::Client::new(&server.url, appstate.clone()).await?;

        tasks.spawn(client.event_loop());
    }

    while let Some(res) = tasks.join_next().await {
        let _ = res?;
    }

    Ok(())
}
