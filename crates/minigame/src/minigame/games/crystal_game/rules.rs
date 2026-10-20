//! CrystalGame's end of game: the server-decided victory, the 16 s clock and
//! the timeout (docs/analysis/minigames/spec-crystalgame.md, sections 6 and 8).
//!
//! The clock starts on the first valid click (in [`moves`](super::moves)) and
//! loses 0.25 s per 250 ms tick. A solved board ends the game with `victory`
//! and stops the clock; the original did not stop it, so it later sent a
//! `failure` as well. Nothing is sent once the game has ended.
//!
//! The 16 s also runs on the wall clock from the first click, and a game with
//! no click within [`PRE_CLICK_LIMIT`] of its start is lost. [`CrystalGame::expire`]
//! checks both on every frame and every tick, so a stalled or flooding client
//! cannot win or stay in the game after the deadline.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

use super::setup::Board;
use super::CrystalGame;

/// Seconds the clock loses per 250 ms tick. Exact in binary, so 64 ticks
/// take the full 16 s to zero.
const TICK_SECS: f64 = 0.25;

/// How long a game waits for its first click before it is lost. Frames that
/// are not clicks do not extend it.
pub(super) const PRE_CLICK_LIMIT: Duration = Duration::from_secs(60);

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

    /// One 250 ms tick. Runs [`CrystalGame::expire`] first. Then it does
    /// nothing while the clock is off (before the first click, or after the
    /// end). Otherwise it loses [`TICK_SECS`], and at zero or below the game
    /// is lost.
    pub(super) fn tick_clock(&mut self) -> Vec<GameOutput> {
        let expired = self.expire();
        if !expired.is_empty() {
            return expired;
        }
        if self.timer_state != 1 {
            return vec![];
        }
        self.time_remaining -= TICK_SECS;
        if self.time_remaining > 0.0 {
            return vec![];
        }
        self.lose("timeout")
    }

    /// Ends the game as lost when a wall-clock limit has passed: the clock's
    /// deadline once it runs, or [`PRE_CLICK_LIMIT`] from the start while no
    /// click has come. Returns the end's outputs, or none while the game goes
    /// on. Called on every frame (the click included) and on every tick.
    pub(super) fn expire(&mut self) -> Vec<GameOutput> {
        if self.outcome.is_some() {
            return vec![];
        }
        let now = Instant::now();
        if self.timer_state == 1 {
            if self.clock_deadline.is_some_and(|at| now >= at) {
                return self.lose("timeout");
            }
        } else if self.idle_deadline.is_some_and(|at| now >= at) {
            return self.lose("no_first_click");
        }
        vec![]
    }

    /// Ends the game as lost: the clock stops and `failure` is sent.
    fn lose(&mut self, reason: &'static str) -> Vec<GameOutput> {
        self.timer_state = 0;
        self.outcome = Some("defeat");
        tracing::info!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            reason,
            "CrystalGame: defeat"
        );
        vec![GameOutput::Send(failure()), GameOutput::Failure]
    }
}
