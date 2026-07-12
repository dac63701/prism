//! Game database — known titles, process names, and metadata.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// A known game entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntry {
    /// Display name
    pub name: String,
    /// Process name(s) to match against (e.g. "RustClient.exe", "Rust")
    pub process_names: Vec<String>,
    /// Window title substrings to match (e.g. "Rust")
    pub window_titles: Vec<String>,
    /// Whether auto-clipping is enabled for this game
    pub auto_clip_enabled: bool,
}

impl GameEntry {
    pub fn known_games() -> Vec<Self> {
        vec![
            Self {
                name: "Rust".into(),
                process_names: vec!["RustClient.exe".into(), "Rust".into(), "rust".into()],
                window_titles: vec!["Rust".into()],
                auto_clip_enabled: false,
            },
            Self {
                name: "Counter-Strike 2".into(),
                process_names: vec!["cs2.exe".into(), "cs2".into()],
                window_titles: vec!["Counter-Strike 2".into(), "CS2".into()],
                auto_clip_enabled: false,
            },
            Self {
                name: "Valorant".into(),
                process_names: vec!["VALORANT.exe".into(), "VALORANT".into()],
                window_titles: vec!["VALORANT".into()],
                auto_clip_enabled: false,
            },
            Self {
                name: "Apex Legends".into(),
                process_names: vec!["r5apex.exe".into(), "r5apex".into()],
                window_titles: vec!["Apex Legends".into()],
                auto_clip_enabled: false,
            },
        ]
    }
}

/// Thread-safe game registry.
pub struct GameRegistry {
    inner: Mutex<Vec<GameEntry>>,
}

impl GameRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(GameEntry::known_games()),
        }
    }

    /// Find a game by its process name (case-insensitive).
    #[allow(dead_code)]
    pub fn detect_by_process(&self, process_name: &str) -> Option<GameEntry> {
        let lower = process_name.to_lowercase();
        self.inner
            .lock()
            .ok()?
            .iter()
            .find(|g| g.process_names.iter().any(|p| p.to_lowercase() == lower))
            .cloned()
    }

    /// Find a game by window title substring (case-insensitive).
    pub fn detect_by_window_title(&self, title: &str) -> Option<GameEntry> {
        let lower = title.to_lowercase();
        self.inner
            .lock()
            .ok()?
            .iter()
            .find(|g| {
                g.window_titles
                    .iter()
                    .any(|w| lower.contains(&w.to_lowercase()))
            })
            .cloned()
    }

    /// Toggle auto-clip for a game.
    #[allow(dead_code)]
    pub fn set_auto_clip(&self, name: &str, enabled: bool) {
        if let Ok(mut games) = self.inner.lock() {
            if let Some(game) = games.iter_mut().find(|g| g.name == name) {
                game.auto_clip_enabled = enabled;
            }
        }
    }

    #[allow(dead_code)]
    pub fn all(&self) -> Vec<GameEntry> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
