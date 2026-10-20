//! CrystalGame's end of game: the server-decided victory, the 16 s clock and
//! the timeout (docs/analysis/minigames/spec-crystalgame.md, sections 6 and 8).
//!
//! The clock starts on the first valid click (in [`moves`](super::moves)) and
//! loses 0.25 s per 250 ms tick. A solved board ends the game with `victory`
//! and stops the clock; the original did not stop it, so it later sent a
//! `failure` as well. Nothing is sent once the game has ended.

use std::collections::HashMap;

use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::setup::Board;
use super::CrystalGame;

/// Seconds the clock loses per 250 ms tick. Exact in binary, so 64 ticks
/// take the full 16 s to zero.
const TICK_SECS: f64 = 0.25;

/// `victory {}`: the player has solved the board.
fn victory() -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("victory".to_string()),
    }
}

/// `failure {}`: the clock ran out.
fn failure() -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("failure".to_string()),
    }
}

impl CrystalGame {
    /// Ends the game with `victory` when the board is solved. Called after
    /// every `processmove`; a refused click gives no output and is left alone,
    /// so only an accepted click (which always sends updateslots) can win.
    /// Stopping the clock here is what stops a later `failure`.
    pub(super) fn settle_click(&mut self, mut out: Vec<GameOutput>) -> Vec<GameOutput> {
        if !out.is_empty() && self.outcome.is_none() && self.board == Board::solved() {
            self.outcome = Some("victory");
            self.timer_state = 0;
            tracing::info!(
                entity_id = self.entity_id,
                entity_name = self.player_name.as_deref(),
                player_id = self.player_id,
                player_name = self.player_name.as_deref(),
                "CrystalGame: victory"
            );
            out.push(GameOutput::Send(victory()));
            out.push(GameOutput::Victory);
        }
        out
    }

    /// One 250 ms tick. Does nothing while the clock is off (before the first
    /// click, or after the end). Otherwise it loses [`TICK_SECS`], and at zero
    /// or below the game is lost: `failure` is sent and the game ends.
    pub(super) fn tick_clock(&mut self) -> Vec<GameOutput> {
        if self.timer_state != 1 {
            return vec![];
        }
        self.time_remaining -= TICK_SECS;
        if self.time_remaining > 0.0 {
            return vec![];
        }
        self.timer_state = 0;
        self.outcome = Some("defeat");
        tracing::info!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            reason = "timeout",
            "CrystalGame: defeat"
        );
        vec![GameOutput::Send(failure()), GameOutput::Failure]
    }
}
