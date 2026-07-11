use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::capture::CapturedFrame;

/// A frame stored in the ring buffer — cloneable snapshot of captured data.
#[derive(Debug, Clone)]
pub struct StoredFrame {
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: crate::capture::PixelFormat,
    pub timestamp: Instant,
    pub is_sync: bool,
}

impl From<CapturedFrame> for StoredFrame {
    fn from(f: CapturedFrame) -> Self {
        Self {
            data: f.data,
            width: f.width,
            height: f.height,
            stride: f.stride,
            pixel_format: f.pixel_format,
            timestamp: f.timestamp,
            is_sync: true,
        }
    }
}

/// Bounded FIFO buffer for gameplay frames with dual constraints:
/// - Max frame count (`capacity`)
/// - Max total bytes (`max_bytes`)
///
/// When either limit is exceeded, the oldest frames are evicted first.
pub struct RingBuffer {
    frames: VecDeque<StoredFrame>,
    /// Hard frame-count limit.
    capacity: usize,
    /// Sum of `data.len()` across all stored frames.
    total_bytes: usize,
    /// If non-zero, the buffer will evict oldest frames to stay under this
    /// budget (even if below the frame-count limit).
    max_bytes: usize,
}

impl RingBuffer {
    /// Create a ring buffer bounded by `capacity` frames with no byte limit.
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
            total_bytes: 0,
            max_bytes: 0,
        }
    }

    /// Create a ring buffer bounded by both frame count and byte budget.
    pub fn with_byte_budget(capacity: usize, max_bytes: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
            total_bytes: 0,
            max_bytes,
        }
    }

    /// Push a frame into the buffer. Oldest frames are evicted first if either
    /// the frame-count limit or the byte budget would be exceeded.
    pub fn push(&mut self, frame: impl Into<StoredFrame>) {
        let frame = frame.into();
        let frame_len = frame.data.len();

        // Evict oldest frames to stay within byte budget
        if self.max_bytes > 0 {
            while self.total_bytes + frame_len > self.max_bytes && !self.frames.is_empty() {
                if let Some(old) = self.frames.pop_front() {
                    self.total_bytes = self.total_bytes.saturating_sub(old.data.len());
                }
            }
        }

        // Evict oldest frames to stay within frame-count limit
        while self.frames.len() >= self.capacity {
            if let Some(old) = self.frames.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(old.data.len());
            }
        }

        self.total_bytes += frame_len;
        self.frames.push_back(frame);
    }

    /// Return all frames within the last `duration` from `now`.
    pub fn clip_since(&self, duration: Duration, now: Instant) -> Vec<StoredFrame> {
        let cutoff = now.checked_sub(duration).unwrap_or(now);
        self.frames
            .iter()
            .filter(|f| f.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    /// Return all frames (oldest first).
    pub fn all_frames(&self) -> Vec<StoredFrame> {
        self.frames.iter().cloned().collect()
    }

    /// Clear all frames.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.total_bytes = 0;
    }

    /// Current number of frames stored.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Maximum frame capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Total bytes of all stored frames.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Scan all stored frames (regardless of timestamp) for H.264 SPS (NAL type 7)
    /// and PPS (NAL type 8) in AVCC format (4-byte big-endian length prefix per NAL).
    /// Returns the first matching pair, or `None`.
    pub fn find_sps_pps_anywhere(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        for frame in &self.frames {
            if frame.pixel_format != crate::capture::PixelFormat::H264 {
                continue;
            }
            let data = &*frame.data;
            let mut offset = 0;
            let mut sps = None;
            let mut pps = None;

            while offset + 4 <= data.len() {
                let nal_len =
                    u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                if offset + nal_len > data.len() {
                    break;
                }
                let nal_type = data[offset] & 0x1F;
                match nal_type {
                    7 => sps = Some(data[offset..offset + nal_len].to_vec()),
                    8 => pps = Some(data[offset..offset + nal_len].to_vec()),
                    _ => {}
                }
                if sps.is_some() && pps.is_some() {
                    return Some((sps.unwrap(), pps.unwrap()));
                }
                offset += nal_len;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;

    fn make_frame(t: Instant) -> CapturedFrame {
        CapturedFrame {
            data: Arc::new(vec![0u8; 100]),
            width: 10,
            height: 10,
            stride: 10,
            pixel_format: PixelFormat::Bgra,
            timestamp: t,
        }
    }

    #[test]
    fn test_push_and_count() {
        let mut buf = RingBuffer::new(5);
        let now = Instant::now();
        for i in 0..3 {
            buf.push(make_frame(now + Duration::from_secs(i)));
        }
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_wraparound() {
        let mut buf = RingBuffer::new(3);
        let now = Instant::now();
        for i in 0..5 {
            buf.push(make_frame(now + Duration::from_secs(i as u64)));
        }
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.all_frames().len(), 3);
    }

    #[test]
    fn test_clip_since() {
        let mut buf = RingBuffer::new(10);
        let now = Instant::now();
        for i in 0..5 {
            buf.push(make_frame(now + Duration::from_secs(i * 2)));
        }
        let clip_time = now + Duration::from_secs(8);
        let clipped = buf.clip_since(Duration::from_secs(3), clip_time);
        assert_eq!(clipped.len(), 2);
    }

    #[test]
    fn test_byte_budget_eviction() {
        // Budget of 250 bytes with 100-byte frames → at most 2 frames
        let mut buf = RingBuffer::with_byte_budget(10, 250);
        let now = Instant::now();
        buf.push(make_frame(now));
        assert_eq!(buf.len(), 1);
        buf.push(make_frame(now + Duration::from_secs(1)));
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.total_bytes(), 200);
        // Third frame exceeds 250 → oldest evicted
        buf.push(make_frame(now + Duration::from_secs(2)));
        assert_eq!(buf.len(), 2);
        assert_eq!(buf.total_bytes(), 200);
    }

    #[test]
    fn test_frame_capacity_still_applies() {
        let mut buf = RingBuffer::with_byte_budget(2, usize::MAX);
        let now = Instant::now();
        for i in 0..3 {
            buf.push(make_frame(now + Duration::from_secs(i)));
        }
        assert_eq!(buf.len(), 2);
    }
}
