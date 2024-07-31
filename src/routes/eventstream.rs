use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;

use crate::mqtt::Client;
use crate::state::AppState;

const MQTT_HOST: &str = "10.0.0.2";
const MQTT_USERNAME: &str = "mqtt";
const MQTT_PASSWORD: &str = "pass";

const TOPICS: &[&str] = &["homeassistant/light/+/+/+", "zigbee2mqtt/+"];

pub async fn get_clip_v2(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut client = Client::new("rust-mqtt", MQTT_USERNAME, MQTT_PASSWORD, MQTT_HOST, 1883);
    client.subscribe(TOPICS[0]).await;
    client.subscribe(TOPICS[1]).await;

    let stream = client.into_stream();

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    )
}
