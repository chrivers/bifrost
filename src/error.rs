use thiserror::Error;
use tokio::task::JoinError;
use uuid::Uuid;

use crate::hue::{event::EventBlock, v2::RType};

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    JoinError(#[from] JoinError),

    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),

    #[error(transparent)]
    SendError(#[from] tokio::sync::broadcast::error::SendError<EventBlock>),

    #[error(transparent)]
    SetLoggerError(#[from] log::SetLoggerError),

    #[error(transparent)]
    BroadcastStreamRecvError(#[from] tokio_stream::wrappers::errors::BroadcastStreamRecvError),

    #[cfg(feature = "mqtt")]
    #[error(transparent)]
    MqttError(#[from] rumqttc::v5::ClientError),

    #[error(transparent)]
    AxumError(#[from] axum::Error),

    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Request failed: {0}")]
    Fail(&'static str),

    #[error("Resource {0} could not be deleted")]
    DeleteDenied(Uuid),

    #[error("Resource {0} not found")]
    NotFound(Uuid),

    #[error("Cannot allocate any more {0:?}")]
    Full(RType),
}

pub type ApiResult<T> = Result<T, ApiError>;
