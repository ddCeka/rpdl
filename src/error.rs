use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Download timeout")]
    Timeout,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Server does not support range requests")]
    RangeNotSupported,

    #[error("Content length unknown")]
    ContentLengthUnknown,

    #[error("Invalid chunk range: {0}")]
    InvalidChunkRange(String),

    #[error("Download was cancelled")]
    Cancelled,

    #[error("Failed to perform operation on zip: {0}")]
    ZipOperationFailed(#[from] zip::result::ZipError),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error")]
    HttpError,
}

pub type Result<T> = color_eyre::Result<T, DownloadError>;
