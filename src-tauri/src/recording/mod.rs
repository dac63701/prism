//! Recording engine — orchestrates the capture → ring-buffer → encoder pipeline.
//!
//! Provides [`Recorder`] as Tauri managed state so commands and other modules
//! can control recording and trigger clip saves.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use base64::{engine::general_purpose, Engine as _};
use image::ImageBuffer;

use crate::buffer::{BufferConfig, BufferManager, StoredFrame};
use crate::capture::{
    create_capture_backend, CaptureBackend, CaptureConfig, CaptureTarget, CapturedFrame,
};
#[cfg(target_os = "macos")]
use crate::encoder::macos::resize_bgra_frame;
#[cfg(target_os = "macos")]
use crate::encoder::macos::vt_encoder::VtH264Encoder;
#[cfg(target_os = "windows")]
use crate::encoder::windows::mf_encoder::MfH264Encoder;
use crate::settings::config::AppSettings;
use crate::settings::config::{is_native_resolution, resolution_dimensions};

/// Polling interval as a fraction of the frame duration,
/// so we don't busy-loop but still catch frames in time.
const POLL_FRACTION: f32 = 1.0;

/// Whether per-frame timing diagnostics are enabled (`PRISM_PROF=1`).
/// Emits phase timings from `poll_and_push` and clip-save to stderr.
fn prof_enabled() -> bool {
    std::env::var("PRISM_PROF").as_deref() == Ok("1")
}

/// Resolve the effective capture FPS. When `fps_auto` is set, match the main
/// display's refresh rate (clamped to a sane 24–240 range); fall back to the
/// configured manual FPS if detection fails.
fn effective_fps(rs: &crate::settings::config::RecordingSettings) -> u32 {
    if rs.fps_auto {
        let detected = crate::capture::primary_display_refresh_rate();
        if (24..=240).contains(&detected) {
            return detected;
        }
        eprintln!(
            "[recording] display refresh rate detection failed ({detected}); using {} FPS",
            rs.fps
        );
    }
    rs.fps
}

/// Tauri-managed recording state.
///
/// Thread-safe: all mutable access goes through a single parking_lot Mutex.
pub struct Recorder {
    inner: parking_lot::Mutex<Option<RecorderInner>>,
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
    /// H.264 hardware encoder for the shadow buffer.
    #[cfg(target_os = "windows")]
    h264_encoder: Option<MfH264Encoder>,
    /// Avoid retrying encoder creation for every captured frame after a
    /// deterministic Media Foundation initialization failure.
    #[cfg(target_os = "windows")]
    h264_encoder_init_failed: bool,
    #[cfg(target_os = "macos")]
    h264_encoder: Option<VtH264Encoder>,
    /// Frame index for the H.264 encoder.
    #[cfg(target_os = "windows")]
    frame_index: u64,
    #[cfg(target_os = "macos")]
    frame_index: u64,
    /// Cached SPS NAL unit (AVCC format) from the H.264 encoder.
    #[cfg(target_os = "windows")]
    sps: Vec<u8>,
    #[cfg(target_os = "macos")]
    sps: Vec<u8>,
    /// Cached PPS NAL unit (AVCC format) from the H.264 encoder.
    #[cfg(target_os = "windows")]
    pps: Vec<u8>,
    #[cfg(target_os = "macos")]
    pps: Vec<u8>,
    /// Monotonic timestamp when recording started (for elapsed-time display).
    recording_started_at: Option<std::time::Instant>,
    /// Uses capture-source dimensions; encoder init deferred until first frame.
    resolution_is_native: bool,
    /// Stored bitrate used for encoder lazy-init.
    native_bitrate_kbps: u32,
    /// Target encoder dimensions (from settings for non-native, from first frame for native).
    target_width: u32,
    target_height: u32,
    /// WASAPI system audio capture (Windows only).
    #[cfg(target_os = "windows")]
    audio: crate::audio::AudioCapturer,
    /// Whether audio capture is enabled by settings.
    #[cfg(target_os = "windows")]
    capture_audio: bool,
}

impl Recorder {
    /// Create a new recorder from app settings.
    pub fn new(settings: &AppSettings) -> Self {
        let rs = &settings.recording;
        let fps = effective_fps(rs);
        let buffer = BufferManager::new(
            BufferConfig {
                max_duration_secs: rs.buffer_duration_secs,
                fps,
                bitrate_kbps: rs.bitrate_kbps,
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
        let native = is_native_resolution(&rs.resolution);
        // Configured output dimensions (0,0 = native). The Windows backend now
        // honors these directly (GPU/CPU downscale); macOS resizes in-process.
        let (target_w, target_h) = if native {
            (0, 0)
        } else {
            resolution_dimensions(&rs.resolution)
        };
        let backend_config = CaptureConfig {
            fps: rs.fps,
            capture_cursor: true,
            target,
            target_width: target_w,
            target_height: target_h,
        };

        // Windows needs the captured frame dimensions for native resolution,
        // so initialize the MFT lazily on the first frame for every setting.
        #[cfg(target_os = "windows")]
        let h264_encoder = None;

        #[cfg(target_os = "macos")]
        let h264_encoder = if native {
            None
        } else {
            match VtH264Encoder::new(target_w, target_h, rs.fps, rs.bitrate_kbps, rs.fps) {
                Ok(enc) => Some(enc),
                Err(e) => {
                    eprintln!(
                        "[prism] VT H.264 encoder init failed — falling back to raw NV12: {e}"
                    );
                    None
                }
            }
        };

        Self {
            inner: parking_lot::Mutex::new(Some(RecorderInner {
                backend,
                buffer,
                backend_config,
                resolution: (1920, 1080),
                output_dir: resolve_output_dir(&settings.recording.output_directory),
                latest_frame: None,
                #[cfg(target_os = "windows")]
                h264_encoder,
                #[cfg(target_os = "windows")]
                h264_encoder_init_failed: false,
                #[cfg(target_os = "macos")]
                h264_encoder,
                #[cfg(target_os = "windows")]
                frame_index: 0,
                #[cfg(target_os = "macos")]
                frame_index: 0,
                #[cfg(target_os = "windows")]
                sps: Vec::new(),
                #[cfg(target_os = "macos")]
                sps: Vec::new(),
                #[cfg(target_os = "windows")]
                pps: Vec::new(),
                #[cfg(target_os = "macos")]
                pps: Vec::new(),
                recording_started_at: None,
                resolution_is_native: native,
                native_bitrate_kbps: rs.bitrate_kbps,
                target_width: target_w,
                target_height: target_h,
                #[cfg(target_os = "windows")]
                audio: crate::audio::AudioCapturer::default(),
                #[cfg(target_os = "windows")]
                capture_audio: rs.capture_audio,
            })),
            running: AtomicBool::new(false),
            polling_spawned: AtomicBool::new(false),
            frames_received: std::sync::atomic::AtomicU64::new(0),
            cached_fps: AtomicU32::new(fps),
        }
    }

    /// Apply new settings at runtime (re-creates buffer, updates config).
    #[allow(dead_code)]
    pub fn reconfigure(&self, settings: &AppSettings) {
        let rs = &settings.recording;
        let fps = effective_fps(rs);
        self.cached_fps.store(fps, Ordering::SeqCst);
        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            // Update output directory
            inner.output_dir = resolve_output_dir(&rs.output_directory);
            // Rebuild buffer with new capacity
            inner.buffer = BufferManager::new(
                BufferConfig {
                    max_duration_secs: rs.buffer_duration_secs,
                    fps,
                    bitrate_kbps: rs.bitrate_kbps,
                },
                inner.resolution.0,
                inner.resolution.1,
            );
            // Rebuild H.264 encoder with new settings.
            // Native resolution defers init until first frame.
            inner.resolution_is_native = is_native_resolution(&rs.resolution);
            inner.native_bitrate_kbps = rs.bitrate_kbps;
            let (target_w, target_h) = if inner.resolution_is_native {
                // (0,0) = native: backend uses the captured source dimensions.
                (0, 0)
            } else {
                resolution_dimensions(&rs.resolution)
            };
            inner.target_width = target_w;
            inner.target_height = target_h;
            #[cfg(target_os = "windows")]
            {
                inner.frame_index = 0;
                inner.sps.clear();
                inner.pps.clear();
                inner.h264_encoder = None;
                inner.h264_encoder_init_failed = false;
            }
            #[cfg(target_os = "macos")]
            {
                inner.frame_index = 0;
                inner.sps.clear();
                inner.pps.clear();
                inner.h264_encoder = None;
            }
            // Update capture config (applied on next backend.start()).
            inner.backend_config.fps = fps;
            inner.backend_config.target_width = target_w;
            inner.backend_config.target_height = target_h;
            #[cfg(target_os = "windows")]
            {
                inner.capture_audio = rs.capture_audio;
            }
        }
    }

    /// Update the capture target (display/window/application) at runtime.
    /// Does not restart the capture — call before starting or stop/start manually.
    pub fn reconfigure_target(&self, target: CaptureTarget) {
        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            inner.backend_config.target = target;
        }
    }

    // ── Recording control ────────────────────────────────────────────────

    /// Start the capture backend and mark as recording.
    /// Safe to call multiple times — no-ops if already recording.
    pub fn start_recording(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut guard = self.inner.lock();
        let inner = guard.as_mut().ok_or("Recorder not initialized")?;

        inner
            .backend
            .start(inner.backend_config.clone())
            .map_err(|e| format!("Failed to start capture: {e}"))?;

        #[cfg(target_os = "windows")]
        if inner.capture_audio {
            inner
                .audio
                .start(inner.buffer.config().max_duration_secs);
        }

        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Stop the capture backend and mark as stopped.
    /// Clears the ring buffer, resets the elapsed timer, and resets the
    /// frame counter so the next recording session starts fresh.
    /// Resets the polling-spawned flag so the next `start_recording` can
    /// re-create the background frame-polling task.
    pub fn stop_recording(&self) -> Result<(), String> {
        self.running.store(false, Ordering::SeqCst);
        self.polling_spawned.store(false, Ordering::SeqCst);
        self.frames_received.store(0, Ordering::SeqCst);

        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            inner
                .backend
                .stop()
                .map_err(|e| format!("Failed to stop capture: {e}"))?;
            inner.buffer.clear();
            inner.recording_started_at = None;
            inner.latest_frame = None;
            #[cfg(target_os = "windows")]
            {
                inner.audio.stop();
                inner.h264_encoder = None;
                inner.frame_index = 0;
                inner.sps.clear();
                inner.pps.clear();
                inner.h264_encoder_init_failed = false;
            }
            #[cfg(target_os = "macos")]
            {
                inner.h264_encoder = None;
                inner.frame_index = 0;
                inner.sps.clear();
                inner.pps.clear();
            }
        }
        Ok(())
    }

    /// Check whether recording is active (atomic, no lock).
    pub fn is_recording(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Spawn the background polling thread if not already spawned.
    /// Returns true if the thread was spawned, false if already running.
    pub fn start_polling(&self, app: AppHandle) -> bool {
        if self.polling_spawned.swap(true, Ordering::SeqCst) {
            return false; // already spawned
        }

        // Dedicated OS thread so H.264 encoding and capture polling never
        // compete with the tokio worker threads that service save_clip /
        // get_preview_frame commands (previously caused lock contention and
        // stutters).
        let app_handle = app;
        std::thread::Builder::new()
            .name("prism-capture".into())
            .spawn(move || {
                loop {
                    let poll_started = std::time::Instant::now();
                    let recorder = app_handle.state::<Recorder>();
                    if !recorder.is_recording() {
                        break;
                    }
                    recorder.poll_and_push();
                    let interval = recorder.poll_interval();
                    // Keep the configured frame period stable instead of adding
                    // capture and encode time to every interval.
                    std::thread::sleep(interval.saturating_sub(poll_started.elapsed()));
                }
            })
            .map_err(|e| eprintln!("[prism] failed to spawn capture thread: {e}"))
            .err()
            .map(|_| {
                self.polling_spawned.store(false, Ordering::SeqCst);
            });

        true
    }

    /// Clear the ring buffer (e.g. on game switch).
    #[allow(dead_code)]
    pub fn clear_buffer(&self) {
        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            inner.buffer.clear();
        }
    }

    // ── Polling (called from background task) ────────────────────────────

    /// Get the buffer duration in seconds.
    pub fn buffer_duration_secs(&self) -> u32 {
        self.inner
            .lock()
            .as_ref()
            .map(|inner| inner.buffer.config().max_duration_secs)
            .unwrap_or(60)
    }

    /// Update the active buffer duration without interrupting capture.
    pub fn set_buffer_duration_secs(&self, duration_secs: u32) {
        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            inner.buffer.set_duration_secs(duration_secs);
        }
    }

    /// Toggle system-audio capture live. When enabled mid-session, starts the
    /// WASAPI capturer with the current buffer duration; when disabled, stops
    /// and clears buffered audio.
    #[cfg(target_os = "windows")]
    pub fn set_capture_audio(&self, enabled: bool) {
        let mut guard = self.inner.lock();
        if let Some(inner) = guard.as_mut() {
            inner.capture_audio = enabled;
            if enabled && self.running.load(Ordering::SeqCst) {
                inner.audio.start(inner.buffer.config().max_duration_secs);
            } else if !enabled {
                inner.audio.stop();
            }
        }
    }

    /// Poll the capture backend for a new frame and push to buffer.
    ///
    /// Uses a 3-phase approach to avoid holding the recorder lock during H.264
    /// encoding, which can block save_clip, get_preview_frame, and other commands:
    ///
    ///   Phase 1 (brief lock): read frame, remove encoder, clone metadata
    ///   Phase 2 (no lock):    H.264 encoding (the expensive part)
    ///   Phase 3 (brief lock): restore encoder, push encoded packets to buffer
    pub fn poll_and_push(&self) -> u32 {
        let prof = prof_enabled();
        let t0 = if prof { Some(std::time::Instant::now()) } else { None };

        // ── Phase 1: Lock, read frame, take encoder, clone state ──────────
        let mut guard = self.inner.lock();
        let inner = match guard.as_mut() {
            Some(i) => i,
            None => return 0,
        };

        let Some(frame) = inner.backend.read_latest_frame() else {
            return 0;
        };

        // Mark recording start on first frame
        if inner.recording_started_at.is_none() {
            inner.recording_started_at = Some(std::time::Instant::now());
        }

        // Update resolution from first frame
        if inner.resolution == (0, 0) || inner.resolution != (frame.width, frame.height) {
            inner.resolution = (frame.width, frame.height);
        }
        // Keep a copy for live preview (Arc clone is cheap)
        inner.latest_frame = Some(frame.clone());

        #[cfg(target_os = "windows")]
        // Phase 1 data for Windows: encoder, metadata, NV12 data
        let phase1 = {
            // Try to init encoder if not available
            if inner.h264_encoder.is_none() && !inner.h264_encoder_init_failed {
                // Desktop Duplication currently captures at source resolution.
                // Feeding an unscaled native frame to a smaller MFT media type
                // is invalid and can force costly encoder fallback behavior.
                let (enc_w, enc_h) = (frame.width, frame.height);
                match MfH264Encoder::new(
                    enc_w,
                    enc_h,
                    inner.backend_config.fps,
                    inner.native_bitrate_kbps,
                    inner.backend_config.fps,
                ) {
                    Ok(enc) => {
                        inner.h264_encoder = Some(enc);
                        inner.frame_index = 0;
                    }
                    Err(e) => {
                        eprintln!(
                            "[prism] H.264 encoder init failed; using raw NV12 fallback: {e}"
                        );
                        inner.h264_encoder_init_failed = true;
                    }
                }
            }

            let encoder = inner.h264_encoder.take();
            let init_failed = inner.h264_encoder_init_failed;
            let sps = inner.sps.clone();
            let pps = inner.pps.clone();
            let idx = inner.frame_index;
            inner.frame_index += 1;
            (frame, encoder, init_failed, sps, pps, idx)
        };

        #[cfg(target_os = "macos")]
        let phase1 = {
            (
                frame,
                inner.target_width,
                inner.target_height,
                inner.resolution_is_native,
                inner.h264_encoder.take(),
                inner.native_bitrate_kbps,
                inner.backend_config.fps,
                inner.sps.clone(),
                inner.pps.clone(),
                inner.frame_index,
            )
        };

        #[cfg(target_os = "linux")]
        let phase1 = (frame,);

        // ── Lock released here (guard drops) ──────────────────────────────
        drop(guard);
        let t1 = if prof { Some(std::time::Instant::now()) } else { None };

        // ── Phase 2: Encoding (NO lock held) ──────────────────────────────
        #[cfg(target_os = "windows")]
        let phase2 = Self::process_windows_frame(phase1);

        #[cfg(target_os = "macos")]
        let phase2 = Self::process_macos_frame(phase1);

        #[cfg(target_os = "linux")]
        let phase2 = Self::process_linux_frame(phase1);
        let t2 = if prof { Some(std::time::Instant::now()) } else { None };

        // ── Phase 3: Lock, restore encoder, push results ──────────────────
        let mut guard = self.inner.lock();
        let inner = match guard.as_mut() {
            Some(i) => i,
            None => return 1, // frame was polled but push may be partial
        };

        #[cfg(target_os = "windows")]
        {
            let (encoder, init_failed, sps, pps, push_items) = phase2;
            inner.h264_encoder = encoder;
            inner.h264_encoder_init_failed = init_failed;
            if !sps.is_empty() {
                inner.sps = sps;
            }
            if !pps.is_empty() {
                inner.pps = pps;
            }
            for item in push_items {
                inner.buffer.push_frame(item);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let (encoder, sps, pps, push_items) = phase2;
            inner.h264_encoder = encoder;
            inner.frame_index += 1;
            if !sps.is_empty() {
                inner.sps = sps;
            }
            if !pps.is_empty() {
                inner.pps = pps;
            }
            for item in push_items {
                inner.buffer.push_frame(item);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let push_items = phase2;
            for item in push_items {
                inner.buffer.push_frame(item);
            }
        }

        self.frames_received.fetch_add(1, Ordering::SeqCst);

        if let (Some(t0), Some(t1), Some(t2)) = (t0, t1, t2) {
            let total = t2.elapsed();
            let lock1 = t1.duration_since(t0);
            let encode = t2.duration_since(t1);
            eprintln!(
                "[prof] lock1={:?} encode={:?} total={:?}",
                lock1, encode, total
            );
        }
        1
    }

    /// Phase 2 (Windows): encode NV12 frame outside the lock.
    #[cfg(target_os = "windows")]
    fn process_windows_frame(
        (frame, encoder_opt, init_failed, sps, pps, _frame_idx): (
            CapturedFrame,
            Option<MfH264Encoder>,
            bool,
            Vec<u8>,
            Vec<u8>,
            u64,
        ),
    ) -> (
        Option<MfH264Encoder>,
        bool,
        Vec<u8>,
        Vec<u8>,
        Vec<StoredFrame>,
    ) {
        let Some(mut encoder) = encoder_opt else {
            // No encoder available (init pending/failed) — store raw NV12 so the
            // shadow buffer keeps accumulating frames. The byte-budgeted ring
            // evicts the oldest raw frames, so this cannot grow unboundedly, and
            // clip-save surfaces a clear "no H.264" error instead of an empty
            // buffer.
            return (
                None,
                init_failed,
                sps,
                pps,
                vec![StoredFrame::from(frame)],
            );
        };

        match encoder.encode_frame(&frame.data) {
            Ok(packets) => {
                let mut new_sps = sps;
                let mut new_pps = pps;
                if new_sps.is_empty() && encoder.sps_pps_ready() {
                    new_sps = encoder.sps().to_vec();
                    new_pps = encoder.pps().to_vec();
                    eprintln!(
                        "[prism] captured SPS({}) PPS({})",
                        new_sps.len(),
                        new_pps.len()
                    );
                }
                let stored: Vec<StoredFrame> = packets
                    .into_iter()
                    .map(|pkt| StoredFrame {
                        data: std::sync::Arc::new(pkt.data),
                        width: frame.width,
                        height: frame.height,
                        stride: 0,
                        pixel_format: crate::capture::PixelFormat::H264,
                        timestamp: frame.timestamp,
                        is_sync: pkt.is_sync,
                    })
                    .collect();
                (Some(encoder), false, new_sps, new_pps, stored)
            }
            Err(e) => {
                eprintln!("H.264 encode error (storing raw NV12 fallback): {e}");
                (
                    Some(encoder),
                    false,
                    sps,
                    pps,
                    vec![StoredFrame::from(frame)],
                )
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[allow(clippy::type_complexity)]
    fn process_macos_frame(
        (
            frame,
            target_width,
            target_height,
            resolution_is_native,
            mut encoder_opt,
            native_bitrate_kbps,
            fps,
            mut sps,
            mut pps,
            frame_idx,
        ): (
            CapturedFrame,
            u32,
            u32,
            bool,
            Option<VtH264Encoder>,
            u32,
            u32,
            Vec<u8>,
            Vec<u8>,
            u64,
        ),
    ) -> (Option<VtH264Encoder>, Vec<u8>, Vec<u8>, Vec<StoredFrame>) {
        if frame_idx == 0 {
            eprintln!(
                "[prism] macOS frame: capture={}x{} target={}x{} native={} fps={}",
                frame.width, frame.height, target_width, target_height, resolution_is_native, fps,
            );
        }
        let tw = target_width.max(1);
        let th = target_height.max(1);
        let (nv12, nv12_width, nv12_height): (Vec<u8>, u32, u32) =
            if frame.pixel_format == crate::capture::PixelFormat::Bgra {
                if frame.width != target_width || frame.height != target_height {
                    match resize_bgra_frame(
                        &frame.data,
                        frame.width,
                        frame.height,
                        tw,
                        th,
                        frame.stride as usize,
                    ) {
                        Ok(resized_bgra) => {
                            let nv12 = crate::capture::bgra_to_nv12(&resized_bgra, tw, th, tw * 4);
                            (nv12, tw, th)
                        }
                        Err(e) => {
                            eprintln!("[prism] BGRA resize failed (using original): {e}");
                            let nv12 = crate::capture::bgra_to_nv12(
                                &frame.data,
                                frame.width,
                                frame.height,
                                frame.stride,
                            );
                            (nv12, frame.width, frame.height)
                        }
                    }
                } else {
                    let nv12 = crate::capture::bgra_to_nv12(
                        &frame.data,
                        frame.width,
                        frame.height,
                        frame.stride,
                    );
                    (nv12, frame.width, frame.height)
                }
            } else {
                (frame.data.to_vec(), frame.width, frame.height)
            };

        // Init encoder if not available
        if encoder_opt.is_none() {
            let (enc_w, enc_h) = if resolution_is_native {
                (nv12_width, nv12_height)
            } else {
                (tw, th)
            };
            match VtH264Encoder::new(enc_w, enc_h, fps, native_bitrate_kbps, fps) {
                Ok(enc) => {
                    encoder_opt = Some(enc);
                }
                Err(e) => eprintln!("[prism] VT encoder init failed: {e}"),
            }
        }

        let Some(ref mut encoder) = encoder_opt else {
            // No encoder — store raw NV12
            return (
                None,
                sps,
                pps,
                vec![StoredFrame::from(CapturedFrame {
                    data: std::sync::Arc::new(nv12),
                    width: nv12_width,
                    height: nv12_height,
                    stride: nv12_width,
                    pixel_format: crate::capture::PixelFormat::Nv12,
                    timestamp: frame.timestamp,
                })],
            );
        };

        match encoder.encode_frame(&nv12, nv12_width, nv12_height) {
            Ok(packets) => {
                if !packets.is_empty() {
                    if sps.is_empty() && encoder.sps_pps_ready() {
                        sps = encoder.sps().to_vec();
                        pps = encoder.pps().to_vec();
                        eprintln!(
                            "[prism] macOS: captured SPS({}) PPS({})",
                            sps.len(),
                            pps.len()
                        );
                    }
                    let stored: Vec<StoredFrame> = packets
                        .into_iter()
                        .map(|pkt| StoredFrame {
                            data: std::sync::Arc::new(pkt.data),
                            width: nv12_width,
                            height: nv12_height,
                            stride: 0,
                            pixel_format: crate::capture::PixelFormat::H264,
                            timestamp: frame.timestamp,
                            is_sync: pkt.is_sync,
                        })
                        .collect();
                    (encoder_opt, sps, pps, stored)
                } else {
                    eprintln!("[prism] VT encoder skipped frame — storing raw NV12");
                    (
                        encoder_opt,
                        sps,
                        pps,
                        vec![StoredFrame::from(CapturedFrame {
                            data: std::sync::Arc::new(nv12),
                            width: nv12_width,
                            height: nv12_height,
                            stride: nv12_width,
                            pixel_format: crate::capture::PixelFormat::Nv12,
                            timestamp: frame.timestamp,
                        })],
                    )
                }
            }
            Err(e) => {
                eprintln!("VT H.264 encode error (falling back to raw NV12): {e}");
                (
                    encoder_opt,
                    sps,
                    pps,
                    vec![StoredFrame::from(CapturedFrame {
                        data: std::sync::Arc::new(nv12),
                        width: nv12_width,
                        height: nv12_height,
                        stride: nv12_width,
                        pixel_format: crate::capture::PixelFormat::Nv12,
                        timestamp: frame.timestamp,
                    })],
                )
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn process_linux_frame((frame,): (CapturedFrame,)) -> Vec<StoredFrame> {
        if frame.pixel_format == crate::capture::PixelFormat::Bgra {
            let nv12 =
                crate::capture::bgra_to_nv12(&frame.data, frame.width, frame.height, frame.stride);
            vec![StoredFrame::from(CapturedFrame {
                data: std::sync::Arc::new(nv12),
                width: frame.width,
                height: frame.height,
                stride: frame.width,
                pixel_format: crate::capture::PixelFormat::Nv12,
                timestamp: frame.timestamp,
            })]
        } else {
            vec![StoredFrame::from(frame)]
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
            .as_ref()
            .map(|inner| inner.latest_frame.is_some())
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
            .as_ref()
            .and_then(|inner| {
                inner
                    .recording_started_at
                    .map(|t| t.elapsed().as_secs_f64())
            })
            .unwrap_or(0.0)
    }

    /// Wall-clock span of buffered frames in seconds (from oldest to newest).
    /// This is the ACTUAL buffer duration, not frame_count / config_fps.
    pub fn buffer_time_secs(&self) -> f64 {
        self.inner
            .lock()
            .as_ref()
            .map(|i| i.buffer.time_span_secs())
            .unwrap_or(0.0)
    }

    /// How many seconds of clip can actually be extracted.
    /// Minimum of configured duration and actual buffer time span.
    pub fn available_clip_secs(&self) -> f64 {
        let configured = self.buffer_duration_secs() as f64;
        let actual = self.buffer_time_secs();
        actual.min(configured)
    }

    /// Get the current buffer frame count (for diagnostics).
    pub fn frame_count(&self) -> usize {
        self.inner
            .lock()
            .as_ref()
            .map(|i| i.buffer.frame_count())
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
        // JPEG conversion is expensive; retain only the Arc-backed frame while
        // locked so capture can continue while the preview is generated.
        let frame = self.inner.lock().as_ref()?.latest_frame.clone()?;

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
                let uv_width = stride.div_ceil(2);

                for dy in 0..preview_h {
                    for dx in 0..preview_w {
                        let sx = (dx * width) / preview_w;
                        let sy = (dy * height) / preview_h;

                        let y_off = (sy * stride + sx) as usize;
                        let uv_off = ((sy / 2) * uv_width + (sx / 2)) as usize * 2;

                        let y_val = y_plane[y_off] as i32 - 16;
                        let u_val = uv_plane[uv_off] as i32 - 128;
                        let v_val = uv_plane[uv_off + 1] as i32 - 128;

                        let r = ((298 * y_val + 409 * v_val + 128) >> 8).clamp(0, 255) as u8;
                        let g = ((298 * y_val - 100 * u_val - 208 * v_val + 128) >> 8).clamp(0, 255)
                            as u8;
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
                        pixel[2] = data[offset]; // B ← R
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
            .encode(&rgb, preview_w, preview_h, image::ExtendedColorType::Rgb8)
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
    /// Most recently captured frame (NV12 on macOS) for server-side thumbnail generation.
    pub preview_frame: Option<CapturedFrame>,
}

impl Recorder {
    /// Extract clip frames and metadata from the ring buffer.
    ///
    /// This is the ONLY operation that needs the recorder lock.
    /// Encoding should happen AFTER releasing the lock.
    pub fn extract_clip_data(&self, duration_secs: u32) -> Result<ClipData, String> {
        let guard = self.inner.lock();
        let inner = guard.as_ref().ok_or("Recorder not initialized")?;
        let frames = if duration_secs > 0 {
            inner.buffer.clip(Duration::from_secs(duration_secs as u64))
        } else {
            inner.buffer.clip_all()
        };
        if frames.is_empty() {
            return Err("No frames available to clip".into());
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let (sps, pps) = {
            // If the cached SPS/PPS from the encoder are empty, try to find them
            // by scanning all buffered H.264 frames (covers the case where the
            // encoder hasn't output a keyframe within the clip window yet).
            let cached_sps_pps = if inner.sps.is_empty() || inner.pps.is_empty() {
                inner.buffer.find_sps_pps_anywhere().map(|(s, p)| {
                    eprintln!(
                        "[prism] found SPS({}) PPS({}) from buffer scan",
                        s.len(),
                        p.len()
                    );
                    (s, p)
                })
            } else {
                Some((inner.sps.clone(), inner.pps.clone()))
            };
            cached_sps_pps.unwrap_or_default()
        };

        Ok(ClipData {
            frames,
            output_dir: inner.output_dir.clone(),
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            sps,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            pps,
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            sps: Vec::new(),
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            pps: Vec::new(),
            preview_frame: inner.latest_frame.clone(),
        })
    }

    /// Extract the system-audio PCM window `[start, end)` matching the saved
    /// video frames. Grabs the audio ring handle under the recorder lock
    /// (briefly) and copies the PCM outside it.
    #[cfg(target_os = "windows")]
    pub fn extract_clip_audio(&self, start: std::time::Instant, end: std::time::Instant) -> Option<Vec<u8>> {
        let ring = self
            .inner
            .lock()
            .as_ref()
            .and_then(|inner| inner.capture_audio.then(|| inner.audio.ring_handle()))?;
        let audio = ring.lock().extract(start, end);
        audio
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Resolve the output directory: use user-configured path or default to Videos/Prism.
pub fn resolve_output_dir(configured: &str) -> PathBuf {
    if !configured.is_empty() {
        return PathBuf::from(configured);
    }
    // Default: ~/Videos/Prism (or platform equivalent)
    dirs::video_dir()
        .map(|d| d.join("Prism"))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Best-effort read MP4 duration from the file header using the mp4 crate.
pub fn read_mp4_duration(path: &std::path::Path) -> Option<u32> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).ok()?;
    let size = file.metadata().ok()?.len();
    let reader = BufReader::new(file);
    let mp4 = mp4::Mp4Reader::read_header(reader, size).ok()?;
    Some(mp4.duration().as_secs() as u32)
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
