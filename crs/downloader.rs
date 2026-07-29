use crate::{
    error::{DownloadError, Result},
    retry::backoff,
    segment::{split_file, Segment},
    types::{DownloadProgress, DownloadStatus, StarLevel},
    verify::sha256_file,
};

use futures_util::StreamExt;
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, watch, Mutex},
};

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub stars: StarLevel,
    pub max_retries: usize,
    pub verify_sha256: Option<String>,
    pub progress_buffer: usize,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            stars: StarLevel::Three,
            max_retries: 5,
            verify_sha256: None,
            progress_buffer: 64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadReport {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    url: String,
    total_size: u64,
    segments: Vec<Segment>,
    completed: Vec<bool>,
}

pub struct DownloadManager {
    client: Client,
}

pub struct DownloadHandle {
    pub progress: watch::Receiver<DownloadProgress>,
    pub task: tokio::task::JoinHandle<Result<DownloadReport>>,
}