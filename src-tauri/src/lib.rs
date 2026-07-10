#![allow(dead_code)]

mod buffer;
mod capture;
mod commands;
mod encoder;
mod games;
mod hotkey;
mod recording;
mod settings;
mod tray;
mod upload;

use std::sync::Mutex;

use recording::Recorder;
use settings::SettingsManager;
use tauri::{Emitter, Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Initialize settings from disk (graceful fallback)
            let settings_mgr = match SettingsManager::new(app.handle()) {
                Ok(mgr) => mgr,
                Err(e) => {
                    eprintln!("Warning: Failed to load settings ({e}), using defaults");
                    let app_data = app
                        .path()
                        .app_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));
                    let store = settings::store::SettingsStore::new(app_data);
                    SettingsManager::with_store(store)
                }
            };
            app.manage(settings_mgr);

            // Initialize game registry
            let game_registry = games::database::GameRegistry::new();
            app.manage(game_registry);

            // Initialize upload queue
            let upload_queue = upload::queue::UploadQueue::new();
            app.manage(upload_queue);

            // Initialize recorder
            let settings = app.state::<SettingsManager>().get();
            let recorder = Mutex::new(Recorder::new(&settings));
            app.manage(recorder);

            // Register global hotkeys from saved settings
            if let Err(e) = hotkey::register_hotkeys(app.handle(), &settings.hotkeys) {
                eprintln!("Warning: Failed to register hotkeys: {e}");
            }

            // Build system tray (graceful fallback — non-fatal)
            if let Err(e) = tray::build_tray(app.handle()) {
                eprintln!("Warning: Failed to build system tray: {e}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::list_clips,
            commands::library::delete_clip,
            commands::library::rename_clip,
            commands::library::open_clip_location,
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::is_recording,
            commands::recording::save_clip,
            commands::recording::get_preview_frame,
            commands::recording::get_buffer_info,
            commands::recording::get_capture_sources,
            commands::recording::set_capture_target,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::settings::validate_hotkey,
            commands::upload::upload_clip_to_server,
            commands::upload::get_upload_queue,
            commands::upload::clear_upload_queue,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let settings = app.state::<SettingsManager>().get();
                if settings.general.minimize_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Fatal: Failed to build Tauri application: {e}");
            std::process::exit(1);
        })
        .run(move |app_handle, event| {
            match event {
                RunEvent::Ready => {
                    // Auto-start recording if enabled (deferred to here so the
                    // Tokio runtime is active and tokio::spawn works).
                    let rec_state = app_handle.state::<Mutex<Recorder>>();
                    if let Some(settings) = app_handle.try_state::<SettingsManager>() {
                        let s = settings.get();
                        if s.recording.always_on_recording {
                            if let Ok(guard) = rec_state.lock() {
                                let _ = guard.start_recording();
                            }
                            let _ = app_handle.emit("recording-state-changed", true);
                            if let Ok(guard) = rec_state.lock() {
                                guard.start_polling(app_handle.clone());
                            }
                        }
                    }
                }
                RunEvent::ExitRequested { .. } => {
                    // Stop recording on exit
                    if let Some(rec) = app_handle.try_state::<Mutex<Recorder>>() {
                        if let Ok(guard) = rec.lock() {
                            let _ = guard.stop_recording();
                        }
                    }
                }
                _ => {}
            }
        });
}
