//! GoauldCrystals: a path-building puzzle (docs/analysis/minigames/spec-goauldcrystals.md).
//!
//! Greens move through empty cells until a green path powers the finish
//! goal. This file is the runtime: the [`GoauldCrystalsGame`] struct, its
//! constructor and the [`MinigameInstance`] impl. The difficulty table, the
//! board generation and the `fullgamestate` builders live in [`setup`].

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

mod moves;
mod power;
mod setup;

#[cfg(test)]
mod tests_log;
#[cfg(test)]
mod tests_moves;
#[cfg(test)]
mod tests_power;
#[cfg(test)]
mod tests_setup;

/// Flipped to `true` by MG-GC3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;

/// Builds the game for a session. Until [`PORTED`] is set the placeholder
/// answers, so a half-ported game is never reachable from content.
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    if PORTED {
        Box::new(GoauldCrystalsGame::new(session))
    } else {
        Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
    }
}

/// Longest client text written to a log line, in characters.
const LOG_TEXT_CAP: usize = 64;

/// Client-supplied text as it may appear in a log line: cut to
/// [`LOG_TEXT_CAP`] characters, with control characters shown as `?`, so one
/// frame cannot fill the log or split a line.
fn loggable(text: &str) -> String {
    text.chars()
        .take(LOG_TEXT_CAP)
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

// ── Board types ──────────────────────────────────────────────────────────────

/// One board cell. `color`: 0 empty, 1 green, 2 yellow, 3 orange, 4 red.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) color: u8,
    /// Visual variant only; no rule reads it.
    pub(super) var: u8,
    /// True when the cell is on the powered green path.
    pub(super) power: bool,
}

/// A start or finish goal on the board edge. `side` 0 is top, 1 right,
/// 2 bottom, 3 left; `offset` runs along that side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Goal {
    pub(super) side: u8,
    pub(super) offset: usize,
    pub(super) x: usize,
    pub(super) y: usize,
}

// ── GoauldCrystals game ──────────────────────────────────────────────────────

pub(super) struct GoauldCrystalsGame {
    // Session identity and parameters
    /// Entity and player identity, carried so every warning names the player.
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    pub(super) player_name: Option<String>,
    /// Session seed; seed 1 on a 4x4 board selects the tutorial layout.
    pub(super) seed: u32,
    /// Difficulty table row, clamped to 0-10.
    pub(super) level: usize,
    /// Difficulty column, clamped to 1-4.
    pub(super) difficulty: u32,
    /// Session ability mask; `ability_bitfield` starts from it on every game.
    pub(super) abilities_mask: u32,
    pub(super) ability_bitfield: u32,

    // Board
    pub(super) w: usize,
    pub(super) h: usize,
    /// Indexed `board[x][y]`, the same layout as the client's `gameArray`.
    pub(super) board: Vec<Vec<Cell>>,
    pub(super) start: Goal,
    pub(super) finish: Goal,
    pub(super) selected: Option<(usize, usize)>,

    // Clock
    pub(super) time_remaining: f64,
    pub(super) max_time: f64,
    /// 0 stopped, 1 running, 2 paused, 3 running backwards.
    pub(super) timer_state: u32,

    // Rule bookkeeping, reset by every game start
    pub(super) yellows_without_orange: u32,
    pub(super) reds_to_yellows: u32,
    pub(super) reds_to_remove: u32,
    pub(super) reds_to_revert: Option<Vec<(usize, usize)>>,
    pub(super) item_cheats_add_time: bool,
    /// Kept across resets; the reset count is reported on completion.
    #[allow(dead_code)] // read by the MG-GC3a reset path
    pub(super) num_resets: u32,
    pub(super) outcome: Option<&'static str>,

    // RNG (seeded, Send-safe)
    pub(super) rng: StdRng,
}

impl GoauldCrystalsGame {
    pub(super) fn new(session: &MinigameSession) -> Self {
        Self {
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            seed: session.seed,
            level: setup::clamp_level(session.tech_competency),
            difficulty: setup::clamp_difficulty(session.difficulty),
            abilities_mask: session.abilities_mask,
            ability_bitfield: session.abilities_mask,
            w: 0,
            h: 0,
            board: Vec::new(),
            start: Goal::default(),
            finish: Goal::default(),
            selected: None,
            time_remaining: 0.0,
            max_time: 0.0,
            timer_state: 0,
            yellows_without_orange: 0,
            reds_to_yellows: 0,
            reds_to_remove: 0,
            reds_to_revert: None,
            item_cheats_add_time: false,
            num_resets: 0,
            outcome: None,
            rng: StdRng::seed_from_u64(session.seed as u64),
        }
    }
}

impl MinigameInstance for GoauldCrystalsGame {
    fn started(&mut self) -> Vec<GameOutput> {
        self.init_game();
        vec![GameOutput::Send(self.build_full_game_state())]
    }

    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        if cmd == "playfieldclick" {
            return self.playfield_click(params);
        }
        // Cheat and reset commands are ported by MG-GC3.
        tracing::warn!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            cmd = %loggable(cmd),
            reason = "unknown_command",
            "GoauldCrystals: unknown command"
        );
        vec![]
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    fn needs_tick(&self) -> bool {
        false
    }
}
