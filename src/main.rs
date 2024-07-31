use std::net::Ipv4Addr;
use std::{net::SocketAddr, time::Duration};

use axum::body::Body;
use axum::routing::get;
use axum::ServiceExt;
use axum::{http::Request, response::Response, Router};
use axum_server::service::MakeService;
use axum_server::tls_rustls::RustlsConfig;

use hyper::body::Incoming;
use mac_address::MacAddress;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use crate::state::AppState;

pub mod hue;

mod mdns;
mod mqtt;
mod routes;
mod state;

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
                .make_span_with(|request: &Request<_>| {
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

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of emulated bridge
    #[arg(short, long, default_value_t = String::from("Bifrost"))]
    name: String,

    /// Mac address to use for api results
    #[arg(short, long)]
    mac: MacAddress,

    /// Ip address to listen on
    #[clap(short = 'l', long)]
    ip: Ipv4Addr,
}

#[tokio::main]
async fn main() {
    colog::init();

    let args = Args::parse();

    let appstate = AppState::new(args.mac);

    log::info!("Serving mac [{}]", args.mac);

    let _mdns = mdns::register_mdns(&appstate);

    let normalized = NormalizePathLayer::trim_trailing_slash().layer(router(appstate));
    let svc = ServiceExt::<axum::extract::Request>::into_make_service(normalized);

    let http = tokio::spawn(http_server(args.ip, svc.clone()));
    let https = tokio::spawn(https_server(args.ip, svc));

    let _ = tokio::join!(http, https);
}
