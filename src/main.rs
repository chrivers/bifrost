use std::fs::File;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};

use axum::body::Body;
use axum::extract::Request;
use axum::routing::get;
use axum::ServiceExt;
use axum::{response::Response, Router};
use axum_server::service::MakeService;
use axum_server::tls_rustls::RustlsConfig;

use hyper::body::Incoming;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use bifrost::config;
use bifrost::error::ApiResult;
use bifrost::mdns;
use bifrost::resource::Resources;
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

async fn config_writer(res: Arc<Mutex<Resources>>) -> ApiResult<()> {
    let rx = res.lock().await.state_channel();
    loop {
        rx.notified().await;

        log::debug!("Config changed, saving..");

        if let Ok(fd) = File::create("state.yaml.tmp") {
            res.lock().await.write(fd)?;
            std::fs::rename("state.yaml.tmp", "state.yaml")?;
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    let mut builder = pretty_env_logger::formatted_timed_builder();

    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("debug,mdns_sd=off,tower_http::trace::on_request=info");
    }

    builder.try_init()?;

    let config = config::parse("config.yaml")?;

    let appstate = AppState::new(config)?;
    if let Ok(fd) = File::open("state.yaml") {
        appstate.res.lock().await.read(fd)?;
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
    tasks.spawn(config_writer(appstate.res.clone()));

    for (name, server) in &appstate.z2m_config().servers {
        let client = z2m::Client::new(
            name.clone(),
            server.url.clone(),
            appstate.config(),
            appstate.res.clone(),
        )?;
        tasks.spawn(client.run_forever());
    }

    while let Some(res) = tasks.join_next().await {
        let _ = res?;
    }

    Ok(())
}
