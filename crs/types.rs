pub enum StarLevel {
    One = 1,
    Two = 2,                                                Three = 3,
    Four = 4,                                               Five = 5,
}
                                                        impl StarLevel {
    /// Maps stars to sensible parallel connections.
    /// ⭐=1, ⭐⭐=2, ⭐⭐⭐=4, ⭐⭐⭐⭐=8, ⭐⭐⭐⭐⭐=16
    pub fn connections(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 4,
            Self::Four => 8,
            Self::Five => 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadStatus {
    Starting,                                               Downloading,
    Merging,
    Verifying,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub status: DownloadStatus,
}

impl DownloadProgress {
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.downloaded as f64 * 100.0 / self.total as f64
        }
    }                                                   } 