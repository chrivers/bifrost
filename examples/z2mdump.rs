use clap::Parser;
use futures::StreamExt;
use hyper::Uri;
use serde::Deserialize;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use bifrost::error::ApiResult;

#[derive(Parser, Debug)]
struct Args {
    /// Url to websocket (example: ws://example.org:8080/)
    url: Uri,
}

#[derive(Debug, Deserialize)]
struct Z2mMessage {
    topic: String,
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(err) => {
            log::error!("Argument error: {err}");
            std::process::exit(1);
        }
    };

    let (mut socket, _) = connect_async(args.url).await?;

    loop {
        let Some(pkt) = socket.next().await else {
            break;
        };

        let Message::Text(txt) = pkt? else { break };

        let json: Z2mMessage = serde_json::from_str(&txt)?;

        if json.topic.starts_with("bridge/") {
            log::info!("Got message [{}]", json.topic);
            println!("{}", txt);
        } else {
            log::info!("No more z2m bridge messages");
            break;
        }
    }

    Ok(())
}
