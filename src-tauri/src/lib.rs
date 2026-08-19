#[cfg(target_os = "windows")]
mod audio;
mod auth;
mod buffer;
mod capture;
mod commands;
mod encoder;
mod games;
mod hotkey;
mod notification;
mod recording;
mod settings;
mod tray;
mod upload;

use auth::AuthManager;
use recording::Recorder;
use settings::SettingsManager;
use tauri::{Emitter, Listener, Manager, RunEvent};

// Fast, scalable allocator for the recording pipeline (thousands of small
// per-frame allocations under heavy memory churn).
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
            // Second-launch activation (e.g. a clicked toast notification)
            // forwards into the running instance. Raise the window so the
            // clip save toast click brings Prism to the foreground.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }

    builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notification::init())
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
            app.manage(games::GameDetector::new());
            app.manage(games::trigger::AutoClipTrigger::new());
            app.manage(games::cs2::Cs2GsiListener::new());
            #[cfg(target_os = "windows")]
            app.manage(games::rust::RustAudioEngine::new());

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
            app.manage(Recorder::new(&settings));

            // Initialize auth manager
            let auth_mgr = AuthManager::new();
            if let Ok(mut state) = auth_mgr.state.lock() {
                state.authenticated = !settings.cloud.access_token.is_empty();
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

            // Register system tray (graceful fallback — non-fatal)
            if let Err(e) = tray::build_tray(app.handle()) {
                eprintln!("Warning: Failed to build system tray: {e}");
            }

            // Windows: self-heal the toast AUMID registration so native
            // notifications render over every app regardless of launch path.
            #[cfg(target_os = "windows")]
            notification::register_aumid(app.handle());

            // macOS: prompt for Notification Center permission up front.
            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().request_permission();
            }

            // Custom title bar: keep decorations disabled on Windows/Linux
            // (rendered in-app), but restore native decorations + overlay
            // traffic lights on macOS so the custom bar sits under the lights.
            #[cfg(target_os = "macos")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(true);
                    let _ = window.set_title_bar_style(tauri::TitleBarStyle::Overlay);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::list_clips,
            commands::library::delete_clip,
            commands::library::rename_clip,
            commands::library::update_clip_metadata,
            commands::library::open_clip_location,
            commands::games::get_detected_game,
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::is_recording,
            commands::recording::save_clip,
            commands::recording::get_preview_frame,
            commands::recording::get_buffer_info,
            commands::recording::get_capture_sources,
            commands::recording::get_display_refresh_rate,
            commands::recording::set_capture_target,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            commands::settings::validate_hotkey,
            commands::auth::cloud_login,
            commands::auth::cloud_logout,
            commands::auth::get_auth_status,
            commands::auth::cloud_handle_auth_code,
            commands::auth::cloud_verify_auth,
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

                // Both services remain external to game processes. Game
                // detection decides when Rust's audio worker is active; CS2
                // posts to its official localhost Game State Integration API.
                app_handle
                    .state::<games::cs2::Cs2GsiListener>()
                    .start(app_handle.clone());
                let gsi_port = app_handle
                    .state::<SettingsManager>()
                    .get()
                    .general
                    .cs2_gsi_port;
                match games::cs2::ensure_gsi_config(gsi_port) {
                    Ok(Some(path)) => {
                        eprintln!("[cs2-gsi] configuration ready at {}", path.display())
                    }
                    Ok(None) => eprintln!(
                        "[cs2-gsi] CS2 installation was not found in a standard Steam library"
                    ),
                    Err(error) => eprintln!("[cs2-gsi] failed to install configuration: {error}"),
                }
                app_handle
                    .state::<games::GameDetector>()
                    .start_polling(app_handle.clone());

                // Auto-start recording if enabled
                let recorder = app_handle.state::<Recorder>();
                if let Some(settings) = app_handle.try_state::<SettingsManager>() {
                    let s = settings.get();
                    if s.recording.always_on_recording {
                        let _ = recorder.start_recording();
                        let _ = app_handle.emit("recording-state-changed", true);
                        recorder.start_polling(app_handle.clone());
                    }
                }
            }
            RunEvent::ExitRequested { .. } => {
                if let Some(recorder) = app_handle.try_state::<Recorder>() {
                    let _ = recorder.stop_recording();
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
            match AuthManager::handle_callback(&handle, code).await {
                Ok(()) => {
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                Err(e) => {
                    eprintln!("[auth] callback error: {e}");
                    let _ = handle.emit("auth-error", e);
                }
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
        let (key, value) = pair.split_once('=')?;
        if key == "code" && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}
