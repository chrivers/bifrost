use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use futures::stream::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::error::ApiResult;
use crate::server::appstate::AppState;

pub async fn get_clip_v2(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = ApiResult<Event>>> {
    let hello = tokio_stream::iter([Ok(Event::default().comment("hi"))]);

    let mut prev_ts = Utc::now().timestamp();
    let mut idx = 0;

    let channel = state.res.lock().await.hue_channel();

    let stream = BroadcastStream::new(channel).map(move |e| {
        let json = [e?];
        log::trace!(
            "## EVENT ##: {}",
            serde_json::to_string(&json).unwrap_or_else(|_| "ERROR".to_string())
        );
        let ts = Utc::now().timestamp();
        if ts == prev_ts {
            idx += 1;
        } else {
            idx = 0;
            prev_ts = ts;
        }
        Ok(Event::default().id(format!("{ts}:{idx}")).json_data(json)?)
    });

    Sse::new(hello.chain(stream))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/clip/v2", get(get_clip_v2))
}
