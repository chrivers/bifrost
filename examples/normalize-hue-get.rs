#![allow(unused_variables)]

use std::io::stdin;

use bifrost::error::ApiResult;
use bifrost::hue::api::ResourceRecord;

#[tokio::main]
async fn main() -> ApiResult<()> {
    pretty_env_logger::init();

    for (index, line) in stdin().lines().enumerate() {
        let data: Result<ResourceRecord, _> = serde_json::from_str(&line?);

        let Ok(msg) = data else {
            let err = data.unwrap_err();
            log::error!("Parse error {err:?} (stdin line {index})");
            continue;
        };

        println!("{}", serde_json::to_string(&msg)?);
    }

    Ok(())
}
