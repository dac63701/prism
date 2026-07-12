//! Game moment — represents a significant in-game event that triggered a clip.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Types of moments that can trigger auto-clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MomentType {
    Kill,
    Death,
    Headshot,
    Explosion,
    Combat,
    Win,
    MatchEnd,
    Manual,
}

/// A game moment — metadata about why a clip was saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMoment {
    pub moment_type: MomentType,
    pub game_name: String,
    /// Unix timestamp in seconds
    pub timestamp_secs: u64,
    pub description: Option<String>,
}

impl GameMoment {
    pub fn new(moment_type: MomentType, game_name: String) -> Self {
        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            moment_type,
            game_name,
            timestamp_secs,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    /// Get a human-readable label for the moment type.
    pub fn type_label(&self) -> &'static str {
        match self.moment_type {
            MomentType::Kill => "Kill",
            MomentType::Death => "Death",
            MomentType::Headshot => "Headshot",
            MomentType::Explosion => "Explosion",
            MomentType::Combat => "Combat",
            MomentType::Win => "Win",
            MomentType::MatchEnd => "Match End",
            MomentType::Manual => "Manual Clip",
        }
    }

    pub fn event_key(&self) -> &'static str {
        match self.moment_type {
            MomentType::Kill => "kill",
            MomentType::Death => "death",
            MomentType::Headshot => "headshot",
            MomentType::Explosion => "explosion",
            MomentType::Combat => "combat",
            MomentType::Win => "win",
            MomentType::MatchEnd => "match_end",
            MomentType::Manual => "manual",
        }
    }
}
