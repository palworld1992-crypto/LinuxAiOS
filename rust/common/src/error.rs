use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}
