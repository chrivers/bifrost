pub mod appstate;
pub mod banner;
pub mod certificate;
pub mod entertainment;
pub mod http;
pub mod hueevents;
pub mod scheduler;
pub mod updater;

use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use axum::routing::IntoMakeService;
use axum::{Router, ServiceExt};

use camino::Utf8PathBuf;
use scheduler::Scheduler;
use tokio::select;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, MissedTickBehavior};
use tower::Layer;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use tower_http::trace::TraceLayer;
use tracing::{info_span, Span};

use crate::error::ApiResult;
use crate::resource::Resources;
use crate::routes;
use crate::server::appstate::AppState;
use crate::server::updater::VersionUpdater;

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

pub async fn config_writer(res: Arc<Mutex<Resources>>, filename: Utf8PathBuf) -> ApiResult<()> {
    const STABILIZE_TIME: Duration = Duration::from_secs(1);

    let rx = res.lock().await.state_channel();
    let tmp = filename.with_extension("tmp");

    let mut old_state = res.lock().await.serialize()?;

    loop {
        /* Wait for change notification */
        rx.notified().await;

        /* Updates often happen in burst, and we don't want to write the state
         * file over and over, so ignore repeated update notifications within
         * STABILIZE_TIME */
        let deadline = tokio::time::Instant::now() + STABILIZE_TIME;
        loop {
            select! {
                () = rx.notified() => {},
                () = sleep_until(deadline) => break,
            }
        }

        /* Now that the state is likely stabilized, serialize the new state */
        let new_state = res.lock().await.serialize()?;

        /* If state is not actually changed, try again */
        if old_state == new_state {
            continue;
        }

        log::debug!("Config changed, saving..");

        let mut fd = File::create(&tmp)?;
        fd.write_all(new_state.as_bytes())?;
        std::fs::rename(&tmp, &filename)?;

        old_state = new_state;
    }
}

#[allow(clippy::significant_drop_tightening)]
pub async fn version_updater(
    res: Arc<Mutex<Resources>>,
    upd: Arc<Mutex<VersionUpdater>>,
) -> ApiResult<()> {
    const INTERVAL: Duration = Duration::from_secs(60);
    let mut interval = tokio::time::interval(INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut version = upd.lock().await.get().await.clone();

    res.lock().await.update_bridge_version(version.clone());

    loop {
        interval.tick().await;

        let mut lock = upd.lock().await;
        let new_version = lock.get().await;
        if new_version != &version {
            log::info!("New version detected! Patching state database with new version numbers..");
            version.clone_from(new_version);
            res.lock().await.update_bridge_version(version.clone());
        }
    }
}

pub async fn scheduler(res: Arc<Mutex<Resources>>) -> ApiResult<()> {
    const STABILIZE_TIME: Duration = Duration::from_secs(1);

    let rx = res.lock().await.state_channel();
    let mut scheduler = Scheduler::new(res);

    scheduler.update().await;

    loop {
        /* Wait for change notification */
        rx.notified().await;

        /* Updates often happen in burst, and we don't want to write the state
         * file over and over, so ignore repeated update notifications within
         * STABILIZE_TIME */
        let deadline = tokio::time::Instant::now() + STABILIZE_TIME;
        loop {
            select! {
                () = rx.notified() => {},
                () = sleep_until(deadline) => break,
            }
        }

        scheduler.update().await;
    }
}
