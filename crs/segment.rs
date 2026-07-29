use serde::{Deserialize, Serialize};

/// Represents a single download segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Segment index.
    pub index: usize,

    /// Start byte (inclusive).
    pub start: u64,

    /// End byte (inclusive).
    pub end: u64,
}

impl Segment {
    /// Creates a new segment.
    pub fn new(index: usize, start: u64, end: u64) -> Self {
        Self { index, start, end }
    }

    /// Returns the segment length in bytes.
    pub fn len(&self) -> u64 {
        self.end - self.start + 1
    }

    /// Returns true if the segment is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Splits a file into multiple download segments.
pub fn split_file(total_size: u64, connections: usize) -> Vec<Segment> {
    if total_size == 0 || connections == 0 {
        return Vec::new();
    }

    let chunk_size = total_size / connections as u64;
    let mut segments = Vec::with_capacity(connections);

    let mut start = 0;

    for index in 0..connections {
        let end = if index == connections - 1 {
            total_size - 1
        } else {
            start + chunk_size - 1
        };

        segments.push(Segment::new(index, start, end));
        start = end + 1;
    }

    segments
}