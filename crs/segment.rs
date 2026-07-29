use std::ops::Range;

/// Represents a single download segment.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Segment index.
    pub id: usize,

    /// Byte range assigned to this segment.
    pub range: Range<u64>,

    /// Downloaded bytes within this segment.
    pub downloaded: u64,
}

impl Segment {
    /// Creates a new segment.
    pub fn new(id: usize, start: u64, end: u64) -> Self {
        Self {
            id,
            range: start..end,
            downloaded: 0,
        }
    }

    /// Returns the segment size.
    pub fn size(&self) -> u64 {
        self.range.end - self.range.start
    }

    /// Returns true if the segment is fully downloaded.
    pub fn is_complete(&self) -> bool {
        self.downloaded >= self.size()
    }

    /// Returns the next byte offset to request.
    pub fn current_offset(&self) -> u64 {
        self.range.start + self.downloaded
    }
}