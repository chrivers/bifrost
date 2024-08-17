use axum::Router;

use crate::state::AppState;

pub mod api;
pub mod clip;
pub mod eventstream;

pub fn router(appstate: AppState) -> Router<()> {
    Router::new()
        .nest("/api", api::router())
        .nest("/clip/v2/resource", clip::router())
        .nest("/eventstream", eventstream::router())
        .with_state(appstate)
}
