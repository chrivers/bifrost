use std::{convert::Infallible, time::Duration};

use async_stream::try_stream;
use axum::response::sse::Event;
use futures::stream::Stream;
use serde_json::json;

use rumqttc::v5::mqttbytes::v5::{Filter, Packet, RetainForwardRule};
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::{AsyncClient, EventLoop, MqttOptions};

pub struct Client {
    client: AsyncClient,
    connection: EventLoop,
}

impl Client {
    pub fn new(name: &str, username: &str, password: &str, host: &str, port: u16) -> Self {
        let mut mqttoptions =
            MqttOptions::new(format!("{name}{}", rand::random::<u32>()), host, port);
        mqttoptions
            .set_keep_alive(Duration::from_secs(5))
            .set_credentials(username, password);
        /* .set_last_will(will); */

        let (client, connection) = AsyncClient::new(mqttoptions, 10);

        Self { client, connection }
    }

    pub async fn subscribe(&mut self, topic: &str) {
        let mut f = Filter::new(topic, QoS::AtLeastOnce);
        f.retain_forward_rule = RetainForwardRule::OnEverySubscribe;
        self.client.subscribe_many([f]).await.unwrap();
    }

    pub fn into_stream(mut self) -> impl Stream<Item = Result<Event, Infallible>> {
        try_stream! {
            yield Event::default().comment("hi");

            while let Ok(message) = self.connection.poll().await {
                let rumqttc::v5::Event::Incoming(msg) = message else {
                    continue
                };

                let Packet::Publish(publ) = msg else {
                    continue
                };

                let js = serde_json::from_slice(&publ.payload[..]).unwrap_or(json!({}));
                /* log::info!("{publ:?}"); */
                yield Event::default()
                    .id(format!("{}:0", chrono::Utc::now().timestamp()))
                    .json_data([js])
                    .unwrap()
            }
        }
    }
}
