//! Recording engine — orchestrates the capture → ring-buffer → encoder pipeline.
//!
//! Provides [`Recorder`] as Tauri managed state so commands and other modules
//! can control recording and trigger clip saves.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use base64::{engine::general_purpose, Engine as _};
use image::ImageBuffer;

use crate::buffer::{BufferConfig, BufferManager, StoredFrame};
use crate::capture::{
    create_capture_backend, CaptureBackend, CaptureConfig, CaptureTarget, CapturedFrame,
};
#[cfg(target_os = "windows")]
use crate::encoder::windows::mf_encoder::MfH264Encoder;
use crate::settings::config::{resolution_dimensions, AppSettings};

/// Polling interval as a fraction of the frame duration,
/// so we don't busy-loop but still catch frames in time.
const POLL_FRACTION: f32 = 0.5;

/// Tauri-managed recording state.
///
/// Thread-safe: all mutable access goes through a single Mutex.
pub struct Recorder {
    inner: Mutex<Option<RecorderInner>>,
    /// Flag readable without the lock — quick state check.
    running: AtomicBool,
    /// Prevents spawning multiple polling tasks.
    polling_spawned: AtomicBool,
    /// Total frames ever received from capture backend (diagnostics).
    frames_received: std::sync::atomic::AtomicU64,
    /// Cached FPS to avoid lock contention in the polling loop.
    cached_fps: AtomicU32,
}

struct RecorderInner {
    backend: Box<dyn CaptureBackend>,
    buffer: BufferManager,
    backend_config: CaptureConfig,
    /// (width, height) — updated on first frame
    resolution: (u32, u32),
    /// Where encoded clips are written
    output_dir: PathBuf,
    /// Most recently captured frame (for live preview)
    latest_frame: Option<CapturedFrame>,
    /// H.264 hardware encoder for the shadow buffer (Windows only).
    #[cfg(target_os = "windows")]
    h264_encoder: Option<MfH264Encoder>,
    /// Frame index for the H.264 encoder (Windows only).
    #[cfg(target_os = "windows")]
    frame_index: u64,
    /// Cached SPS NAL unit (AVCC format) from the H.264 encoder.
    #[cfg(target_os = "windows")]
    sps: Vec<u8>,
    /// Cached PPS NAL unit (AVCC format) from the H.264 encoder.
    #[cfg(target_os = "windows")]
    pps: Vec<u8>,
    /// Monotonic timestamp when recording started (for elapsed-time display).
    recording_started_at: Option<std::time::Instant>,
}

impl Recorder {
    /// Create a new recorder from app settings.
    pub fn new(settings: &AppSettings) -> Self {
        let rs = &settings.recording;
        let buffer = BufferManager::new(
            BufferConfig {
                max_duration_secs: rs.buffer_duration_secs,
                fps: rs.fps,
            },
            1920,
            1080,
        );
        let backend = create_capture_backend();
        // Parse capture target from settings (JSON-serialized string)
        let target = if rs.capture_target.is_empty() {
            CaptureTarget::default()
        } else {
            serde_json::from_str(&rs.capture_target).unwrap_or_default()
        };
        let backend_config = CaptureConfig {
            fps: rs.fps,
            capture_cursor: true,
            target,
        };

        #[cfg(target_os = "windows")]
        let h264_encoder = {
            let (w, h) = resolution_dimensions(&rs.resolution);
            match MfH264Encoder::new(w, h, rs.fps, rs.bitrate_kbps, rs.fps.saturating_mul(2)) {
                Ok(enc) => Some(enc),
                Err(e) => {
                    eprintln!("[prism] H.264 encoder init failed — falling back to raw NV12: {e}");
                    None
                }
            }
        };

        Self {
            inner: Mutex::new(Some(RecorderInner {
                backend,
                buffer,
                backend_config,
                resolution: (1920, 1080),
                output_dir: resolve_output_dir(&settings.recording.output_directory),
                latest_frame: None,
                #[cfg(target_os = "windows")]
                h264_encoder,
                #[cfg(target_os = "windows")]
                frame_index: 0,
                #[cfg(target_os = "windows")]
                sps: Vec::new(),
                #[cfg(target_os = "windows")]
                pps: Vec::new(),
                recording_started_at: None,
            })),
            running: AtomicBool::new(false),
            polling_spawned: AtomicBool::new(false),
            frames_received: std::sync::atomic::AtomicU64::new(0),
            cached_fps: AtomicU32::new(rs.fps),
        }
    }

    /// Apply new settings at runtime (re-creates buffer, updates config).
    pub fn reconfigure(&self, settings: &AppSettings) {
        let rs = &settings.recording;
        self.cached_fps.store(rs.fps, Ordering::SeqCst);
        let mut guard = self.inner.lock().expect("recorder lock poisoned");
        if let Some(inner) = guard.as_mut() {
            // Update output directory
            inner.output_dir = resolve_output_dir(&rs.output_directory);
            // Rebuild buffer with new capacity
            inner.buffer = BufferManager::new(
                BufferConfig {
                    max_duration_secs: rs.buffer_duration_secs,
                    fps: rs.fps,
                },
                inner.resolution.0,
                inner.resolution.1,
            );
            // Rebuild H.264 encoder with new settings
            #[cfg(target_os = "windows")]
            {
                let (w, h) = resolution_dimensions(&rs.resolution);
                inner.frame_index = 0;
                inner.sps.clear();
                inner.pps.clear();
                inner.h264_encoder = match MfH264Encoder::new(
                    w, h, rs.fps, rs.bitrate_kbps, rs.fps.saturating_mul(2),
                ) {
                    Ok(enc) => Some(enc),
                    Err(e) => {
                        eprintln!("[prism] H.264 encoder reinit failed — falling back to raw NV12: {e}");
                        None
                    }
                };
            }
            // Update capture config
            inner.backend_config.fps = rs.fps;
        }
    }

    /// Update the capture target (display/window/application) at runtime.
    /// Does not restart the capture — call before starting or stop/start manually.
    pub fn reconfigure_target(&self, target: CaptureTarget) {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(inner) = guard.as_mut() {
                inner.backend_config.target = target;
            }
        }
    }

    // ── Recording control ────────────────────────────────────────────────

    /// Start the capture backend and mark as recording.
    /// Safe to call multiple times — no-ops if already recording.
    pub fn start_recording(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut guard = self.inner.lock().expect("recorder lock poisoned");
        let inner = guard.as_mut().ok_or("Recorder not initialized")?;

        inner
            .backend
            .start(inner.backend_config.clone())
            .map_err(|e| format!("Failed to start capture: {e}"))?;

        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Stop the capture backend and mark as stopped.
    /// Resets the polling-spawned flag so the next `start_recording` can
    /// re-create the background frame-polling task.
    pub fn stop_recording(&self) -> Result<(), String> {
        self.running.store(false, Ordering::SeqCst);
        self.polling_spawned.store(false, Ordering::SeqCst);

        let mut guard = self.inner.lock().expect("recorder lock poisoned");
        if let Some(inner) = guard.as_mut() {
            inner
                .backend
                .stop()
                .map_err(|e| format!("Failed to stop capture: {e}"))?;
        }
        Ok(())
    }

    /// Check whether recording is active (atomic, no lock).
    pub fn is_recording(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Spawn the background polling task if not already spawned.
    /// Returns true if the task was spawned, false if already running.
    pub fn start_polling(&self, app: AppHandle) -> bool {
        if self.polling_spawned.swap(true, Ordering::SeqCst) {
            return false; // already spawned
        }

        let app_handle = app;
        tauri::async_runtime::spawn(async move {
            loop {
                let (running, interval) = {
                    let state = app_handle.state::<std::sync::Mutex<Recorder>>();
                    let guard = match state.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    if !guard.is_recording() {
                        (false, std::time::Duration::ZERO)
                    } else {
                        guard.poll_and_push();
                        (true, guard.poll_interval())
                    }
                };
                if !running {
                    break;
                }
                tokio::time::sleep(interval).await;
            }
        });

        true
    }

    /// Clear the ring buffer (e.g. on game switch).
    pub fn clear_buffer(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            if let Some(inner) = guard.as_mut() {
                inner.buffer.clear();
            }
        }
    }

    // ── Polling (called from background task) ────────────────────────────

    /// Get the buffer duration in seconds.
    pub fn buffer_duration_secs(&self) -> u32 {
        self.inner
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .map(|inner| inner.buffer.config().max_duration_secs)
            })
            .unwrap_or(60)
    }

    /// Poll the capture backend for a new frame and push to buffer.
    /// Returns the number of frames polled (0 or 1).
    /// Should be called in a loop with appropriate timing.
    pub fn poll_and_push(&self) -> u32 {
        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return 0,
        };
        let inner = match guard.as_mut() {
            Some(i) => i,
            None => return 0,
        };

if let Some(frame) = inner.backend.read_latest_frame() {
            // Mark recording start on first frame
            if inner.recording_started_at.is_none() {
                inner.recording_started_at = Some(std::time::Instant::now());
            }

            let res = inner.resolution;
            // Update resolution from first frame
            if res == (0, 0) || res != (frame.width, frame.height) {
                inner.resolution = (frame.width, frame.height);
            }
            // Keep a copy for live preview (Arc clone is cheap)
            inner.latest_frame = Some(frame.clone());

            #[cfg(target_os = "windows")]
            {
                // Encode NV12 frame → compressed H.264 packets → ring buffer
                if let Some(ref mut encoder) = inner.h264_encoder {
                    match encoder.encode_frame(&frame.data) {
                        Ok(packets) => {
                            // Capture SPS/PPS from encoder if not yet available
                            if inner.sps.is_empty() && encoder.sps_pps_ready() {
                                inner.sps = encoder.sps().to_vec();
                                inner.pps = encoder.pps().to_vec();
                                eprintln!(
                                    "[prism] captured SPS({}) PPS({})",
                                    inner.sps.len(),
                                    inner.pps.len()
                                );
                            }

                            for pkt in packets {
                                let stored = StoredFrame {
                                    data: std::sync::Arc::new(pkt.data),
                                    width: frame.width,
                                    height: frame.height,
                                    stride: 0,
                                    pixel_format: crate::capture::PixelFormat::H264,
                                    timestamp: frame.timestamp,
                                    is_sync: pkt.is_sync,
                                };
                                inner.buffer.push_frame(stored);
                            }
                        }
                        Err(e) => {
                            // Encoding failed — store the raw NV12 frame as fallback
                            eprintln!("H.264 encode error (falling back to raw): {e}");
                            inner.buffer.push_frame(frame);
                        }
                    }
                } else {
                    // No encoder available — store raw
                    inner.buffer.push_frame(frame);
                }
                inner.frame_index += 1;
            }

            #[cfg(not(target_os = "windows"))]
            inner.buffer.push_frame(frame);

            self.frames_received.fetch_add(1, Ordering::SeqCst);
            1
        } else {
            0
        }
    }

    /// Calculate the sleep duration between polls based on settings FPS.
    /// Uses the cached atomic FPS to avoid lock contention.
    pub fn poll_interval(&self) -> Duration {
        let fps = self.cached_fps.load(Ordering::Relaxed);
        if fps == 0 {
            return Duration::from_millis(16);
        }
        let frame_ms = 1000.0 / fps as f32;
        Duration::from_secs_f32(frame_ms * POLL_FRACTION / 1000.0)
    }

    /// Whether a preview frame is available.
    pub fn preview_available(&self) -> bool {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|inner| inner.latest_frame.is_some()))
            .unwrap_or(false)
    }

    /// Cached FPS value (atomic, no lock).
    pub fn cached_fps(&self) -> u32 {
        self.cached_fps.load(Ordering::Relaxed)
    }

    /// Total frames received since recording started.
    pub fn total_frames_received(&self) -> u64 {
        self.frames_received
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Seconds elapsed since recording started (0 if not recording).
    pub fn recording_elapsed_secs(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref().and_then(|inner| {
                    inner
                        .recording_started_at
                        .map(|t| t.elapsed().as_secs_f64())
                })
            })
            .unwrap_or(0.0)
    }

    /// Seconds of buffer time available (frame_count / fps).
    pub fn buffer_time_secs(&self) -> f64 {
        let fps = self.cached_fps.load(Ordering::Relaxed);
        if fps == 0 {
            return 0.0;
        }
        let fc = self.frame_count();
        fc as f64 / fps as f64
    }

    /// Get the current buffer frame count (for diagnostics).
    pub fn frame_count(&self) -> usize {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|i| i.buffer.frame_count()))
            .unwrap_or(0)
    }

    // ── Live preview ─────────────────────────────────────────────────────

    /// Maximum width for the preview JPEG (maintains aspect ratio).
    const PREVIEW_MAX_WIDTH: u32 = 1280;

    /// Encode the latest captured frame as a JPEG data URL for frontend preview.
    ///
    /// Handles both NV12 (chroma-subsampled, the ring-buffer format) and BGRA
    /// (legacy macOS) sources. Uses point-sampled downscaling in a single pass.
    ///
    /// Returns `None` if no frame has been captured yet.
    pub fn get_preview_frame(&self) -> Option<String> {
        let guard = self.inner.lock().ok()?;
        let inner = guard.as_ref()?;
        let frame = inner.latest_frame.as_ref()?;

        let width = frame.width;
        let height = frame.height;
        let stride = frame.stride;
        let data = frame.data.as_slice();
        let fmt = frame.pixel_format;

        // Downscale dimensions while maintaining aspect ratio
        let preview_w = Self::PREVIEW_MAX_WIDTH.min(width);
        let preview_h = (height as f64 * (preview_w as f64 / width as f64))
            .round()
            .max(1.0) as u32;

        let mut rgb = ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(preview_w, preview_h);

        match fmt {
            crate::capture::PixelFormat::Nv12 => {
                let y_plane = data;
                let y_size = (width * height) as usize;
                let uv_plane = &data[y_size..];

                for dy in 0..preview_h {
                    for dx in 0..preview_w {
                        let sx = (dx * width) / preview_w;
                        let sy = (dy * height) / preview_h;

                        let y_off = (sy * stride + sx) as usize;
                        let uv_off = ((sy / 2) * (stride / 2) + (sx / 2)) as usize * 2;

                        let y_val = y_plane[y_off] as i32 - 16;
                        let u_val = uv_plane[uv_off] as i32 - 128;
                        let v_val = uv_plane[uv_off + 1] as i32 - 128;

                        let r = ((298 * y_val + 409 * v_val + 128) >> 8).clamp(0, 255) as u8;
                        let g = ((298 * y_val - 100 * u_val - 208 * v_val + 128) >> 8).clamp(0, 255) as u8;
                        let b = ((298 * y_val + 516 * u_val + 128) >> 8).clamp(0, 255) as u8;

                        let pixel = rgb.get_pixel_mut(dx, dy);
                        pixel[0] = r;
                        pixel[1] = g;
                        pixel[2] = b;
                    }
                }
            }
            crate::capture::PixelFormat::Bgra => {
                for dy in 0..preview_h {
                    for dx in 0..preview_w {
                        let sx = (dx * width) / preview_w;
                        let sy = (dy * height) / preview_h;
                        let offset = (sy as usize * stride as usize + sx as usize * 4)
                            .min(data.len().saturating_sub(4));
                        let pixel = rgb.get_pixel_mut(dx, dy);
                        pixel[0] = data[offset + 2]; // R ← B
                        pixel[1] = data[offset + 1]; // G
                        pixel[2] = data[offset];     // B ← R
                    }
                }
            }
            crate::capture::PixelFormat::H264 => {
                // H.264 frames are compressed — can't render as preview.
                // The preview path stores the latest decoded NV12 frame
                // separately, so this arm is only for exhaustiveness.
            }
        }

        // Encode to JPEG
        let mut jpg_buf = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg_buf, 80);
        if encoder
            .encode(
                &rgb,
                preview_w,
                preview_h,
                image::ExtendedColorType::Rgb8,
            )
            .is_err()
        {
            return None;
        }

        let b64 = general_purpose::STANDARD.encode(&jpg_buf);
        Some(format!("data:image/jpeg;base64,{b64}"))
    }

}

// ── Clip data extraction (call under lock, encode outside) ───────────────

/// Data extracted from the recorder for encoding a clip.
pub struct ClipData {
    pub frames: Vec<StoredFrame>,
    pub output_dir: PathBuf,
    /// Cached SPS NAL unit (AVCC format) from the H.264 encoder.
    pub sps: Vec<u8>,
    /// Cached PPS NAL unit (AVCC format) from the H.264 encoder.
    pub pps: Vec<u8>,
}

impl Recorder {
    /// Extract clip frames and metadata from the ring buffer.
    ///
    /// This is the ONLY operation that needs the recorder lock.
    /// Encoding should happen AFTER releasing the lock.
    pub fn extract_clip_data(&self, duration_secs: u32) -> Result<ClipData, String> {
        let guard = self.inner.lock().map_err(|e| e.to_string())?;
        let inner = guard.as_ref().ok_or("Recorder not initialized")?;
        let frames = if duration_secs > 0 {
            inner.buffer.clip(Duration::from_secs(duration_secs as u64))
        } else {
            inner.buffer.clip_all()
        };
        if frames.is_empty() {
            return Err("No frames available to clip".into());
        }
        Ok(ClipData {
            frames,
            output_dir: inner.output_dir.clone(),
            #[cfg(target_os = "windows")]
            sps: inner.sps.clone(),
            #[cfg(target_os = "windows")]
            pps: inner.pps.clone(),
            #[cfg(not(target_os = "windows"))]
            sps: Vec::new(),
            #[cfg(not(target_os = "windows"))]
            pps: Vec::new(),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Resolve the output directory: use user-configured path or default to Videos/Prism.
pub(crate) fn resolve_output_dir(configured: &str) -> PathBuf {
    if !configured.is_empty() {
        return PathBuf::from(configured);
    }
    // Default: ~/Videos/Prism (or platform equivalent)
    dirs::video_dir()
        .map(|d| d.join("Prism"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Get a formatted timestamp string for filenames.
pub(crate) fn chrono_now_formatted() -> String {
    // Simple ISO-like without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // Format as YYYYMMDD_HHMMSS
    const SECS_PER_DAY: u64 = 86400;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_MIN: u64 = 60;

    // Days since epoch
    let days = secs / SECS_PER_DAY;
    let rem = secs % SECS_PER_DAY;
    let hours = rem / SECS_PER_HOUR;
    let rem = rem % SECS_PER_HOUR;
    let mins = rem / SECS_PER_MIN;
    let secs_rem = rem % SECS_PER_MIN;

    // Approximate year (not perfect but good enough for filenames)
    let year = 1970 + (days as f64 / 365.25) as u64;
    // Approximate month/day
    let remaining_days = days - ((year - 1970) * 365 + ((year - 1969) / 4));
    let month = 1 + remaining_days / 28;
    let day = 1 + remaining_days % 28;

    format!(
        "{y:04}{m:02}{d:02}_{h:02}{min:02}{s:02}",
        y = year.min(9999),
        m = month.min(12),
        d = day.min(31),
        h = hours,
        min = mins,
        s = secs_rem
    )
}
