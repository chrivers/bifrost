#![allow(unused_variables)]

use std::io::stdin;

use bifrost::error::ApiResult;
use bifrost::hue::api::ResourceRecord;

use json_diff_ng::compare_serde_values;
use serde_json::Value;

fn false_positive((a, b): &(&Value, &Value)) -> bool {
    a.is_number() && b.is_number() && a.as_f64() == b.as_f64()
}

#[tokio::main]
async fn main() -> ApiResult<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Debug)
        .parse_default_env()
        .init();

    for (index, line) in stdin().lines().enumerate() {
        let line = line?;
        let before: Value = serde_json::from_str(&line)?;
        let data: Result<ResourceRecord, _> = serde_json::from_str(&line);

        let Ok(msg) = data else {
            let err = data.unwrap_err();
            log::error!("Parse error {err:?} (stdin line {})", index + 1);
            eprintln!("{}", &line);
            continue;
        };

        let after = serde_json::to_value(&msg)?;

        let diffs = compare_serde_values(&before, &after, true, &[]).unwrap();
        if diffs
            .all_diffs()
            .iter()
            .any(|x| x.1.values.map(|q| !false_positive(&q)).unwrap_or(true))
        {
            log::error!("Difference detected on {:?}", msg.obj.rtype());
            println!(
                "--------------------------------------------------------------------------------"
            );
            println!("{}", &line);
            println!(
                "--------------------------------------------------------------------------------"
            );
            println!("{}", serde_json::to_string(&msg)?);
            println!(
                "--------------------------------------------------------------------------------"
            );
            for (d_type, d_path) in diffs.all_diffs() {
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
        }
    }

    Ok(())
}
