use clap::Parser;
use futures::StreamExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use bifrost::error::ApiResult;

#[derive(Parser, Debug)]
struct Args {
    /// Url to websocket (ws://example.org:1234/)
    url: String,
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    let args = Args::parse();

    let (mut socket, _) = connect_async(args.url).await?;

    loop {
        let Some(pkt) = socket.next().await else {
            break;
        };

        let Message::Text(txt) = pkt? else { break };

        println!("{}", txt);
    }

    Ok(())
}
