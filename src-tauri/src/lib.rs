#![allow(dead_code)]

mod auth;
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

use auth::AuthManager;
use recording::Recorder;
use settings::SettingsManager;
use tauri::{Emitter, Listener, Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Single-instance plugin must be registered first (handles deep-link
    // routing on Windows/Linux where the OS spawns a new process).
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            for url in &argv {
                if url.starts_with("prism://") {
                    handle_deep_link(app, url);
                }
            }
        }));
    }

    builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
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

            // Initialize upload queue with persistence
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let upload_queue = upload::queue::UploadQueue::new();
            upload_queue.set_persist_path(app_data);
            upload_queue.cleanup_completed();
            app.manage(upload_queue);

            // Initialize recorder
            let settings = app.state::<SettingsManager>().get();
            let recorder = Mutex::new(Recorder::new(&settings));
            app.manage(recorder);

            // Initialize auth manager
            let auth_mgr = AuthManager::new();
            if let Ok(mut state) = auth_mgr.state.lock() {
                state.authenticated = !settings.cloud.api_key.is_empty();
                state.display_name = settings.cloud.account_display_name.clone();
                state.email = settings.cloud.account_email.clone();
            }
            app.manage(auth_mgr);

            // Handle deep-link events (macOS / iOS — fires while app is running)
            let app_handle = app.handle().clone();
            app.listen("deep-link", move |event| {
                let url = event.payload();
                eprintln!("[auth] deep-link event received: {url}");
                handle_deep_link(&app_handle, url);
            });

            // Cold-start deep link: check if app was launched with a prism:// URL
            let args: Vec<String> = std::env::args().collect();
            for arg in &args {
                if arg.starts_with("prism://") {
                    eprintln!("[auth] cold-start deep link: {arg}");
                    handle_deep_link(app.handle(), arg);
                    break;
                }
            }

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
            commands::auth::cloud_login,
            commands::auth::cloud_logout,
            commands::auth::get_auth_status,
            commands::uploads::upload_clip,
            commands::uploads::upload_queue_status,
            commands::uploads::cancel_upload,
            commands::uploads::retry_upload,
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
        });

    let app = builder
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Fatal: Failed to build Tauri application: {e}");
            std::process::exit(1);
        });

    app.run(move |app_handle, event| {
        match event {
            RunEvent::Ready => {
                // Start background upload processor
                upload::start_upload_processor(app_handle.clone());

                // Auto-start recording if enabled
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

/// Handle a `prism://auth/callback?code=xxx` deep-link URL.
fn handle_deep_link(app: &tauri::AppHandle, url: &str) {
    if let Some(code) = extract_auth_code(url) {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = AuthManager::handle_callback(&handle, code).await {
                eprintln!("[auth] callback error: {e}");
                let _ = handle.emit("auth-error", e);
            }
        });
    }
}

/// Extract the `code` query parameter from a `prism://auth/callback?code=xxx` URL.
fn extract_auth_code(url: &str) -> Option<String> {
    let url = url.trim();
    let query_start = url.find('?')?;
    let query = &url[query_start + 1..];
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next()?;
        if key == "code" && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}
