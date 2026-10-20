//! Placeholder minigame — auto-victory handler for stub game types.
//!
//! Handles: Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid.
//! These game types have no Flash SWF implementation beyond a basic shell.
//! The only supported client message is `victory` → win, once the session
//! has run for [`CLIENT_DECLARED_MIN_PLAY`] (D-MG13). An earlier `victory` is
//! refused and the session stays open.
//!
//! The shell shows a 60 s countdown (MG-64). When it runs out with no
//! `victory`, the server sends `failure` and the game is a defeat.
//!
//! Reference: `python/base/minigame/Placeholder.py`

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::Instant;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;
use crate::sfs_vars;

/// How long a client-declared game must run before its `victory` counts
/// (D-MG13). A shell's `victory` is the SWF's own word, so this floor stops a
/// client from winning the instant the game starts.
pub(crate) const CLIENT_DECLARED_MIN_PLAY: Duration = Duration::from_secs(5);

/// The shell's countdown in seconds (MG-64). The value is sent as
/// `timeRemaining`, and the server times the game out when it reaches zero.
const TIME_LIMIT_SECS: u32 = 60;

/// Session ticks in the countdown: 60 s at the 250 ms tick is 240 ticks.
const TIMEOUT_TICKS: u32 = 240;

/// Refused `victory` frames logged in full before sampling. Past this, one
/// row in every 256 is logged, so a flood of early frames cannot fill the log.
const REFUSAL_LOG_CAP: u32 = 16;

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod tests;

pub(super) struct PlaceholderGame {
    game_name: String,
    /// Entity and player identity, so a refused `victory` names the player
    /// (TG-MG-05). Zero and `None` when built by [`PlaceholderGame::new`].
    entity_id: u32,
    player_name: Option<String>,
    started_at: Option<Instant>,
    ticks: u32,
    timed_out: bool,
    /// Early `victory` frames refused so far, logged or not.
    refusals: u32,
}

impl PlaceholderGame {
    pub(super) fn new(game_name: &str) -> Self {
        Self {
            game_name: game_name.to_string(),
            entity_id: 0,
            player_name: None,
            started_at: None,
            ticks: 0,
            timed_out: false,
            refusals: 0,
        }
    }

    /// The placeholder for `session`, carrying the player's identity into
    /// its refusal rows.
    pub(super) fn for_session(session: &MinigameSession) -> Self {
        let mut game = Self::new(&session.game_name);
        game.entity_id = session.entity_id;
        game.player_name = session.player_name.clone();
        game
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
        // The shell reads the timer from fullgamestate, then starts it running.
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("fullgamestate".to_string()),
                "timerState" => SfsValue::Number(0.0),
                "timeRemaining" => SfsValue::Number(f64::from(TIME_LIMIT_SECS)),
            }),
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("timerupdate".to_string()),
                "timerState" => SfsValue::Number(1.0),
                "timeRemaining" => SfsValue::Number(f64::from(TIME_LIMIT_SECS)),
            }),
        ]
    }

    fn message(&mut self, cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        // After the timeout the game is over and nothing is acted on.
        if self.timed_out {
            return vec![];
        }
        match cmd {
            "victory" => {
                let now = Instant::now();
                if !victory_allowed(self.started_at, now) {
                    self.refusals = self.refusals.saturating_add(1);
                    if self.refusals <= REFUSAL_LOG_CAP || self.refusals.is_multiple_of(256) {
                        let elapsed_ms = self.started_at.map_or(0, |started| {
                            now.saturating_duration_since(started).as_millis()
                        }) as u64;
                        tracing::warn!(
                            entity_id = self.entity_id,
                            entity_name = self.player_name.as_deref(),
                            game = %self.game_name,
                            elapsed_ms,
                            reason = "min_play_time",
                            refusals = self.refusals,
                            "Client-declared minigame victory refused: under the minimum play time"
                        );
                    }
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
        if self.started_at.is_none() || self.timed_out {
            return vec![];
        }
        self.ticks += 1;
        if self.ticks < TIMEOUT_TICKS {
            return vec![];
        }
        self.timed_out = true;
        tracing::debug!(
            game = %self.game_name,
            reason = "timeout",
            "Placeholder minigame timed out after the 60 s countdown"
        );
        vec![
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("failure".to_string()),
            }),
            GameOutput::Failure,
        ]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("failure".to_string()),
        })]
    }

    fn needs_tick(&self) -> bool {
        true
    }
}
