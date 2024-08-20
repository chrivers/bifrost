use std::collections::HashMap;
use std::fs::File;
use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use camino::Utf8Path;
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::{ApiError, ApiResult};
use crate::hue::legacy_api::{ApiConfig, ApiShortConfig, Whitelist};
use crate::resource::Resources;
use crate::server::{self, certificate};

#[derive(Clone)]
pub struct AppState {
    conf: Arc<AppConfig>,
    pub res: Arc<Mutex<Resources>>,
}

impl AppState {
    pub fn from_config(config: AppConfig) -> ApiResult<Self> {
        let certfile = &config.bifrost.cert_file;

        let certpath = Utf8Path::new(certfile);
        if certpath.is_file() {
            certificate::check_certificate(certpath, config.bridge.mac)?;
        } else {
            log::warn!("Missing certificate file [{certfile}], generating..");
            certificate::generate_and_save(certpath, config.bridge.mac)?;
        }

        let mut res = Resources::new();

        if let Ok(fd) = File::open(&config.bifrost.state_file) {
            log::debug!("Existing state file found, loading..");
            res.read(fd)?;
        } else {
            log::debug!("No state file found, initializing..");
            res.init(&server::certificate::hue_bridge_id(config.bridge.mac))?;
        }

        let conf = Arc::new(config);
        let res = Arc::new(Mutex::new(res));

        Ok(Self { conf, res })
    }

    pub async fn tls_config(&self) -> ApiResult<RustlsConfig> {
        let certfile = &self.conf.bifrost.cert_file;

        log::debug!("Loading certificate from [{certfile}]");
        RustlsConfig::from_pem_file(&certfile, &certfile)
            .await
            .map_err(|e| ApiError::Certificate(certfile.to_owned(), e))
    }

    #[must_use]
    pub fn config(&self) -> Arc<AppConfig> {
        self.conf.clone()
    }

    #[must_use]
    pub fn api_short_config(&self) -> ApiShortConfig {
        let mac = self.conf.bridge.mac;
        ApiShortConfig {
            bridgeid: certificate::hue_bridge_id(mac),
            mac,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn api_config(&self, username: Uuid) -> ApiConfig {
        ApiConfig {
            short_config: self.api_short_config(),
            ipaddress: self.conf.bridge.ipaddress,
            netmask: self.conf.bridge.netmask,
            gateway: self.conf.bridge.gateway,
            timezone: self.conf.bridge.timezone.clone(),
            whitelist: HashMap::from([(
                username,
                Whitelist {
                    create_date: Utc::now(),
                    last_use_date: Utc::now(),
                    name: "User#foo".to_string(),
                },
            )]),
            ..ApiConfig::default()
        }
    }
}
