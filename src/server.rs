use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use axum::routing::IntoMakeService;
use axum::{Router, ServiceExt};
use axum_server::service::MakeService;
use axum_server::tls_rustls::RustlsConfig;

use camino::Utf8PathBuf;
use hyper::body::Incoming;
use tokio::sync::Mutex;
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use crate::error::ApiResult;
use crate::resource::Resources;
use crate::routes;
use crate::state::AppState;

fn trace_layer_on_response(response: &Response<Body>, latency: Duration, span: &Span) {
    span.record(
        "latency",
        tracing::field::display(format!("{}μs", latency.as_micros())),
    );
    span.record("status", tracing::field::display(response.status()));
}

fn router(appstate: AppState) -> Router<()> {
    routes::router(appstate).layer(
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
}

#[must_use]
pub fn build_service(appstate: AppState) -> IntoMakeService<NormalizePath<Router>> {
    let normalized = NormalizePathLayer::trim_trailing_slash().layer(router(appstate));

    ServiceExt::<Request>::into_make_service(normalized)
}

pub async fn http_server<S>(listen_addr: Ipv4Addr, svc: S) -> ApiResult<()>
where
    S: Send + MakeService<SocketAddr, Request<Incoming>>,
    S::MakeFuture: Send,
{
    let addr = SocketAddr::from((listen_addr, 80));
    log::info!("http listening on {}", addr);

    axum_server::bind(addr).serve(svc).await?;

    Ok(())
}

pub async fn https_server<S>(listen_addr: Ipv4Addr, svc: S, config: RustlsConfig) -> ApiResult<()>
where
    S: Send + MakeService<SocketAddr, Request<Incoming>>,
    S::MakeFuture: Send,
{
    let addr = SocketAddr::from((listen_addr, 443));
    log::info!("https listening on {}", addr);

    axum_server::bind_rustls(addr, config).serve(svc).await?;

    Ok(())
}

pub async fn config_writer(res: Arc<Mutex<Resources>>, filename: Utf8PathBuf) -> ApiResult<()> {
    let rx = res.lock().await.state_channel();
    let tmp = filename.with_extension("tmp");
    loop {
        rx.notified().await;

        log::debug!("Config changed, saving..");

        if let Ok(fd) = File::create(&tmp) {
            res.lock().await.write(fd)?;
            std::fs::rename(&tmp, &filename)?;
        }

        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}
