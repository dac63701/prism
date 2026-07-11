//! Configuration structs with serde serialization.
//! Default settings are production-sensible for a game clipping app.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppSettings {
    pub recording: RecordingSettings,
    pub hotkeys: HotkeySettings,
    pub general: GeneralSettings,
    pub storage: StorageSettings,
    pub cloud: CloudSettings,
}

// ── Recording ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSettings {
    /// Ring buffer duration in seconds (10–1800)
    pub buffer_duration_secs: u32,
    /// Capture FPS (24, 30, 60)
    pub fps: u32,
    /// Target output bitrate in kilobits per second.
    #[serde(default = "default_bitrate_kbps")]
    pub bitrate_kbps: u32,
    /// Target output resolution: "native" | "720p" | "1080p" | "1440p" | "2160p".
    /// "native" preserves the capture source's original dimensions.
    #[serde(default = "default_resolution_string")]
    pub resolution: String,
    /// Output directory — if empty, use default OS Videos/Prism
    pub output_directory: String,
    /// Serialized capture target ("display", "display_id:N", "window:N", "application:bundle_id")
    pub capture_target: String,
    /// Start recording buffer automatically on app launch
    pub always_on_recording: bool,
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            buffer_duration_secs: 30,
            fps: 60,
            bitrate_kbps: default_bitrate_kbps(),
            resolution: default_resolution().into(),
            output_directory: String::new(),
            capture_target: String::new(),
            always_on_recording: true,
        }
    }
}

// ── Hotkeys ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    /// "Ctrl+Shift+X" on Windows/Linux, "Cmd+Shift+X" on macOS
    pub save_clip: String,
    pub toggle_recording: String,
    pub open_library: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        const MOD: &str = "Cmd";
        #[cfg(not(target_os = "macos"))]
        const MOD: &str = "Ctrl";

        Self {
            save_clip: format!("{MOD}+Shift+X"),
            toggle_recording: format!("{MOD}+Shift+R"),
            open_library: format!("{MOD}+Shift+L"),
        }
    }
}

// ── General ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    pub launch_at_startup: bool,
    pub minimize_to_tray: bool,
    pub show_clip_notification: bool,
    pub game_detection_enabled: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            show_clip_notification: true,
            game_detection_enabled: false,
        }
    }
}

// ── Storage ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    /// Max disk usage in GB before auto-cleanup (0 = unlimited)
    pub max_clips_gb: u32,
    /// Auto-delete clips older than N days (None = disabled)
    pub auto_prune_days: Option<u32>,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            max_clips_gb: 50,
            auto_prune_days: None,
        }
    }
}

// ── Cloud ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSettings {
    /// Self-hosted Prism server URL (e.g. "https://clips.example.com")
    pub server_url: String,
    /// API key for authenticating upload requests
    pub api_key: String,
    /// Auto-upload clips immediately after saving
    pub auto_upload: bool,
    /// Max concurrent uploads (0 = sequential)
    pub max_concurrent_uploads: u32,
    /// Display name of the connected account (if any)
    pub account_display_name: String,
    /// Email of the connected account (if any)
    pub account_email: String,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            server_url: String::from("https://goprism.studio"),
            api_key: String::new(),
            auto_upload: false,
            max_concurrent_uploads: 1,
            account_display_name: String::new(),
            account_email: String::new(),
        }
    }
}

/// Default output resolution for new installs and resets.
pub fn default_resolution() -> &'static str {
    "1080p"
}

pub fn default_resolution_string() -> String {
    default_resolution().to_string()
}

/// Default output bitrate for new installs and resets.
pub fn default_bitrate_kbps() -> u32 {
    8_000
}

/// Map a user-facing resolution label to dimensions.
/// Returns `(0, 0)` for "native" — callers should use capture-source dimensions.
pub fn resolution_dimensions(label: &str) -> (u32, u32) {
    match label.to_ascii_lowercase().as_str() {
        "native" => (0, 0),
        "720p" => (1280, 720),
        "1440p" => (2560, 1440),
        "2160p" | "4k" => (3840, 2160),
        _ => (1920, 1080),
    }
}

/// Returns `true` when the resolution label is set to native capture.
pub fn is_native_resolution(label: &str) -> bool {
    label.eq_ignore_ascii_case("native")
}
