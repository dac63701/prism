//! Persistent settings storage (JSON file).
//! Reads/writes AppSettings to a JSON file in the app data directory.

use std::path::PathBuf;

use super::config::AppSettings;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// Create a new store at the given directory path.
    /// The directory is created if it doesn't exist.
    pub fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join(SETTINGS_FILE);
        Self { path }
    }

    /// Load settings from disk, returning defaults if the file doesn't exist.
    pub fn load(&self) -> Result<AppSettings, SettingsError> {
        if !self.path.exists() {
            let defaults = AppSettings::default();
            // Persist defaults so the file exists on first launch
            self.save(&defaults)?;
            return Ok(defaults);
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| SettingsError::Io(format!("Failed to read settings: {e}")))?;

        let settings: AppSettings = serde_json::from_str(&content)
            .map_err(|e| SettingsError::Parse(format!("Failed to parse settings: {e}")))?;

        Ok(settings)
    }

    /// Save settings to disk atomically (write to temp, then rename).
    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SettingsError::Io(format!("Failed to create settings dir: {e}")))?;
        }

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| SettingsError::Serialize(e.to_string()))?;

        // Atomic write: write to .tmp, then rename
        // On Windows, rename fails if the target already exists, so remove it first.
        let tmp_path = self.path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)
            .map_err(|e| SettingsError::Io(format!("Failed to write settings: {e}")))?;
        let _ = std::fs::remove_file(&self.path);
        std::fs::rename(&tmp_path, &self.path)
            .map_err(|e| SettingsError::Io(format!("Failed to commit settings: {e}")))?;

        Ok(())
    }

    /// Return the file path (useful for diagnostics).
    #[allow(dead_code)]
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Serialize error: {0}")]
    Serialize(String),
}
