use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;

use crate::mqtt::Client;
use crate::state::AppState;

pub async fn get_clip_v2(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let conf = state.mqtt_config();

    let mut client = Client::new(
        "rust-mqtt",
        &conf.username,
        &conf.password,
        &conf.host,
        1883,
    );

    for topic in &conf.topics {
        log::debug!("Subscribing to [{topic}]");
        client.subscribe(topic).await;
    }

    let stream = client.into_stream();

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    )
}
