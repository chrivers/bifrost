#![allow(unused_variables)]

use std::io::stdin;

use bifrost::{
    error::ApiResult,
    z2m::{
        api::{Availability, Message, RawMessage},
        update::DeviceUpdate,
    },
};
use log::LevelFilter;

#[tokio::main]
#[rustfmt::skip]
async fn main() -> ApiResult<()> {
    pretty_env_logger::formatted_builder().filter_level(LevelFilter::Debug).init();

    for line in stdin().lines() {
        let line = line?;

        let raw_data = serde_json::from_str::<RawMessage>(&line);

        let Ok(raw_msg) = raw_data else {
            log::error!("INVALID LINE: {:#?}", raw_data);
            continue;
        };

        /* bridge messages are those on bridge/+ topics */

        if raw_msg.topic.starts_with("bridge/") {
            let data = serde_json::from_str(&line);

            let Ok(msg) = data else {
                log::error!("INVALID LINE [bridge]: {:#?}", data);
                continue;
            };

            match msg {
                Message::BridgeInfo(ref obj) => {
                    println!("{:#?}", obj.config_schema);
                },
                Message::BridgeLogging(ref obj) => {
                    println!("{obj:#?}");
                },
                Message::BridgeExtensions(ref obj) => {
                    println!("{obj:#?}");
                },
                Message::BridgeDevices(ref devices) => {
                    for dev in devices {
                        println!("{dev:#?}");
                    }
                },
                Message::BridgeGroups(ref obj) => {
                    println!("{obj:#?}");
                },
                Message::BridgeDefinitions(ref obj) => {
                    println!("{obj:#?}");
                },
                Message::BridgeState(ref obj) => {
                    println!("{obj:#?}");
                },
                Message::BridgeEvent(ref obj) => {
                    println!("{obj:#?}");
                },
            }

            continue;
        }

        /* everything that ends in /availability are online/offline updates */

        if raw_msg.topic.ends_with("/availability") {
            let data = serde_json::from_value::<Availability>(raw_msg.payload);

            let Ok(msg) = data else {
                log::error!("INVALID LINE [availability]: {}", data.unwrap_err());
                eprintln!("{line}");
                eprintln!();
                continue;
            };

            continue;
        }

        /* everything else: device updates */

        let data = serde_json::from_value::<DeviceUpdate>(raw_msg.payload);

        let Ok(msg) = data else {
            log::error!("INVALID LINE [device]: {}", data.unwrap_err());
            eprintln!("{line}");
            eprintln!();
            continue;
        };

        /* having unknown fields is not an error. they are simply not mapped */
        /* if !msg.__.is_empty() { */
        /*     log::warn!("Unknown fields found: {:?}", msg.__.keys()); */
        /* } */
        println!("{msg:#?}");
    }

    Ok(())
}
