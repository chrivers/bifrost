use thiserror::Error;

pub mod api;
pub mod clip;
pub mod eventstream;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Request failed: {0}")]
    Fail(&'static str),
}
