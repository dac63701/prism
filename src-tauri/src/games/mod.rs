//! External, anti-cheat-safe game detection and automatic moment handling.
//!
//! Prism only identifies a game from its top-level window and receives events
//! from documented APIs or the operating system. It never reads or injects into
//! a game process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub mod cs2;
pub mod database;
pub mod moment;
#[cfg(target_os = "windows")]
pub mod rust;
pub mod trigger;

use database::GameRegistry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DetectedGame {
    pub name: String,
    pub pid: u32,
}

/// Tracks the game window currently available to Prism's supported detectors.
pub struct GameDetector {
    active: Mutex<Option<DetectedGame>>,
    polling: AtomicBool,
}

impl GameDetector {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(None),
            polling: AtomicBool::new(false),
        }
    }

    pub fn active_game(&self) -> Option<DetectedGame> {
        self.active.lock().ok().and_then(|game| game.clone())
    }

    fn set_active_game(&self, next: Option<DetectedGame>) -> Option<Option<DetectedGame>> {
        let mut active = self.active.lock().ok()?;
        if *active == next {
            return None;
        }
        let previous = active.clone();
        *active = next;
        Some(previous)
    }

    pub fn start_polling(&self, app: AppHandle) {
        if self.polling.swap(true, Ordering::SeqCst) {
            return;
        }

        tauri::async_runtime::spawn(async move {
            loop {
                let settings = app.state::<crate::settings::SettingsManager>().get();
                let next = if settings.general.game_detection_enabled {
                    let registry = app.state::<GameRegistry>();
                    detect_running_game(&registry)
                } else {
                    None
                };

                let detector = app.state::<GameDetector>();
                if let Some(previous) = detector.set_active_game(next.clone()) {
                    if let Some(old) = previous {
                        let _ = app.emit("game-lost", old);
                    }
                    if let Some(found) = next.clone() {
                        let _ = app.emit("game-detected", found);
                    }
                }

                if next.as_ref().map(|game| game.name.as_str()) == Some("Counter-Strike 2") {
                    if let Err(error) = cs2::ensure_gsi_config(settings.general.cs2_gsi_port) {
                        eprintln!("[cs2-gsi] failed to install configuration: {error}");
                    }
                }

                #[cfg(target_os = "windows")]
                {
                    let audio = app.state::<rust::RustAudioEngine>();
                    audio.update_active_game(&app, next.as_ref());
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

#[cfg(target_os = "windows")]
fn detect_running_game(registry: &GameRegistry) -> Option<DetectedGame> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    struct Context<'a> {
        registry: &'a GameRegistry,
        found: Option<DetectedGame>,
    }

    unsafe extern "system" fn visit_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = unsafe { &mut *(lparam.0 as *mut Context<'_>) };
        if !unsafe { IsWindowVisible(hwnd).as_bool() } {
            return BOOL(1);
        }

        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return BOOL(1);
        }

        let mut text = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut text) };
        if copied <= 0 {
            return BOOL(1);
        }

        let title = String::from_utf16_lossy(&text[..copied as usize]);
        let Some(game) = context.registry.detect_by_window_title(&title) else {
            return BOOL(1);
        };

        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        context.found = Some(DetectedGame {
            name: game.name,
            pid,
        });
        BOOL(0)
    }

    let mut context = Context {
        registry,
        found: None,
    };
    let ptr = &mut context as *mut Context<'_> as isize;
    let _ = unsafe { EnumWindows(Some(visit_window), LPARAM(ptr)) };
    context.found
}

#[cfg(not(target_os = "windows"))]
fn detect_running_game(_registry: &GameRegistry) -> Option<DetectedGame> {
    None
}
