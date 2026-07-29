use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("download failed with HTTP status {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("server does not support range requests")]
    RangeNotSupported,

    #[error("invalid content length")]
    InvalidContentLength,

    #[error("segment {0} failed after retries")]
    SegmentFailed(usize),

    #[error("download verification failed")]
    VerificationFailed,

    #[error("expected file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("invalid star level")]
    InvalidStarLevel,
}

pub type Result<T> = std::result::Result<T, DownloadError>; 