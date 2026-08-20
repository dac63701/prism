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
    #[serde(default)]
    pub auto_clip: AutoClipSettings,
}

// ── Recording ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSettings {
    /// Ring buffer duration in seconds (10–1800)
    pub buffer_duration_secs: u32,
    /// Capture FPS (24, 30, 60). Ignored when `fps_auto` is true.
    pub fps: u32,
    /// Match capture FPS to the display's refresh rate automatically.
    #[serde(default)]
    pub fps_auto: bool,
    /// Target output bitrate in kilobits per second.
    #[serde(default = "default_bitrate_kbps")]
    pub bitrate_kbps: u32,
    /// Target output resolution: "native" | "720p" | "1080p" | "1440p" | "2160p".
    /// "native" preserves the capture source's original dimensions.
    #[serde(default = "default_resolution_string")]
    pub resolution: String,
    /// Output aspect ratio: "match" (preserve the source — never stretches),
    /// or a forced "16:9" | "21:9" | "32:9". Ignored when `resolution` is native.
    #[serde(default = "default_aspect_ratio_string")]
    pub aspect_ratio: String,
    /// Output directory — if empty, use default OS Videos/Prism
    pub output_directory: String,
    /// Serialized capture target ("display", "display_id:N", "window:N", "application:bundle_id")
    pub capture_target: String,
    /// Start recording buffer automatically on app launch
    pub always_on_recording: bool,
    /// Capture system audio (WASAPI loopback) alongside video.
    #[serde(default = "default_true")]
    pub capture_audio: bool,
    /// Quality preset key ("fast" | "balanced" | "high"). "custom" when the
    /// individual resolution/fps/bitrate fields have been hand-tuned.
    #[serde(default = "default_quality_preset")]
    pub quality_preset: String,
}

fn default_quality_preset() -> String {
    "balanced".into()
}

/// Default output aspect ratio: preserve the capture source so ultrawide
/// displays record without distortion.
pub fn default_aspect_ratio() -> &'static str {
    "match"
}

pub fn default_aspect_ratio_string() -> String {
    default_aspect_ratio().to_string()
}

fn default_true() -> bool {
    true
}

impl Default for RecordingSettings {
    fn default() -> Self {
        Self {
            buffer_duration_secs: 30,
            fps: 60,
            fps_auto: true,
            bitrate_kbps: default_bitrate_kbps(),
            resolution: default_resolution().into(),
            aspect_ratio: default_aspect_ratio_string(),
            output_directory: String::new(),
            capture_target: String::new(),
            always_on_recording: false,
            capture_audio: true,
            quality_preset: default_quality_preset(),
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
    /// Localhost port used by Counter-Strike 2 Game State Integration.
    #[serde(default = "default_cs2_gsi_port")]
    pub cs2_gsi_port: u16,
    /// Unix timestamp (seconds) when the user last dismissed the first-boot
    /// sign-in prompt. `None` = never dismissed (show on next launch).
    /// Re-prompts after a cooldown if the user still isn't signed in.
    #[serde(default)]
    pub sign_in_prompt_dismissed_at: Option<u64>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            launch_at_startup: false,
            minimize_to_tray: true,
            show_clip_notification: true,
            game_detection_enabled: false,
            cs2_gsi_port: default_cs2_gsi_port(),
            sign_in_prompt_dismissed_at: None,
        }
    }
}

// ── Auto clipping ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoClipSettings {
    pub enabled: bool,
    /// Minimum time between automatic clips for the same game.
    pub cooldown_secs: u32,
    /// 0.0 is least sensitive and 1.0 is most sensitive.
    pub audio_sensitivity: f32,
    pub games: Vec<PerGameAutoClip>,
}

impl Default for AutoClipSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            cooldown_secs: 30,
            audio_sensitivity: 0.5,
            games: vec![
                PerGameAutoClip {
                    game_name: "Counter-Strike 2".into(),
                    enabled: true,
                    kill_clip_duration: 20,
                    death_clip_duration: 30,
                    combat_event_duration: 20,
                    events: vec![
                        "kill".into(),
                        "death".into(),
                        "headshot".into(),
                        "win".into(),
                    ],
                    audio_enabled: false,
                    audio_sensitivity: None,
                },
                PerGameAutoClip {
                    game_name: "Rust".into(),
                    enabled: true,
                    kill_clip_duration: 20,
                    death_clip_duration: 30,
                    combat_event_duration: 20,
                    events: vec!["headshot".into(), "explosion".into(), "combat".into()],
                    audio_enabled: true,
                    audio_sensitivity: None,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerGameAutoClip {
    pub game_name: String,
    pub enabled: bool,
    pub kill_clip_duration: u32,
    pub death_clip_duration: u32,
    pub combat_event_duration: u32,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub audio_enabled: bool,
    #[serde(default)]
    pub audio_sensitivity: Option<f32>,
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
    /// API key for authenticating upload requests (deprecated — use access_token)
    pub api_key: String,
    /// JWT access token from desktop exchange (used for upload auth)
    pub access_token: String,
    /// JWT refresh token for obtaining new access tokens
    pub refresh_token: String,
    /// Auto-upload clips immediately after saving
    pub auto_upload: bool,
    /// Max concurrent uploads (0 = sequential)
    pub max_concurrent_uploads: u32,
    /// Display name of the connected account (if any)
    pub account_display_name: String,
    /// Email of the connected account (if any)
    pub account_email: String,
    /// Avatar URL of the connected account (if any)
    #[serde(default)]
    pub avatar_url: String,
    /// Default visibility for uploaded clips: "public" | "unlisted" | "private"
    #[serde(default = "default_visibility")]
    pub default_visibility: String,
    /// Auto-copy the share link to the clipboard when an upload finishes.
    #[serde(default)]
    pub copy_share_link_after_upload: bool,
    /// How many times a failed upload is retried before giving up.
    #[serde(default = "default_max_upload_retries")]
    pub max_upload_retries: u32,
}

fn default_visibility() -> String {
    "unlisted".into()
}

fn default_max_upload_retries() -> u32 {
    2
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            server_url: String::from("https://goprism.studio"),
            api_key: String::new(),
            access_token: String::new(),
            refresh_token: String::new(),
            auto_upload: false,
            max_concurrent_uploads: 1,
            account_display_name: String::new(),
            account_email: String::new(),
            avatar_url: String::new(),
            default_visibility: default_visibility(),
            copy_share_link_after_upload: false,
            max_upload_retries: default_max_upload_retries(),
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

pub fn default_cs2_gsi_port() -> u16 {
    4_000
}

/// Map a user-facing resolution label to dimensions.
/// Returns `(0, 0)` for "native" — callers should use capture-source dimensions.
pub fn resolution_dimensions(label: &str) -> (u32, u32) {
    if label.eq_ignore_ascii_case("native") {
        (0, 0)
    } else if label.eq_ignore_ascii_case("720p") {
        (1280, 720)
    } else if label.eq_ignore_ascii_case("1440p") {
        (2560, 1440)
    } else if label.eq_ignore_ascii_case("2160p") || label.eq_ignore_ascii_case("4k") {
        (3840, 2160)
    } else {
        (1920, 1080)
    }
}

/// Returns `true` when the resolution label is set to native capture.
pub fn is_native_resolution(label: &str) -> bool {
    label.eq_ignore_ascii_case("native")
}

/// Compute the output dimensions for a capture source of `src_w × src_h`
/// scaled to a preset vertical resolution `target_h`, honoring the aspect mode.
///
/// - `"match"` preserves the source aspect ratio (no distortion) — an
///   ultrawide 3440×1440 source at 1080p becomes 2580×1080, not a stretched
///   1920×1080.
/// - `"16:9" | "21:9" | "32:9"` force that ratio (the previous behavior for
///   a 16:9 source; deliberately stretches when the source differs).
///
/// Returns even dimensions for H.264/NV12 alignment. When `src_w`/`src_h` or
/// `target_h` are zero (native capture), returns the source dimensions.
pub fn aspect_dimensions(src_w: u32, src_h: u32, target_h: u32, aspect: &str) -> (u32, u32) {
    if src_w == 0 || src_h == 0 || target_h == 0 {
        return (src_w, src_h);
    }
    let h = target_h & !1;
    if h == 0 {
        return (src_w, src_h);
    }
    let w = match aspect.to_ascii_lowercase().as_str() {
        "16:9" => h * 16 / 9,
        "21:9" => h * 21 / 9,
        "32:9" => h * 32 / 9,
        _ => {
            // Preserve the source aspect ratio, scaled to the target height.
            let w = (src_w as u64 * h as u64 + src_h as u64 / 2) / src_h as u64;
            w.max(2) as u32
        }
    };
    (w.max(2) & !1, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_match_preserves_ultrawide_ratio() {
        // 3440×1440 (21:9) at 1080p stays ultrawide instead of stretching.
        assert_eq!(aspect_dimensions(3440, 1440, 1080, "match"), (2580, 1080));
        // 5120×1440 (32:9) at 1080p keeps its ratio.
        assert_eq!(aspect_dimensions(5120, 1440, 1080, "match"), (3840, 1080));
        // 16:9 source is unchanged.
        assert_eq!(aspect_dimensions(1920, 1080, 1080, "match"), (1920, 1080));
    }

    #[test]
    fn aspect_forced_ratios() {
        assert_eq!(aspect_dimensions(3440, 1440, 1080, "16:9"), (1920, 1080));
        assert_eq!(aspect_dimensions(1920, 1080, 1080, "21:9"), (2520, 1080));
        assert_eq!(aspect_dimensions(1920, 1080, 1440, "32:9"), (5120, 1440));
        assert_eq!(aspect_dimensions(3440, 1440, 1440, "match"), (3440, 1440));
    }

    #[test]
    fn aspect_dimensions_edge_cases() {
        // Native / zero inputs fall back to the source dimensions.
        assert_eq!(aspect_dimensions(1920, 1080, 0, "match"), (1920, 1080));
        assert_eq!(aspect_dimensions(0, 0, 1080, "match"), (0, 0));
        // Output stays even-aligned.
        let (w, h) = aspect_dimensions(1000, 700, 501, "match");
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
    }
}
