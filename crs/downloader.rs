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
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{mpsc, Mutex},
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

impl DownloadManager {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("rust-download-core/0.1")
            .build()?;
        Ok(Self { client })
    }

    pub async fn download(
        &self,
        url: &str,
        output: impl AsRef<Path>,
        options: DownloadOptions,
    ) -> Result<DownloadReport> {
        let output = output.as_ref().to_path_buf();
        let temp_dir = output.with_extension("download");
        fs::create_dir_all(&temp_dir).await?;

        let head = self.client.head(url).send().await?;
        let total = head
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        let supports_range = head
            .headers()
            .get(header::ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("bytes"))
            .unwrap_or(false);

        // If size or range support is unavailable, use a safe single-stream download.
        if total.is_none() || !supports_range || options.stars.connections() == 1 {
            self.single_stream(url, &output).await?;
            let hash = if options.verify_sha256.is_some() {
                Some(sha256_file(&output).await?)
            } else {
                None
            };
            if let Some(expected) = options.verify_sha256 {
                if hash.as_deref() != Some(expected.trim()) {
                    return Err(DownloadError::VerificationFailed);
                }
            }
            return Ok(DownloadReport {
                path: output,
                bytes: fs::metadata(&output).await?.len(),
                sha256: hash,
            });
        }

        let total = total.unwrap();
        let manifest_path = temp_dir.join("manifest.json");
        let manifest = self
            .load_or_create_manifest(
                &manifest_path,
                url,
                total,
                options.stars.connections(),
            )
            .await?;

        let progress = Arc::new(Mutex::new(DownloadProgress {
            downloaded: 0,
            total,
            status: DownloadStatus::Starting,
        }));

        let (tx, mut rx) = mpsc::channel(options.progress_buffer.max(1));
        let client = self.client.clone();
        let temp_dir_arc = Arc::new(temp_dir.clone());
        let max_retries = options.max_retries;

        let progress_task = {
            let progress = progress.clone();
            tokio::spawn(async move {
                while let Some(delta) = rx.recv().await {
                    let mut p = progress.lock().await;
                    p.downloaded = p.downloaded.saturating_add(delta);
                    p.status = DownloadStatus::Downloading;
                }
            })
        };

        let manifest_arc = Arc::new(Mutex::new(manifest));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(options.stars.connections()));
        let mut tasks = Vec::new();

        for segment_index in 0..manifest_arc.lock().await.segments.len() {
            if manifest_arc.lock().await.completed[segment_index] {
                continue;
            }

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let client = client.clone();
            let url = url.to_string();
            let manifest_arc = manifest_arc.clone();
            let temp_dir = temp_dir_arc.clone();
            let tx = tx.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = permit;
                let segment = {
                    let m = manifest_arc.lock().await;
                    m.segments[segment_index].clone()
                };

                let result = download_segment(
                    &client,
                    &url,
                    &temp_dir,
                    &segment,
                    max_retries,
                    tx,
                )
                .await;

                if result.is_ok() {
                    let mut m = manifest_arc.lock().await;
                    m.completed[segment_index] = true;
                }
                result
            }));
        }

        for task in tasks {
            task.await
                .map_err(|_| DownloadError::SegmentFailed(usize::MAX))??;
        }

        drop(tx);
        progress_task.await.ok();

        {
            let m = manifest_arc.lock().await;
            let data = serde_json::to_vec_pretty(&*m)?;
            fs::write(&manifest_path, data).await?;
        }

        {
            let mut p = progress.lock().await;
            p.status = DownloadStatus::Merging;
        }

        self.merge_segments(&temp_dir, &output, &manifest_arc.lock().await.segments)
            .await?;

        {
            let mut p = progress.lock().await;
            p.status = DownloadStatus::Verifying;
        }

        let hash = if options.verify_sha256.is_some() {
            Some(sha256_file(&output).await?)
        } else {
            None
        };

        if let Some(expected) = options.verify_sha256 {
            if hash.as_deref() != Some(expected.trim()) {
                return Err(DownloadError::VerificationFailed);
            }
        }

        {
            let mut p = progress.lock().await;
            p.status = DownloadStatus::Completed;
        }

        fs::remove_dir_all(&temp_dir).await.ok();

        Ok(DownloadReport {
            path: output,
            bytes: total,
            sha256: hash,
        })
    }

    async fn single_stream(&self, url: &str, output: &Path) -> Result<()> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(DownloadError::HttpStatus {
                status: response.status().as_u16(),
                message: response.status().to_string(),
            });
        }

        let mut stream = response.bytes_stream();
        let mut file = File::create(output).await?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn load_or_create_manifest(
        &self,
        path: &Path,
        url: &str,
        total: u64,
        connections: usize,
    ) -> Result<Manifest> {
        if let Ok(data) = fs::read(path).await {
            if let Ok(manifest) = serde_json::from_slice::<Manifest>(&data) {
                if manifest.url == url && manifest.total_size == total {
                    return Ok(manifest);
                }
            }
        }

        let segments = split_file(total, connections);
        let manifest = Manifest {
            url: url.to_string(),
            total_size: total,
            completed: vec![false; segments.len()],
            segments,
        };

        fs::write(path, serde_json::to_vec_pretty(&manifest)?).await?;
        Ok(manifest)
    }

    async fn merge_segments(
        &self,
        temp_dir: &Path,
        output: &Path,
        segments: &[Segment],
    ) -> Result<()> {
        let mut final_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(output)
            .await?;

        for segment in segments {
            let path = temp_dir.join(format!("segment-{}.part", segment.index));
            if !path.exists() {
                return Err(DownloadError::FileNotFound(path));
            }

            let mut part = File::open(&path).await?;
            let mut buffer = vec![0u8; 1024 * 1024];

            loop {
                let n = part.read(&mut buffer).await?;
                if n == 0 {
                    break;
                }
                final_file.write_all(&buffer[..n]).await?;
            }
        }

        final_file.flush().await?;
        Ok(())
    }
}

async fn download_segment(
    client: &Client,
    url: &str,
    temp_dir: &Path,
    segment: &Segment,
    max_retries: usize,
    tx: mpsc::Sender<u64>,
) -> Result<()> {
    let path = temp_dir.join(format!("segment-{}.part", segment.index));

    let expected = segment.len();
    let mut existing = if let Ok(meta) = fs::metadata(&path).await {
        meta.len().min(expected)
    } else {
        0
    };

    if existing == expected {
        return Ok(());
    }

    for attempt in 0..=max_retries {
        let start = segment.start + existing;
        let end = segment.end;

        let response = client
            .get(url)
            .header(header::RANGE, format!("bytes={}-{}", start, end))
            .send()
            .await;

        match response {
            Ok(response) if response.status() == StatusCode::PARTIAL_CONTENT => {
                let mut stream = response.bytes_stream();
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await?;

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    file.write_all(&chunk).await?;
                    existing += chunk.len() as u64;
                    tx.send(chunk.len() as u64).await.ok();
                }

                file.flush().await?;

                if existing == expected {
                    return Ok(());
                }
            }
            Ok(response) => {
                if response.status().is_success() && start == 0 {
                    return Err(DownloadError::RangeNotSupported);
                }
                if attempt == max_retries {
                    return Err(DownloadError::SegmentFailed(segment.index));
                }
            }
            Err(_) if attempt == max_retries => {
                return Err(DownloadError::SegmentFailed(segment.index));
            }
            Err(_) => {}
        }

        backoff(attempt).await;
    }

    Err(DownloadError::SegmentFailed(segment.index))
} 