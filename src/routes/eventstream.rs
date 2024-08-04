use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use chrono::Utc;
use futures::stream::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

pub async fn get_clip_v2(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let hello = tokio_stream::iter([Ok(Event::default().comment("hi"))]);

    let mut prev_ts = Utc::now().timestamp();
    let mut idx = 0;

    let raw = BroadcastStream::new(state.channel().await);
    let stream = raw.map(move |e| {
        let json = [e.unwrap()];
        log::info!(
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
        Event::default().id(format!("{ts}:{idx}")).json_data(json)
    });

    Sse::new(hello.chain(stream)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    )
}
