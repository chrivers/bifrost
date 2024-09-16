#![allow(unused_variables)]

use std::io::{stdin, Stdin};

use bifrost::error::ApiResult;
use bifrost::hue::api::ResourceRecord;

use json_diff_ng::compare_serde_values;
use serde_json::{de::IoRead, Deserializer, StreamDeserializer, Value};

fn false_positive((a, b): &(&Value, &Value)) -> bool {
    a.is_number() && b.is_number() && a.as_f64() == b.as_f64()
}

/*
 * Handle both "raw" hue bridge dumps, and the "linedump" format that is
 * generally easier to work with.
 *
 * For each element in the input, if it is an object, look for a ".data" array,
 * and if found, iterate over that. Otherwise, assume the whole object is what
 * we are parsing.
 */
fn extract(
    stream: StreamDeserializer<IoRead<Stdin>, Value>,
) -> impl Iterator<Item = (usize, Value)> + '_ {
    stream
        .flat_map(|val| {
            val.as_ref()
                .unwrap()
                .as_object()
                .and_then(|obj| obj.get("data"))
                .map(|data| data.as_array().unwrap().to_owned())
                .unwrap_or_else(|| vec![val.unwrap()])
        })
        .enumerate()
}

fn compare(before: &Value, after: &Value, msg: ResourceRecord) -> ApiResult<()> {
    let diffs = compare_serde_values(before, after, true, &[]).unwrap();
    let all_diffs = diffs.all_diffs();

    if !all_diffs
        .iter()
        .any(|x| x.1.values.map(|q| !false_positive(&q)).unwrap_or(true))
    {
        return Ok(());
    }

    log::error!("Difference detected on {:?}", msg.obj.rtype());
    eprintln!("--------------------------------------------------------------------------------");
    println!("{}", serde_json::to_string(before)?);
    eprintln!("--------------------------------------------------------------------------------");
    println!("{}", serde_json::to_string(&msg)?);
    eprintln!("--------------------------------------------------------------------------------");
    for (d_type, d_path) in all_diffs {
        if let Some(ref ab) = d_path.values {
            if false_positive(ab) {
                continue;
            }
        }
        match d_type {
            json_diff_ng::DiffType::LeftExtra => {
                eprintln!(" - {d_path}");
            }
            json_diff_ng::DiffType::Mismatch => {
                eprintln!(" * {d_path}");
            }
            json_diff_ng::DiffType::RightExtra => {
                eprintln!(" + {d_path}");
            }
            json_diff_ng::DiffType::RootMismatch => {
                eprintln!("{d_type}: {d_path}");
            }
        }
    }
    eprintln!();

    Ok(())
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    let stream = Deserializer::from_reader(stdin()).into_iter::<Value>();

    for (index, obj) in extract(stream) {
        let before = obj;
        let data: Result<ResourceRecord, _> = serde_json::from_value(before.clone());

        let Ok(msg) = data else {
            let err = data.unwrap_err();
            log::error!("Parse error {err:?} (object index {})", index);
            eprintln!("{}", &serde_json::to_string(&before)?);
            continue;
        };

        let after = serde_json::to_value(&msg)?;

        compare(&before, &after, msg)?;
    }

    Ok(())
}
