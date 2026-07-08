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
    Bgra,
    /// 8-bit YUV 4:2:0 planar (NV12) — preferred by hardware encoders
    Nv12,
}

/// What to capture — a display, window, or application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureTarget {
    /// Capture the main display
    #[serde(rename = "display")]
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

impl Default for CaptureTarget {
    fn default() -> Self {
        Self::Display
    }
}

/// Configuration passed to a capture backend on start.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target frames per second (e.g. 30, 60)
    pub fps: u32,
    /// Whether to include the cursor in captured frames
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
    fn is_active(&self) -> bool;
}

/// Enumerate available capture sources (displays, applications) for the source selector UI.
pub fn enumerate_capture_sources() -> CaptureSources {
    #[cfg(target_os = "macos")]
    {
        macos::enumerate_sources()
    }
    #[cfg(not(target_os = "macos"))]
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
    pub fn store(&self, frame: CapturedFrame) {
        if let Ok(mut guard) = self.inner.lock() {
            *guard = Some(frame);
        }
    }

    /// Take the latest frame, leaving `None` in its place.
    pub fn take(&self) -> Option<CapturedFrame> {
        self.inner.lock().ok().and_then(|mut g| g.take())
    }
}

// ---------------------------------------------------------------------------
// Fallback backend for unsupported platforms
// ---------------------------------------------------------------------------

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
