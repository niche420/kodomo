use thiserror::Error;

pub type Result<T> = std::result::Result<T, StreamError>;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("Capture error: {0}")]
    Capture(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Input error: {0}")]
    Input(String),

    #[error("Already running")]
    AlreadyRunning,

    #[error("Not running")]
    NotRunning,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}