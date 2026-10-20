//! CrystalGame: a 14 x 13 lights-out board that the player solves by
//! clicking. The port lands in MG-CG1..3 (docs/analysis/minigames/spec-crystalgame.md).
//!
//! This file holds the [`CrystalGame`] struct and its [`MinigameInstance`]
//! impl; the board model (the click, the scramble, `fullgamestate`) lives in
//! [`setup`]. Until MG-CG3 sets [`PORTED`], [`create`] still hands the name to
//! the placeholder, so the port is not reachable from content.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

mod setup;

#[cfg(test)]
mod tests_setup;

/// Flipped to `true` by MG-CG3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;

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
    board: setup::Board,
}

impl CrystalGame {
    fn new(session: &MinigameSession) -> Self {
        let mut rng = StdRng::seed_from_u64(u64::from(session.seed));
        let mut board = setup::Board::solved();
        board.scramble(&mut rng);
        Self { board }
    }
}

impl MinigameInstance for CrystalGame {
    fn started(&mut self) -> Vec<GameOutput> {
        vec![GameOutput::Send(setup::fullgamestate(&self.board))]
    }

    fn message(&mut self, cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        // Clicks, reset and the cheat commands are wired in MG-CG2.
        tracing::debug!(game = "CrystalGame", cmd, "CrystalGame: unhandled command");
        vec![]
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        // The end-of-game messages are defined with the timer in MG-CG3.
        vec![]
    }

    fn needs_tick(&self) -> bool {
        false
    }
}
