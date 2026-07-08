//! Circular ring buffer that stores the last N seconds of video frames.

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
        }
    }
}

/// Fixed-capacity circular buffer for gameplay frames.
pub struct RingBuffer {
    /// Pre-allocated slot array (None = empty slot).
    slots: Vec<Option<StoredFrame>>,
    /// Total capacity (max frame count).
    capacity: usize,
    /// Index where the next frame will be written.
    write_index: usize,
    /// Number of frames currently stored.
    count: usize,
}

impl RingBuffer {
    /// Create a ring buffer that holds at most `capacity` frames.
    pub fn new(capacity: usize) -> Self {
        let mut slots = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slots.push(None);
        }
        Self {
            slots,
            capacity,
            write_index: 0,
            count: 0,
        }
    }

    /// Push a frame into the buffer. If full, the oldest frame is overwritten.
    pub fn push(&mut self, frame: impl Into<StoredFrame>) {
        self.slots[self.write_index] = Some(frame.into());
        self.write_index = (self.write_index + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Return all frames within the last `duration` from `now`.
    ///
    /// Frames are returned in chronological order (oldest first).
    pub fn clip_since(&self, duration: Duration, now: Instant) -> Vec<StoredFrame> {
        let cutoff = now.checked_sub(duration).unwrap_or(now);
        let mut result = Vec::new();

        let start = if self.count < self.capacity {
            0
        } else {
            self.write_index // oldest frame is at write_index when full
        };
        let len = self.count;

        for i in 0..len {
            let idx = (start + i) % self.capacity;
            if let Some(ref frame) = self.slots[idx] {
                if frame.timestamp >= cutoff {
                    result.push(frame.clone());
                }
            }
        }

        result
    }

    /// Return all frames in the buffer (oldest first).
    pub fn all_frames(&self) -> Vec<StoredFrame> {
        let start = if self.count < self.capacity {
            0
        } else {
            self.write_index
        };
        let len = self.count;
        let mut result = Vec::with_capacity(len);

        for i in 0..len {
            let idx = (start + i) % self.capacity;
            if let Some(ref frame) = self.slots[idx] {
                result.push(frame.clone());
            }
        }

        result
    }

    /// Clear all frames from the buffer.
    pub fn clear(&mut self) {
        for slot in &mut self.slots[..self.count.max(self.capacity)] {
            *slot = None;
        }
        self.write_index = 0;
        self.count = 0;
    }

    /// Current number of frames stored.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Maximum frame capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::PixelFormat;
    use std::time::{Duration, Instant};

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
        // Push 5 frames at 1s intervals
        for i in 0..5 {
            buf.push(make_frame(now + Duration::from_secs(i * 2)));
        }
        // Clip last 3 seconds from `now + 8s` (frame 4 is at 8s)
        let clip_time = now + Duration::from_secs(8);
        let clipped = buf.clip_since(Duration::from_secs(3), clip_time);
        // Should get frames at 6s and 8s (indices 2, 3, 4 but 4=8s, 3=6s, 2=4s...
        // actually: frames[0]=0s, [1]=2s, [2]=4s, [3]=6s, [4]=8s
        // clip_since(3s, 8s) = frames with timestamp >= 5s = frames[3]=6s, frames[4]=8s
        assert_eq!(clipped.len(), 2);
    }
}
