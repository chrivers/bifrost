use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

pub async fn get_clip_v2(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let hello = tokio_stream::iter([Ok(Event::default().comment("hi"))]);

    let raw = BroadcastStream::new(state.channel().await);
    let stream = raw.map(|e| {
        Event::default()
            .id(format!("{}:0", chrono::Utc::now().timestamp()))
            .json_data([e.unwrap()])
    });

    Sse::new(hello.chain(stream)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    )
}
