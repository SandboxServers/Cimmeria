//! CrystalGame: a 14 x 13 lights-out board that the player solves by
//! clicking. The port lands in MG-CG1..3 (docs/analysis/minigames/spec-crystalgame.md).
//!
//! This file holds the [`CrystalGame`] struct and its [`MinigameInstance`]
//! impl; the board model (the click, the scramble, `fullgamestate`) lives in
//! [`setup`], the client commands (click, reset, refusals) in [`moves`], and
//! the clock, victory and timeout in [`rules`].

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

mod moves;
mod rules;
mod setup;

#[cfg(test)]
mod tests_moves;
#[cfg(test)]
mod tests_rules;
#[cfg(test)]
mod tests_setup;

/// Flipped to `true` by MG-CG3 only, once victory is server-decided.
pub(crate) const PORTED: bool = true;

pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    if PORTED {
        Box::new(CrystalGame::new(session))
    } else {
        Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
    }
}

/// One CrystalGame session. The board is scrambled from the session seed
/// when the game is built, so a given seed always opens the same board.
struct CrystalGame {
    // Identity, carried so every rejection row names the player (TG-MG-05).
    entity_id: u32,
    player_id: i32,
    player_name: Option<String>,
    board: setup::Board,
    /// 0 while idle (no click yet), 1 once the first valid click started
    /// the clock.
    timer_state: u32,
    /// Seconds left on the clock, counted down by [`rules`] while it runs.
    time_remaining: f64,
    /// `Some` once the game has ended: `"victory"`, `"defeat"` on timeout, or
    /// `"canceled"` after a disconnect.
    outcome: Option<&'static str>,
}

/// The longest client command name written to a log row. The client chooses
/// `cmd`, so an unbounded value would let one connection flood the log.
const MAX_LOGGED_CMD: usize = 32;

/// `cmd` cut to [`MAX_LOGGED_CMD`] characters for a log row.
fn capped_cmd(cmd: &str) -> String {
    cmd.chars().take(MAX_LOGGED_CMD).collect()
}

impl CrystalGame {
    fn new(session: &MinigameSession) -> Self {
        let mut rng = StdRng::seed_from_u64(u64::from(session.seed));
        let mut board = setup::Board::solved();
        board.scramble(&mut rng);
        Self {
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            board,
            timer_state: 0,
            time_remaining: f64::from(setup::TIME_LIMIT),
            outcome: None,
        }
    }

    /// Logs a refused command at WARN with the player's identity and the
    /// `reason` that the packet's message rules name. The caller sends nothing.
    fn reject(&self, cmd: &str, reason: &'static str) {
        let logged_cmd = capped_cmd(cmd);
        tracing::warn!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            cmd = logged_cmd.as_str(),
            reason,
            "CrystalGame: command refused"
        );
    }
}

impl MinigameInstance for CrystalGame {
    fn started(&mut self) -> Vec<GameOutput> {
        vec![GameOutput::Send(setup::fullgamestate(&self.board))]
    }

    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        match cmd {
            "processmove" => {
                let out = self.process_move(params);
                self.settle_click(out)
            }
            "processreset" => self.process_reset(),
            "activateitemcheat" => {
                // CrystalGame has no item cheats (M8).
                self.reject(cmd, "item_cheat_unavailable");
                vec![]
            }
            "activateabilitycheat" => {
                // CrystalGame implements no ability cheat (spec section 7).
                self.reject(cmd, "ability_cheat_unavailable");
                vec![]
            }
            // A client `victory` is refused: the server decides victory.
            "victory" => {
                self.reject(cmd, "client_victory_refused");
                vec![]
            }
            _ => {
                self.reject(cmd, "unknown_command");
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        self.tick_clock()
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        // The client has gone (D-MG17): end the game silently, clock stopped.
        self.outcome = Some("canceled");
        self.timer_state = 0;
        vec![]
    }

    /// The session samples this once, before its first tick, so it stays true
    /// from the start: an idle game's tick does nothing (the clock is off until
    /// the first click). It is false once the game has ended.
    fn needs_tick(&self) -> bool {
        self.outcome.is_none()
    }
}
