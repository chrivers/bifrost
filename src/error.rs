use std::sync::Arc;

use thiserror::Error;
use tokio::task::JoinError;
use uuid::Uuid;

use crate::{
    hue::{
        api::{RType, ResourceLink},
        event::EventBlock,
    },
    z2m::request::ClientRequest,
};

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
    SendErrorHue(#[from] tokio::sync::broadcast::error::SendError<EventBlock>),

    #[error(transparent)]
    SendErrorZ2m(#[from] tokio::sync::broadcast::error::SendError<Arc<ClientRequest>>),

    #[error(transparent)]
    SetLoggerError(#[from] log::SetLoggerError),

    #[error(transparent)]
    BroadcastStreamRecvError(#[from] tokio_stream::wrappers::errors::BroadcastStreamRecvError),

    #[error(transparent)]
    TokioRecvError(#[from] tokio::sync::broadcast::error::RecvError),

    #[error(transparent)]
    AxumError(#[from] axum::Error),

    #[error(transparent)]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Unexpected eof on z2m socket")]
    UnexpectedZ2mEof,

    #[error("Unexpected z2m message: {0:?}")]
    UnexpectedZ2mReply(tokio_tungstenite::tungstenite::Message),

    #[error("State changes not supported for: {0:?}")]
    UpdateUnsupported(RType),

    #[error("Request failed: {0}")]
    Fail(&'static str),

    #[error("Resource {0} could not be deleted")]
    DeleteDenied(Uuid),

    #[error("Missing auxiliary data resource {0:?}")]
    AuxNotFound(ResourceLink),

    #[error("Resource {0} not found")]
    NotFound(Uuid),

    #[error("Resource type wrong: expected {0:?} but found {1:?}")]
    WrongType(RType, RType),

    #[error("Cannot allocate any more {0:?}")]
    Full(RType),
}

pub type ApiResult<T> = Result<T, ApiError>;
