//! Rust combat detection from its process-scoped Windows audio output.

mod analyzer;
mod audio_capture;
mod templates;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use tauri::{AppHandle, Manager};

use crate::games::DetectedGame;

/// Owns the currently requested Rust audio capture. A generation counter lets
/// a capture thread stop itself immediately when the game closes or changes.
pub struct RustAudioEngine {
    active_pid: AtomicU32,
    generation: AtomicU64,
    capture_alive: AtomicBool,
}

impl RustAudioEngine {
    pub fn new() -> Self {
        Self {
            active_pid: AtomicU32::new(0),
            generation: AtomicU64::new(0),
            capture_alive: AtomicBool::new(false),
        }
    }

    pub fn update_active_game(&self, app: &AppHandle, active: Option<&DetectedGame>) {
        let settings = app.state::<crate::settings::SettingsManager>().get();
        let enabled = settings.auto_clip.enabled
            && settings
                .auto_clip
                .games
                .iter()
                .find(|game| game.game_name == "Rust")
                .is_some_and(|game| game.enabled && game.audio_enabled);
        let pid = if enabled && active.map(|game| game.name.as_str()) == Some("Rust") {
            active.map(|game| game.pid).unwrap_or_default()
        } else {
            0
        };

        let previous_pid = self.active_pid.swap(pid, Ordering::SeqCst);
        if pid == 0 {
            if previous_pid != 0 {
                self.generation.fetch_add(1, Ordering::SeqCst);
                self.capture_alive.store(false, Ordering::SeqCst);
            }
            return;
        }

        if pid == previous_pid && self.capture_alive.load(Ordering::SeqCst) {
            return;
        }

        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.capture_alive.store(true, Ordering::SeqCst);
        let sensitivity = settings
            .auto_clip
            .games
            .iter()
            .find(|game| game.game_name == "Rust")
            .and_then(|game| game.audio_sensitivity)
            .unwrap_or(settings.auto_clip.audio_sensitivity)
            .clamp(0.0, 1.0);
        audio_capture::spawn_capture(app.clone(), pid, generation, sensitivity);
    }

    pub(crate) fn is_current(&self, pid: u32, generation: u64) -> bool {
        self.active_pid.load(Ordering::SeqCst) == pid
            && self.generation.load(Ordering::SeqCst) == generation
    }

    pub(crate) fn mark_capture_stopped(&self, generation: u64) {
        if self.generation.load(Ordering::SeqCst) == generation {
            self.capture_alive.store(false, Ordering::SeqCst);
        }
    }
}
