//! Placeholder minigame — auto-victory handler for stub game types.
//!
//! Handles: Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid.
//! These game types have no Flash SWF implementation beyond a basic shell.
//! The only supported client message is `victory` → win, once the session
//! has run for [`CLIENT_DECLARED_MIN_PLAY`] (D-MG13). An earlier `victory` is
//! refused and the session stays open.
//!
//! Reference: `python/base/minigame/Placeholder.py`

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::Instant;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// How long a client-declared game must run before its `victory` counts
/// (D-MG13). A shell's `victory` is the SWF's own word, so this floor stops a
/// client from winning the instant the game starts.
pub(crate) const CLIENT_DECLARED_MIN_PLAY: Duration = Duration::from_secs(5);

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod tests;

pub(super) struct PlaceholderGame {
    game_name: String,
    started_at: Option<Instant>,
}

impl PlaceholderGame {
    pub(super) fn new(game_name: &str) -> Self {
        Self {
            game_name: game_name.to_string(),
            started_at: None,
        }
    }
}

/// Whether a `victory` arriving at `now` may count: the game must have started,
/// and at least [`CLIENT_DECLARED_MIN_PLAY`] must have passed since then.
fn victory_allowed(started_at: Option<Instant>, now: Instant) -> bool {
    match started_at {
        Some(started) => now.saturating_duration_since(started) >= CLIENT_DECLARED_MIN_PLAY,
        None => false,
    }
}

impl MinigameInstance for PlaceholderGame {
    fn started(&mut self) -> Vec<GameOutput> {
        tracing::info!(game = %self.game_name, "Placeholder minigame started");
        self.started_at = Some(Instant::now());
        // Send a minimal fullgamestate so the SWF doesn't crash
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "gameStarted" => SfsValue::String("false".to_string()),
        })]
    }

    fn message(&mut self, cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        match cmd {
            "victory" => {
                let now = Instant::now();
                if !victory_allowed(self.started_at, now) {
                    let elapsed_ms = self.started_at.map_or(0, |started| {
                        now.saturating_duration_since(started).as_millis()
                    }) as u64;
                    tracing::warn!(
                        game = %self.game_name,
                        elapsed_ms,
                        reason = "min_play_time",
                        "Client-declared minigame victory refused: under the minimum play time"
                    );
                    return vec![];
                }
                tracing::info!(game = %self.game_name, "Placeholder minigame: victory");
                vec![
                    GameOutput::Send(sfs_vars! {
                        "_cmd" => SfsValue::String("victory".to_string()),
                    }),
                    GameOutput::Victory,
                ]
            }
            _ => {
                tracing::debug!(game = %self.game_name, cmd, "Placeholder: ignoring unknown command");
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("failure".to_string()),
        })]
    }

    fn needs_tick(&self) -> bool {
        false
    }
}
