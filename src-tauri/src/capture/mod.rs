//! Screen capture — platform-specific backends, common trait, and factory.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

/// Shared frame type flowing from capture backend → ring buffer → encoder.
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
    pub timestamp: std::time::Instant,
}

/// Supported pixel formats for capture → encoder pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 32-bit BGRA (macOS default, Windows DXGI default)
    #[allow(dead_code)]
    Bgra,
    /// 8-bit YUV 4:2:0 planar (NV12) — preferred by hardware encoders
    Nv12,
    /// Compressed H.264 NAL unit in AVCC format (4-byte length prefix).
    /// Used by the Windows shadow buffer — data holds encoded bitstream.
    H264,
}

/// What to capture — a display, window, or application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum CaptureTarget {
    /// Capture the main display
    #[serde(rename = "display")]
    #[default]
    Display,
    /// Capture a specific display by its SC display ID
    #[serde(rename = "display_id")]
    DisplayId(u32),
    /// Capture a specific window by its window ID
    #[serde(rename = "window")]
    Window(u32),
    /// Capture a specific application by bundle identifier
    #[serde(rename = "application")]
    Application(String),
}

/// Configuration passed to a capture backend on start.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target frames per second (e.g. 30, 60)
    pub fps: u32,
    /// Whether to include the cursor in captured frames
    #[allow(dead_code)]
    pub capture_cursor: bool,
    /// What display / window / application to capture
    pub target: CaptureTarget,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            fps: 60,
            capture_cursor: true,
            target: CaptureTarget::default(),
        }
    }
}

/// Serialization-friendly info about a display for the source selector UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayInfo {
    pub display_id: u32,
    pub width: u32,
    pub height: u32,
    pub is_main: bool,
}

/// Serialization-friendly info about an application for the source selector UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub pid: i32,
    pub name: String,
    pub bundle_id: String,
    pub window_count: u32,
}

/// Returned by the `get_capture_sources` IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSources {
    pub displays: Vec<DisplayInfo>,
    pub applications: Vec<AppInfo>,
}

/// Errors originating from the capture subsystem.
#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("Capture backend not available on this platform")]
    UnsupportedPlatform,

    #[error("Failed to start capture stream: {0}")]
    StartFailed(String),

    #[error("Capture stream error: {0}")]
    StreamError(String),

    #[error("No frame available")]
    NoFrame,
}

/// Platform-agnostic interface for continuous screen capture.
///
/// Implementations deliver frames in real time (via callback or polling)
/// and expose the latest frame through [`read_latest_frame`].
pub trait CaptureBackend: Send {
    /// Start capturing with the given configuration.
    fn start(&mut self, config: CaptureConfig) -> Result<(), CaptureError>;

    /// Stop capturing and release resources.
    fn stop(&mut self) -> Result<(), CaptureError>;

    /// Return the most recently captured frame, if any.
    ///
    /// Implementations should return the frame data and clear their internal
    /// "latest" slot so the caller can distinguish "new frame" from "stale".
    fn read_latest_frame(&mut self) -> Option<CapturedFrame>;

    /// Whether the backend is currently capturing.
    #[allow(dead_code)]
    fn is_active(&self) -> bool;
}

/// Enumerate available capture sources (displays, applications) for the source selector UI.
pub fn enumerate_capture_sources() -> CaptureSources {
    #[cfg(target_os = "macos")]
    {
        macos::enumerate_sources()
    }
    #[cfg(target_os = "windows")]
    {
        windows::enumerate_sources()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        CaptureSources {
            displays: vec![],
            applications: vec![],
        }
    }
}

/// Create the platform-appropriate capture backend.
pub fn create_capture_backend() -> Box<dyn CaptureBackend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacCaptureBackend::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsCaptureBackend::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxCaptureBackend::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(UnsupportedBackend)
    }
}

// ---------------------------------------------------------------------------
// Shared utility: ring-buffer-friendly "latest frame" holder
// ---------------------------------------------------------------------------

/// A thread-safe slot that holds the single most recent frame.
///
/// Used by platform backends to hand frames from the capture callback thread
/// to the polling consumer (ring buffer).
#[derive(Debug)]
pub struct LatestFrame {
    inner: Mutex<Option<CapturedFrame>>,
}

impl LatestFrame {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Store a new frame (replaces previous).
    #[allow(dead_code)]
    pub fn store(&self, frame: CapturedFrame) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(frame);
        }
    }

    /// Take the latest frame, leaving `None` in its place.
    #[allow(dead_code)]
    pub fn take(&self) -> Option<CapturedFrame> {
        self.inner.lock().ok().and_then(|mut g| g.take())
    }
}

// ---------------------------------------------------------------------------
// BGRA → NV12 conversion (chroma subsampling, 4 B/px → 1.5 B/px)
// ---------------------------------------------------------------------------

/// Convert a BGRA frame to NV12 format in-place into a new buffer.
///
/// NV12 layout: [Y plane: width×height bytes] [UV plane: (width/2)×(height/2)×2 bytes]
/// Total: width × height × 3/2 bytes
///
/// Uses integer-only BT.601 coefficients (no floating point).
pub fn bgra_to_nv12(bgra: &[u8], width: u32, height: u32, bgra_stride: u32) -> Vec<u8> {
    let y_size = (width * height) as usize;
    let uv_width = width.div_ceil(2);
    let uv_height = height.div_ceil(2);
    let uv_size = (uv_width * uv_height * 2) as usize;
    let mut nv12 = vec![0u8; y_size + uv_size];
    let (y_plane, uv_plane) = nv12.split_at_mut(y_size);

    for y in 0..height {
        let row_bgra = (y * bgra_stride) as usize;
        let y_row = (y * width) as usize;
        let uv_row = ((y / 2) * uv_width) as usize * 2;
        let y_even = y % 2 == 0;
        for x in 0..width {
            let bgra_off = row_bgra + (x * 4) as usize;
            let b = bgra[bgra_off] as i32;
            let g = bgra[bgra_off + 1] as i32;
            let r = bgra[bgra_off + 2] as i32;

            let y_val = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[y_row + x as usize] = y_val.clamp(0, 255) as u8;

            if y_even && x % 2 == 0 {
                let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                let uv_off = uv_row + (x / 2) as usize * 2;
                uv_plane[uv_off] = u.clamp(0, 255) as u8;
                uv_plane[uv_off + 1] = v.clamp(0, 255) as u8;
            }
        }
    }

    nv12
}

enum Nv12Format { Rgb, #[allow(dead_code)] Bgra }

fn nv12_convert(nv12: &[u8], width: u32, height: u32, fmt: Nv12Format) -> Vec<u8> {
    let y_size = (width * height) as usize;
    let y_plane = &nv12[..y_size];
    let uv_plane = &nv12[y_size..];
    let uv_width = width.div_ceil(2);
    let bpp: usize = match fmt { Nv12Format::Rgb => 3, Nv12Format::Bgra => 4 };
    let mut out = vec![0u8; (width * height) as usize * bpp];

    for y in 0..height {
        let y_row = (y * width) as usize;
        let uv_row = ((y / 2) * uv_width) as usize * 2;
        let out_row = y_row * bpp;
        for x in 0..width {
            let y_off = y_row + x as usize;
            let uv_off = uv_row + (x / 2) as usize * 2;

            let y_val = y_plane[y_off] as i32 - 16;
            let u_val = uv_plane[uv_off] as i32 - 128;
            let v_val = uv_plane[uv_off + 1] as i32 - 128;

            let r = ((298 * y_val + 409 * v_val + 128) >> 8).clamp(0, 255) as u8;
            let g = ((298 * y_val - 100 * u_val - 208 * v_val + 128) >> 8).clamp(0, 255) as u8;
            let b = ((298 * y_val + 516 * u_val + 128) >> 8).clamp(0, 255) as u8;

            let off = out_row + x as usize * bpp;
            match fmt {
                Nv12Format::Rgb => {
                    out[off] = r;
                    out[off + 1] = g;
                    out[off + 2] = b;
                }
                Nv12Format::Bgra => {
                    out[off] = b;
                    out[off + 1] = g;
                    out[off + 2] = r;
                    out[off + 3] = 255;
                }
            }
        }
    }

    out
}

/// Convert an NV12 frame to RGB (for preview / JPEG encoding).
/// Output is tightly packed R8G8B8 (3 bytes per pixel).
pub fn nv12_to_rgb(nv12: &[u8], width: u32, height: u32) -> Vec<u8> {
    nv12_convert(nv12, width, height, Nv12Format::Rgb)
}

/// Convert an NV12 frame to BGRA (4 bytes per pixel, for VideoToolbox IOSurface).
/// Output is tightly packed BGRA with stride = width * 4.
#[allow(dead_code)]
pub fn nv12_to_bgra(nv12: &[u8], width: u32, height: u32) -> Vec<u8> {
    nv12_convert(nv12, width, height, Nv12Format::Bgra)
}

// ---------------------------------------------------------------------------
// Fallback backend for unsupported platforms
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct UnsupportedBackend;

impl CaptureBackend for UnsupportedBackend {
    fn start(&mut self, _config: CaptureConfig) -> Result<(), CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        Err(CaptureError::UnsupportedPlatform)
    }

    fn read_latest_frame(&mut self) -> Option<CapturedFrame> {
        None
    }

    fn is_active(&self) -> bool {
        false
    }
}
