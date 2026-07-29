pub mod downloader;
pub mod error;
pub mod retry;
pub mod segment;
pub mod types;
pub mod verify;

pub use downloader::{DownloadManager, DownloadOptions, DownloadReport};
pub use error::{DownloadError, Result};
pub use types::{DownloadProgress, DownloadStatus, StarLevel}; 