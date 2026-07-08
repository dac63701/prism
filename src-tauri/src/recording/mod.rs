//! Recording engine — orchestrates the capture → ring-buffer → encoder pipeline.
//!
//! Provides [`Recorder`] as Tauri managed state so commands and other modules
//! can control recording and trigger clip saves.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use base64::{engine::general_purpose, Engine as _};
use image::codecs::jpeg::JpegEncoder;
use image::{imageops::FilterType, ImageBuffer, Rgb};

use crate::buffer::{BufferConfig, BufferManager, StoredFrame};
use crate::capture::{
    create_capture_backend, CaptureBackend, CaptureConfig, CaptureTarget, CapturedFrame,
};
use crate::encoder::codecs::{Codec, EncoderConfig};
use crate::encoder::create_encoder;
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

        Self {
            inner: Mutex::new(Some(RecorderInner {
                backend,
                buffer,
                backend_config,
                resolution: (1920, 1080),
                output_dir: resolve_output_dir(&settings.recording.output_directory),
                latest_frame: None,
            })),
            running: AtomicBool::new(false),
            polling_spawned: AtomicBool::new(false),
            frames_received: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Apply new settings at runtime (re-creates buffer, updates config).
    pub fn reconfigure(&self, settings: &AppSettings) {
        let rs = &settings.recording;
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
            let res = inner.resolution;
            // Update resolution from first frame
            if res == (0, 0) || res != (frame.width, frame.height) {
                inner.resolution = (frame.width, frame.height);
            }
            // Keep a copy for live preview (Arc clone is cheap)
            inner.latest_frame = Some(frame.clone());
            inner.buffer.push_frame(frame);
            self.frames_received.fetch_add(1, Ordering::SeqCst);
            1
        } else {
            0
        }
    }

    /// Calculate the sleep duration between polls based on settings FPS.
    pub fn poll_interval(&self) -> Duration {
        let fps = match self.inner.lock() {
            Ok(ref g) => g.as_ref().map(|i| i.backend_config.fps).unwrap_or(60),
            Err(_) => 60,
        };
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

    /// Total frames received since recording started.
    pub fn total_frames_received(&self) -> u64 {
        self.frames_received
            .load(std::sync::atomic::Ordering::SeqCst)
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
    /// Returns `None` if no frame has been captured yet.
    pub fn get_preview_frame(&self) -> Option<String> {
        let guard = self.inner.lock().ok()?;
        let inner = guard.as_ref()?;
        let frame = inner.latest_frame.as_ref()?;

        let width = frame.width;
        let height = frame.height;
        let stride = frame.stride;
        let data = frame.data.as_slice();

        // Downscale dimensions while maintaining aspect ratio
        let preview_w = Self::PREVIEW_MAX_WIDTH.min(width);
        let preview_h = (height as f64 * (preview_w as f64 / width as f64))
            .round()
            .max(1.0) as u32;

        // Convert BGRA → full-size RGB image buffer
        let mut rgb = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let offset = (y as usize * stride as usize + x as usize * 4)
                    .min(data.len().saturating_sub(4));
                let pixel = rgb.get_pixel_mut(x, y);
                pixel[0] = data[offset + 2]; // R ← B (source is BGRA)
                pixel[1] = data[offset + 1]; // G
                pixel[2] = data[offset]; // B ← R
            }
        }

        // Resize for preview
        let preview = image::imageops::resize(&rgb, preview_w, preview_h, FilterType::Triangle);

        // Encode as JPEG
        let mut jpg_buf = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut jpg_buf, 80);
        if encoder
            .encode(
                &preview,
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

    // ── Clip saving ──────────────────────────────────────────────────────

    /// Save a clip from the last N seconds of the ring buffer.
    ///
    /// Extracts frames from buffer, creates an encoder, writes MP4,
    /// and returns the output path on success.
    pub fn save_clip(&self, duration_secs: u32, settings: &AppSettings) -> Result<PathBuf, String> {
        let rs = &settings.recording;

        // 1. Extract frames from buffer (borrow briefly)
        let (frames, output_dir) = {
            let guard = self.inner.lock().map_err(|e| e.to_string())?;
            let inner = guard.as_ref().ok_or("Recorder not initialized")?;
            let frames = if duration_secs > 0 {
                inner.buffer.clip(Duration::from_secs(duration_secs as u64))
            } else {
                inner.buffer.clip_all()
            };
            (frames, inner.output_dir.clone())
        };

        if frames.is_empty() {
            return Err("No frames available to clip".into());
        }

        // 2. Build encoder config from settings
        let (target_width, target_height) = resolution_dimensions(&rs.resolution);

        let enc_config = EncoderConfig {
            codec: Codec::H264,
            bitrate_kbps: rs.bitrate_kbps,
            fps: rs.fps,
            keyframe_interval: rs.fps.saturating_mul(2), // keyframe every 2s
            target_width,
            target_height,
        };

        // 3. Generate output path
        let timestamp = chrono_now_formatted();
        let filename = format!("clip_{timestamp}.mp4");
        let output_path = output_dir.join(&filename);

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {e}"))?;

        // 4. Encode
        let mut encoder = create_encoder();
        encoder
            .encode_clip(&frames, &output_path, &enc_config)
            .map_err(|e| format!("Encoding failed: {e}"))?;

        Ok(output_path)
    }

    /// Save a clip with automatic duration from settings.
    pub fn save_clip_from_settings(&self, settings: &AppSettings) -> Result<PathBuf, String> {
        let duration = settings.recording.buffer_duration_secs;
        self.save_clip(duration, settings)
    }
}

// ── Clip data extraction (call under lock, encode outside) ───────────────

/// Data extracted from the recorder for encoding a clip.
pub struct ClipData {
    pub frames: Vec<StoredFrame>,
    pub output_dir: PathBuf,
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
