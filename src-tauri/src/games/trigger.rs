//! Shared event-to-clip bridge for all supported game detectors.

use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::recording::save_clip_internal;
use crate::games::moment::{GameMoment, MomentType};
use crate::recording::Recorder;
use crate::settings::config::PerGameAutoClip;
use crate::settings::SettingsManager;

pub struct AutoClipTrigger {
    last_triggered: StdMutex<HashMap<String, Instant>>,
}

impl AutoClipTrigger {
    pub fn new() -> Self {
        Self {
            last_triggered: StdMutex::new(HashMap::new()),
        }
    }

    fn allow(&self, game: &str, cooldown_secs: u32) -> bool {
        let Ok(mut triggered) = self.last_triggered.lock() else {
            return false;
        };
        let now = Instant::now();
        if let Some(last) = triggered.get(game) {
            if now.duration_since(*last).as_secs() < cooldown_secs as u64 {
                return false;
            }
        }
        triggered.insert(game.to_string(), now);
        true
    }
}

/// Queue a clip for a verified external game event. The only recorder lock is
/// held while cloning buffered packets; MP4 muxing runs outside that lock.
pub fn trigger_auto_clip(app: &AppHandle, moment: GameMoment) {
    let active_game = app.state::<crate::games::GameDetector>().active_game();
    if active_game.as_ref().map(|game| game.name.as_str()) != Some(moment.game_name.as_str()) {
        return;
    }

    let settings = app.state::<SettingsManager>().get();
    if !settings.auto_clip.enabled {
        return;
    }

    let Some(game) = settings
        .auto_clip
        .games
        .iter()
        .find(|game| game.game_name == moment.game_name && game.enabled)
    else {
        return;
    };

    if !game.events.iter().any(|event| event == moment.event_key()) {
        return;
    }

    let trigger = app.state::<AutoClipTrigger>();
    if !trigger.allow(&moment.game_name, settings.auto_clip.cooldown_secs) {
        return;
    }

    let duration = clip_duration(game, moment.moment_type);
    let filename = format!(
        "{}_{}_{}.mp4",
        file_name_part(&moment.game_name),
        crate::recording::chrono_now_formatted(),
        moment.type_label().replace(' ', "")
    );
    let _ = app.emit("auto-clip-triggered", moment.clone());

    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let settings = handle.state::<SettingsManager>().get();
        let recorder = handle.state::<Recorder>();
        if !recorder.is_recording() {
            return;
        }
        if let Err(error) = save_clip_internal(&handle, &recorder, &settings, duration, filename) {
            eprintln!(
                "[auto-clip] failed to save {} clip: {error}",
                moment.game_name
            );
            let _ = handle.emit("auto-clip-failed", error);
        }
    });
}

fn clip_duration(game: &PerGameAutoClip, moment_type: MomentType) -> u32 {
    match moment_type {
        MomentType::Death => game.death_clip_duration,
        MomentType::Kill | MomentType::Headshot => game.kill_clip_duration,
        _ => game.combat_event_duration,
    }
}

fn file_name_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
}
