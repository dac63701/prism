//! System audio capture (Windows) — WASAPI loopback → PCM ring buffer.
//!
//! Captures the default render device in loopback mode so clip MP4s can carry
//! system audio (games, Discord, browsers). PCM is stored as interleaved
//! float32 stereo in a byte-accounted ring buffer alongside an `Instant`
//! timestamp per block, allowing clip-save to extract the exact window that
//! matches the video frames.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use wasapi::{initialize_mta, Direction, SampleType, StreamMode, WaveFormat};

pub mod aac;

/// Default capture format: int16 PCM, 48 kHz, stereo.
pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;
/// Bytes per sample frame (int16 × 2 channels).
pub const BYTES_PER_FRAME: usize = (CHANNELS as usize) * 2;

/// One contiguous PCM chunk captured from the loopback device.
struct AudioBlock {
    /// Monotonic time when this chunk was read from WASAPI.
    timestamp: Instant,
    /// Interleaved float32 stereo PCM.
    data: Vec<u8>,
}

/// Byte-accounted FIFO of PCM blocks, evicting the oldest data first.
pub struct AudioRingBuffer {
    blocks: VecDeque<AudioBlock>,
    total_bytes: usize,
    max_bytes: usize,
    sample_rate: u32,
    bytes_per_frame: usize,
}

impl AudioRingBuffer {
    pub fn new(max_bytes: usize, sample_rate: u32, channels: u16) -> Self {
        Self {
            blocks: VecDeque::new(),
            total_bytes: 0,
            max_bytes,
            sample_rate,
            bytes_per_frame: (channels as usize) * 2,
        }
    }

    pub fn push(&mut self, timestamp: Instant, data: Vec<u8>) {
        let frame_len = data.len();
        while self.total_bytes + frame_len > self.max_bytes && !self.blocks.is_empty() {
            if let Some(old) = self.blocks.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(old.data.len());
            }
        }
        self.total_bytes += frame_len;
        self.blocks.push_back(AudioBlock { timestamp, data });
    }

    /// Extract the PCM window `[start, end)` (sample-accurate at both edges).
    /// Returns `None` when no audio overlaps the window.
    pub fn extract(&self, start: Instant, end: Instant) -> Option<Vec<u8>> {
        if end <= start {
            return None;
        }
        let frame_rate = self.sample_rate as f64;
        let mut out = Vec::new();
        for block in &self.blocks {
            let block_dur = Duration::from_secs_f64(
                block.data.len() as f64 / self.bytes_per_frame as f64 / frame_rate,
            );
            let block_end = block.timestamp + block_dur;
            if block_end <= start || block.timestamp >= end {
                continue;
            }
            // Overlap window clamped into this block.
            let ov_start = start.max(block.timestamp);
            let ov_end = end.min(block_end);
            // Frame offset into the block (float32 interleaved).
            let frame_offset = (ov_start.duration_since(block.timestamp).as_secs_f64() * frame_rate)
                .round() as usize;
            let frame_count = ((ov_end.duration_since(ov_start).as_secs_f64() * frame_rate).round()
                as usize)
                .max(1);
            let byte_off = frame_offset.saturating_mul(self.bytes_per_frame);
            let byte_len = frame_count
                .saturating_mul(self.bytes_per_frame)
                .min(block.data.len().saturating_sub(byte_off));
            if byte_len > 0 {
                out.extend_from_slice(&block.data[byte_off..byte_off + byte_len]);
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.total_bytes = 0;
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Wall-clock span of stored audio in seconds (oldest to newest).
    #[allow(dead_code)]
    pub fn time_span_secs(&self) -> f64 {
        match (self.blocks.front(), self.blocks.back()) {
            (Some(first), Some(last)) => {
                last.timestamp.duration_since(first.timestamp).as_secs_f64()
            }
            _ => 0.0,
        }
    }
}

/// Manages the WASAPI loopback capture thread and the shared PCM ring buffer.
pub struct AudioCapturer {
    ring: Arc<Mutex<AudioRingBuffer>>,
    running: Arc<AtomicBool>,
    spawned: AtomicBool,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    sample_rate: u32,
    channels: u16,
}

impl Default for AudioCapturer {
    fn default() -> Self {
        Self::new(SAMPLE_RATE, CHANNELS)
    }
}

impl AudioCapturer {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        // Budget for `buffer_duration_secs` of audio plus a couple seconds of
        // slack, capped at 256 MB so a pathological duration can't blow up.
        let ring = Arc::new(Mutex::new(AudioRingBuffer::new(
            Self::byte_budget(60),
            sample_rate,
            channels,
        )));
        Self {
            ring,
            running: Arc::new(AtomicBool::new(false)),
            spawned: AtomicBool::new(false),
            thread: Mutex::new(None),
            sample_rate,
            channels,
        }
    }

    fn byte_budget(buffer_duration_secs: u32) -> usize {
        let bytes_per_sec = (SAMPLE_RATE as usize) * BYTES_PER_FRAME;
        let target = (buffer_duration_secs as usize)
            .saturating_add(4)
            .saturating_mul(bytes_per_sec);
        target.min(256 * 1024 * 1024)
    }

    /// Start the capture thread (no-op if already running). Resizes the ring
    /// budget to cover `buffer_duration_secs` of audio.
    pub fn start(&self, buffer_duration_secs: u32) {
        if self.spawned.swap(true, Ordering::SeqCst) {
            return;
        }
        self.ring.lock().clear();
        *self.ring.lock() = AudioRingBuffer::new(
            Self::byte_budget(buffer_duration_secs),
            self.sample_rate,
            self.channels,
        );

        self.running.store(true, Ordering::SeqCst);
        let ring = Arc::clone(&self.ring);
        let running = Arc::clone(&self.running);

        let sample_rate = self.sample_rate;
        let channels = self.channels;

        let handle = thread::Builder::new()
            .name("prism-audio".into())
            .spawn(move || {
                if let Err(error) = capture_loop(ring, running, sample_rate, channels) {
                    eprintln!("[prism-audio] capture ended: {error}");
                }
            })
            .expect("failed to spawn audio capture thread");
        *self.thread.lock() = Some(handle);
    }

    /// Stop the capture thread and clear buffered audio.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().take() {
            let _ = handle.join();
        }
        self.ring.lock().clear();
        self.spawned.store(false, Ordering::SeqCst);
    }

    /// Clear buffered audio without stopping capture.
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.ring.lock().clear();
    }

    /// Extract the PCM window `[start, end)` from the ring buffer.
    #[allow(dead_code)]
    pub fn extract(&self, start: Instant, end: Instant) -> Option<Vec<u8>> {
        self.ring.lock().extract(start, end)
    }

    /// Clone of the shared ring buffer handle, so callers can extract audio
    /// without holding the recorder lock for the duration of the copy.
    pub fn ring_handle(&self) -> Arc<parking_lot::Mutex<AudioRingBuffer>> {
        Arc::clone(&self.ring)
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[allow(dead_code)]
    pub fn channels(&self) -> u16 {
        self.channels
    }
}

/// WASAPI loopback capture loop. Reads packets on a polling timer and pushes
/// float32 PCM into the shared ring buffer.
fn capture_loop(
    ring: Arc<Mutex<AudioRingBuffer>>,
    running: Arc<AtomicBool>,
    sample_rate: u32,
    channels: u16,
) -> Result<(), String> {
    initialize_mta()
        .ok()
        .map_err(|error| format!("COM init failed: {error}"))?;

    // Loopback: default render endpoint, initialized as a capture stream.
    let format = WaveFormat::new(
        16,
        16,
        &SampleType::Int,
        sample_rate as usize,
        channels as usize,
        None,
    );
    let enumerator =
        wasapi::DeviceEnumerator::new().map_err(|error| format!("enumerator failed: {error}"))?;
    let device = enumerator
        .get_default_device(&Direction::Render)
        .map_err(|error| format!("default render device failed: {error}"))?;
    let mut client = device
        .get_iaudioclient()
        .map_err(|error| format!("audio client failed: {error}"))?;

    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|error| format!("loopback init failed: {error}"))?;
    let capture = client
        .get_audiocaptureclient()
        .map_err(|error| format!("capture client failed: {error}"))?;
    client
        .start_stream()
        .map_err(|error| format!("start stream failed: {error}"))?;

    let bytes_per_frame = (channels as usize) * 2;
    while running.load(Ordering::SeqCst) {
        let packet = capture
            .get_next_packet_size()
            .map_err(|error| format!("packet size failed: {error}"))?
            .unwrap_or_default();
        if packet > 0 {
            let mut buf = vec![0u8; packet as usize * bytes_per_frame];
            let (frames, _) = capture
                .read_from_device(&mut buf)
                .map_err(|error| format!("read failed: {error}"))?;
            buf.truncate(frames as usize * bytes_per_frame);
            ring.lock().push(Instant::now(), buf);
        }
        thread::sleep(Duration::from_millis(20));
    }

    let _ = client.stop_stream();
    Ok(())
}

impl Drop for AudioCapturer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_evicts_oldest_bytes() {
        let mut ring = AudioRingBuffer::new(10 * BYTES_PER_FRAME, SAMPLE_RATE, CHANNELS);
        let now = Instant::now();
        ring.push(now, vec![0u8; 8 * BYTES_PER_FRAME]);
        ring.push(
            now + Duration::from_millis(1),
            vec![0u8; 8 * BYTES_PER_FRAME],
        );
        assert_eq!(ring.len(), 1, "oldest block should be evicted");
    }

    #[test]
    fn extract_returns_only_overlapping_window() {
        let mut ring = AudioRingBuffer::new(usize::MAX, SAMPLE_RATE, CHANNELS);
        let now = Instant::now();
        // 1 second of 48 kHz float32 stereo = 48_000 frames * 8 bytes.
        let one_sec = vec![0u8; 48_000 * BYTES_PER_FRAME];
        ring.push(now, one_sec);

        // 500 ms window starting at +250 ms → 250 ms of samples.
        let start = now + Duration::from_millis(250);
        let end = start + Duration::from_millis(500);
        let out = ring.extract(start, end).unwrap();
        let expected_frames = (48_000.0_f64 * 0.5).round() as usize;
        assert_eq!(out.len(), expected_frames * BYTES_PER_FRAME);
    }

    #[test]
    fn extract_none_outside_window() {
        let mut ring = AudioRingBuffer::new(usize::MAX, SAMPLE_RATE, CHANNELS);
        let now = Instant::now();
        ring.push(now, vec![0u8; 48_000 * BYTES_PER_FRAME]);
        assert!(ring
            .extract(now + Duration::from_secs(5), now + Duration::from_secs(6))
            .is_none());
    }

    #[test]
    fn extract_stitches_adjacent_blocks() {
        let mut ring = AudioRingBuffer::new(usize::MAX, SAMPLE_RATE, CHANNELS);
        let now = Instant::now();
        let half = vec![0u8; 24_000 * BYTES_PER_FRAME]; // 500 ms
        ring.push(now, half.clone());
        ring.push(now + Duration::from_millis(500), half);

        let out = ring
            .extract(now, now + Duration::from_millis(1_000))
            .unwrap();
        assert_eq!(out.len(), 48_000 * BYTES_PER_FRAME);
    }
}
