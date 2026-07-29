use serde::{Deserialize, Serialize};

/// Defines the download performance profile.
/// More stars mean more parallel download connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarLevel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
}

impl StarLevel {
    /// Returns the recommended number of parallel connections.
    pub const fn connections(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 4,
            Self::Four => 8,
            Self::Five => 16,
        }
    }
}

/// Current download state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Starting,
    Downloading,
    Merging,
    Verifying,
    Completed,
    Failed(String),
}

impl Default for DownloadStatus {
    fn default() -> Self {
        Self::Starting
    }
}

/// Download progress information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Downloaded bytes.
    pub downloaded: u64,

    /// Total file size in bytes.
    pub total: u64,

    /// Current download status.
    pub status: DownloadStatus,
}

impl DownloadProgress {
    /// Returns download progress as a percentage (0.0 - 100.0).
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }

        (self.downloaded as f64 / self.total as f64) * 100.0
    }

    /// Returns true if the download is complete.
    pub fn is_finished(&self) -> bool {
        matches!(self.status, DownloadStatus::Completed)
    }

    /// Returns true if the download failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.status, DownloadStatus::Failed(_))
    }

    /// Returns the remaining bytes.
    pub fn remaining(&self) -> u64 {
        self.total.saturating_sub(self.downloaded)
    }

    /// Returns true if the download has started.
    pub fn has_started(&self) -> bool {
        self.downloaded > 0
    }

    /// Returns true if the download is still active.
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Starting
                | DownloadStatus::Downloading
                | DownloadStatus::Merging
                | DownloadStatus::Verifying
        )
    }
}