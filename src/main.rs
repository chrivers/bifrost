use std::io::Write;

use tokio::task::JoinSet;

use bifrost::config;
use bifrost::error::ApiResult;
use bifrost::mdns;
use bifrost::server::{self, banner};
use bifrost::state::AppState;
use bifrost::z2m;

/*
 * Formatter function to output in syslog format. This makes sense when running
 * as a service (where output might go to a log file, or the system journal)
 */
fn syslog_format(
    buf: &mut pretty_env_logger::env_logger::fmt::Formatter,
    record: &log::Record,
) -> std::io::Result<()> {
    writeln!(
        buf,
        "<{}>{}: {}",
        match record.level() {
            log::Level::Error => 3,
            log::Level::Warn => 4,
            log::Level::Info => 6,
            log::Level::Debug => 7,
            log::Level::Trace => 7,
        },
        record.target(),
        record.args()
    )
}

fn init_logging() -> ApiResult<()> {
    /* Try to provide reasonable default filters, when RUST_LOG is not specified */
    const DEFAULT_LOG_FILTERS: &[&str] = &[
        "debug",
        "mdns_sd=off",
        "tower_http::trace::on_request=info",
        "axum::rejection=trace",
    ];

    let log_filters = std::env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_LOG_FILTERS.join(","));

    /* Detect if we need syslog or human-readable formatting */
    if std::env::var("SYSTEMD_EXEC_PID").is_ok_and(|pid| pid == std::process::id().to_string()) {
        Ok(pretty_env_logger::env_logger::builder()
            .format(syslog_format)
            .parse_filters(&log_filters)
            .try_init()?)
    } else {
        Ok(pretty_env_logger::formatted_timed_builder()
            .parse_filters(&log_filters)
            .try_init()?)
    }
}

async fn build_tasks(appstate: AppState) -> ApiResult<JoinSet<ApiResult<()>>> {
    let bconf = &appstate.config().bridge;
    let _mdns = mdns::register_mdns(bconf.mac, bconf.ipaddress);

    let mut tasks = JoinSet::new();

    let svc = server::build_service(appstate.clone());

    log::info!("Serving mac [{}]", bconf.mac);

    let tls_config = appstate.tls_config().await?;
    let state_file = appstate.config().bifrost.state_file.clone();

    tasks.spawn(server::http_server(
        bconf.ipaddress,
        bconf.http_port,
        svc.clone(),
    ));
    tasks.spawn(server::https_server(
        bconf.ipaddress,
        bconf.https_port,
        svc,
        tls_config,
    ));
    tasks.spawn(server::config_writer(appstate.res.clone(), state_file));

    for (name, server) in &appstate.config().z2m.servers {
        let client = z2m::Client::new(
            name.clone(),
            server.url.clone(),
            appstate.config(),
            appstate.res.clone(),
        )?;
        tasks.spawn(client.run_forever());
    }

    Ok(tasks)
}

async fn run() -> ApiResult<()> {
    init_logging()?;

    #[cfg(feature = "server-banner")]
    banner::print()?;

    let config = config::parse("config.yaml".into())?;
    log::debug!("Configuration loaded successfully");

    let appstate = AppState::from_config(config)?;

    let mut tasks = build_tasks(appstate).await?;

    loop {
        match tasks.join_next().await {
            None => break Ok(()),
            Some(Ok(Ok(res))) => log::info!("Worker returned: {res:?}"),
            Some(Ok(Err(res))) => log::error!("Worked task failed: {res:?}"),
            Some(Err(err)) => log::error!("Error spawning from worker: {err:?}"),
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        log::error!("Bifrost error: {err}");
        log::error!("Fatal error encountered, cannot continue.");
    }
}
