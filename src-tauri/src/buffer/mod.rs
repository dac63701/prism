//! Ring buffer for keeping the last N seconds of gameplay.
//!
//! The [`BufferManager`] orchestrates frame ingestion from a capture backend
//! and enables saving clips on demand (hotkey trigger, moment detection).

pub mod pool;
pub mod ring;

use crate::capture::CapturedFrame;
pub use pool::FramePool;
pub use ring::{RingBuffer, StoredFrame};
use std::time::Duration;

/// High-level configuration for the ring-buffer recording engine.
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum duration of buffer (seconds).
    pub max_duration_secs: u32,
    /// Expected capture frame rate.
    pub fps: u32,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_duration_secs: 60,
            fps: 60,
        }
    }
}

impl BufferConfig {
    /// Maximum number of frames the ring buffer will hold.
    pub fn capacity(&self) -> usize {
        (self.max_duration_secs as usize).saturating_mul(self.fps as usize)
    }

    /// Estimated bytes per frame at the given resolution.
    pub fn frame_size(&self, width: u32, height: u32) -> usize {
        // BGRA: 4 bytes per pixel
        (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4)
    }
}

/// Manages the ring buffer and frame pool for continuous recording.
pub struct BufferManager {
    buffer: RingBuffer,
    pool: FramePool,
    config: BufferConfig,
}

impl BufferManager {
    /// Create a new buffer manager with the given config and estimated resolution.
    pub fn new(config: BufferConfig, width: u32, height: u32) -> Self {
        let frame_size = config.frame_size(width, height);
        // Pre-allocate half the ring buffer's capacity to balance memory vs allocations
        let prealloc = (config.capacity() / 2).max(1);
        let pool = FramePool::new(frame_size, prealloc);
        let buffer = RingBuffer::new(config.capacity());

        Self {
            buffer,
            pool,
            config,
        }
    }

    /// Ingest a captured frame: push into ring buffer, release old data.
    pub fn push_frame(&mut self, frame: CapturedFrame) {
        // TODO: Optionally use pool for data allocation
        self.buffer.push(frame);
    }

    /// Save a clip from the last N seconds of buffer.
    pub fn clip(&self, duration: Duration) -> Vec<StoredFrame> {
        let now = std::time::Instant::now();
        self.buffer.clip_since(duration, now)
    }

    /// Save a clip from the entire buffer.
    pub fn clip_all(&self) -> Vec<StoredFrame> {
        self.buffer.all_frames()
    }

    /// Clear the buffer (e.g., on app pause / game switch).
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn config(&self) -> &BufferConfig {
        &self.config
    }

    pub fn frame_count(&self) -> usize {
        self.buffer.len()
    }
}
