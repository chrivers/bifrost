use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    MqttError(#[from] rumqttc::v5::ClientError),

    #[error(transparent)]
    AxumError(#[from] axum::Error),

    #[error("Request failed: {0}")]
    Fail(&'static str),

    #[error("Resource {0} not found")]
    NotFound(Uuid),
}

pub type ApiResult<T> = Result<T, ApiError>;
