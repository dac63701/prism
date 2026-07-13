//! Application settings — persistent configuration store.
//!
//! Provides [`SettingsManager`] as Tauri managed state so commands
//! and other modules can read/write settings safely.

pub mod config;
pub mod store;

use std::sync::RwLock;

use config::AppSettings;
use store::{SettingsError, SettingsStore};
use tauri::{AppHandle, Emitter, Manager};

/// Managed Tauri state for application settings.
/// Thread-safe: reads and writes go through RwLock.
pub struct SettingsManager {
    store: SettingsStore,
    inner: RwLock<AppSettings>,
}

impl SettingsManager {
    /// Create with an existing store (no disk loading).
    pub fn with_store(store: SettingsStore) -> Self {
        let settings = store.load().unwrap_or_default();
        Self {
            store,
            inner: RwLock::new(settings),
        }
    }

    /// Create and load settings from disk.
    pub fn new(app: &AppHandle) -> Result<Self, SettingsError> {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|e| SettingsError::Io(format!("App data dir: {e}")))?;
        let store = SettingsStore::new(app_data);
        let settings = store.load()?;
        Ok(Self {
            store,
            inner: RwLock::new(settings),
        })
    }

    /// Get a snapshot of current settings.
    pub fn get(&self) -> AppSettings {
        self.inner.read().map(|r| r.clone()).unwrap_or_default()
    }

    /// Update settings, persist to disk, and emit change event.
    /// Returns the new settings on success.
    pub fn set(
        &self,
        app: &AppHandle,
        new_settings: AppSettings,
    ) -> Result<AppSettings, SettingsError> {
        self.store.save(&new_settings)?;
        if let Ok(mut w) = self.inner.write() {
            *w = new_settings.clone();
        }

        // Notify frontend of the change
        let _ = app.emit("settings-changed", &new_settings);

        Ok(new_settings)
    }

    /// Reset all settings to defaults, persist, and emit event.
    pub fn reset(&self, app: &AppHandle) -> Result<AppSettings, SettingsError> {
        self.set(app, AppSettings::default())
    }
}
