//! Ring buffer for keeping the last N seconds of gameplay.
//!
//! The [`BufferManager`] orchestrates frame ingestion from a capture backend
//! and enables saving clips on demand (hotkey trigger, moment detection).

pub mod ring;

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

    /// NV12 frame size at the given resolution: width × height × 1.5.
    /// This is the size of raw frames when the encoder is unavailable
    /// (fallback path). Compressed H.264 packets are ~10 KB and the
    /// byte-budgeted ring buffer handles both cases correctly.
    #[allow(dead_code)]
    pub fn frame_size(&self, width: u32, height: u32) -> usize {
        (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(3)
            .saturating_div(2)
    }
}

/// Manages the ring buffer and frame pool for continuous recording.
pub struct BufferManager {
    buffer: RingBuffer,
    config: BufferConfig,
}

impl BufferManager {
    /// Maximum memory budget for the shadow buffer (256 MB).
    /// The ring buffer uses byte-accounted eviction to automatically stay
    /// within this budget regardless of whether frames are compressed H.264
    /// (~10 KB) or raw NV12 (~3 MB at 1080p).
    const SHADOW_BUFFER_BYTES: usize = 256 * 1024 * 1024;

    /// Hard frame-count ceiling — prevents unbounded slot allocation even
    /// with small frame sizes. 7 minutes of 60 fps = 25,200 frames.
    const MAX_FRAME_CAPACITY: usize = 30_000;

    /// Create a new buffer manager with the given config and estimated resolution.
    pub fn new(config: BufferConfig, _width: u32, _height: u32) -> Self {
        // Frame-capacity ceiling: at most MAX_FRAME_CAPACITY or config capacity,
        // whichever is smaller.
        let capacity = config.capacity().clamp(60, Self::MAX_FRAME_CAPACITY);
        let buffer = RingBuffer::with_byte_budget(capacity, Self::SHADOW_BUFFER_BYTES);

        Self { buffer, config }
    }

    /// Push a frame into the ring buffer.
    /// Accepts either `CapturedFrame` (raw capture data) or `StoredFrame` (encoded packet).
    pub fn push_frame(&mut self, frame: impl Into<StoredFrame>) {
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

    /// Scan all buffered H.264 frames for SPS/PPS NAL units.
    pub fn find_sps_pps_anywhere(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        self.buffer.find_sps_pps_anywhere()
    }
}
