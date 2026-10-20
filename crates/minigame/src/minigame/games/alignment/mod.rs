//! Alignment: the port lands in MG-AL1..3 (docs/analysis/minigames/spec-alignment.md).
//!
//! MG-AL1 is the setup and the state shell. `create()` still hands the name to
//! the placeholder: `PORTED` stays `false` until MG-AL3, so nothing here is
//! reachable from content yet. The shell is exercised by the tests only.
#![allow(dead_code)] // unreachable until MG-AL3 flips PORTED; the tests build it

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;
use crate::sfs_vars;

mod setup;
#[cfg(test)]
mod tests_setup;

/// Flipped to `true` by MG-AL3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;

/// Where the game is in the port's phase machine (spec section 5). MG-AL1
/// only reaches `Ready`; `Ended` is set by `aborted()` and closes the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Ready,
    Ended,
}

/// Alignment game state for one session.
pub(crate) struct AlignmentGame {
    entity_id: u32,
    tech_competency: u32,
    abilities_mask: u32,
    /// Tier 1-4, clamped from the session's difficulty.
    tier: u8,
    /// Goal level 0-10, from `techcomp`.
    level: u8,
    rng: StdRng,
    /// Piece ids indexed `[x][y]`.
    board: Vec<Vec<u8>>,
    /// The board the copy reset restores.
    board_init: Vec<Vec<u8>>,
    goal_scores: Vec<u32>,
    goal_totals: Vec<u32>,
    goal_icons: Vec<u8>,
    bonus_icons: Vec<u8>,
    /// Time limit of the tier, in seconds.
    time_limit: u32,
    /// Stored as the time plus 0.25 s; every outbound copy subtracts it.
    time_remaining: f64,
    timer_state: u8,
    num_resets: u32,
    hint_index: u8,
    phase: Phase,
    outcome: Option<&'static str>,
}

impl AlignmentGame {
    /// Builds the game for a session and draws its board from the session seed.
    pub(crate) fn new(session: &MinigameSession) -> Self {
        let tier = setup::clamp_tier(session.difficulty);
        let mut rng = StdRng::seed_from_u64(u64::from(session.seed));
        let board = setup::select_icons(&mut rng, tier);
        Self::build(session, rng, board)
    }

    /// Test-only: builds the game with an injected board, so tests can predict
    /// every move (spec section 9).
    #[cfg(test)]
    pub(crate) fn with_board(session: &MinigameSession, board: Vec<Vec<u8>>) -> Self {
        let rng = StdRng::seed_from_u64(u64::from(session.seed));
        Self::build(session, rng, board)
    }

    fn build(session: &MinigameSession, rng: StdRng, board: Vec<Vec<u8>>) -> Self {
        let tier = setup::clamp_tier(session.difficulty);
        let level = setup::level_for(session.tech_competency);
        let spec = setup::tier_spec(tier);
        let goal_totals = setup::goal_totals(tier, level);
        Self {
            entity_id: session.entity_id,
            tech_competency: session.tech_competency,
            abilities_mask: session.abilities_mask,
            tier,
            level,
            rng,
            board_init: board.clone(),
            board,
            goal_scores: vec![0; goal_totals.len()],
            goal_totals,
            goal_icons: spec.goal_icons.to_vec(),
            bonus_icons: setup::BONUS_ICONS.to_vec(),
            time_limit: spec.time,
            time_remaining: f64::from(spec.time) + 0.25,
            timer_state: 1,
            num_resets: 0,
            hint_index: 0,
            phase: Phase::Ready,
            outcome: None,
        }
    }

    /// `processreset`: restores the initial board, counts the reset and resets
    /// the clock, then resends `fullgamestate` (spec section 5).
    fn reset(&mut self) -> Vec<GameOutput> {
        self.num_resets += 1;
        self.board = self.board_init.clone();
        self.time_remaining = f64::from(self.time_limit) + 0.25;
        self.timer_state = 1;
        self.hint_index = 0;
        self.goal_scores = vec![0; self.goal_totals.len()];
        self.phase = Phase::Ready;
        vec![GameOutput::Send(self.full_game_state())]
    }
}

impl MinigameInstance for AlignmentGame {
    fn started(&mut self) -> Vec<GameOutput> {
        tracing::info!(
            entity_id = self.entity_id,
            tier = self.tier,
            level = self.level,
            "Alignment started"
        );
        vec![GameOutput::Send(self.full_game_state())]
    }

    fn message(&mut self, cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        if self.phase == Phase::Ended {
            return vec![];
        }
        match cmd {
            "processreset" => self.reset(),
            _ => {
                // Every other command belongs to a later slice (AL2, AL3). A
                // client `victory` lands here too: the server decides victory.
                tracing::debug!(
                    entity_id = self.entity_id,
                    cmd,
                    "Alignment: command not handled yet"
                );
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        if self.phase == Phase::Ended {
            return vec![];
        }
        self.phase = Phase::Ended;
        self.outcome = Some("canceled");
        vec![GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("failure".to_string()),
        })]
    }

    fn needs_tick(&self) -> bool {
        false
    }
}

/// Create a minigame instance for the given session. Alignment is not ported
/// yet, so this is still the placeholder (C4).
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
}
