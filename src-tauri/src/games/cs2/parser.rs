use serde_json::Value;

use crate::games::moment::{GameMoment, MomentType};

#[derive(Default)]
pub(super) struct GsiState {
    initialized: bool,
    kills: u64,
    deaths: u64,
    health: i64,
    headshots: u64,
    win_team: Option<String>,
}

impl GsiState {
    pub(super) fn consume(&mut self, payload: &Value) -> Vec<GameMoment> {
        let kills = number(payload, &["player", "match_stats", "kills"]);
        let deaths = number(payload, &["player", "match_stats", "deaths"]);
        let health = signed_number(payload, &["player", "state", "health"]);
        let headshots = number(payload, &["player", "state", "round_killhs"]);
        let win_team = string(payload, &["round", "win_team"]);

        if !self.initialized {
            self.initialized = true;
            self.kills = kills;
            self.deaths = deaths;
            self.health = health;
            self.headshots = headshots;
            self.win_team = win_team;
            return Vec::new();
        }

        let mut moments = Vec::new();
        let got_headshot = headshots > self.headshots;
        if kills > self.kills {
            let kind = if got_headshot {
                MomentType::Headshot
            } else {
                MomentType::Kill
            };
            let description = if got_headshot {
                "CS2 headshot".to_string()
            } else {
                "CS2 kill".to_string()
            };
            moments.push(
                GameMoment::new(kind, "Counter-Strike 2".into()).with_description(description),
            );
        }
        if deaths > self.deaths || (health == 0 && self.health > 0) {
            moments.push(
                GameMoment::new(MomentType::Death, "Counter-Strike 2".into())
                    .with_description("CS2 death".into()),
            );
        }
        if let Some(team) = win_team.as_deref() {
            if self.win_team.as_deref() != Some(team) {
                moments.push(
                    GameMoment::new(MomentType::Win, "Counter-Strike 2".into())
                        .with_description(format!("Round won by {team}")),
                );
            }
        }

        self.kills = kills;
        self.deaths = deaths;
        self.health = health;
        self.headshots = headshots;
        self.win_team = win_team;
        moments
    }
}

fn number(payload: &Value, path: &[&str]) -> u64 {
    value_at(payload, path)
        .and_then(Value::as_u64)
        .unwrap_or_default()
}

fn signed_number(payload: &Value, path: &[&str]) -> i64 {
    value_at(payload, path)
        .and_then(Value::as_i64)
        .unwrap_or(-1)
}

fn string(payload: &Value, path: &[&str]) -> Option<String> {
    value_at(payload, path)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn value_at<'a>(mut value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    for key in path {
        value = value.get(*key)?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::GsiState;
    use crate::games::moment::MomentType;

    fn payload(kills: u64, deaths: u64, health: i64, headshots: u64) -> serde_json::Value {
        json!({
            "player": {
                "match_stats": { "kills": kills, "deaths": deaths },
                "state": { "health": health, "round_killhs": headshots }
            },
            "round": {}
        })
    }

    #[test]
    fn emits_a_headshot_instead_of_a_duplicate_kill() {
        let mut state = GsiState::default();
        assert!(state.consume(&payload(4, 1, 100, 0)).is_empty());

        let moments = state.consume(&payload(5, 1, 100, 1));
        assert_eq!(moments.len(), 1);
        assert_eq!(moments[0].moment_type, MomentType::Headshot);
    }
}
