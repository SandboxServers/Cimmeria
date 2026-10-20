//! ConverseBasicHumanoid: a timed card contest against a server-side NPC
//! (docs/analysis/minigames/spec-conversebasichumanoid.md).
//!
//! MG-CH1 lands the state, the seeded deal, the accord threshold and the
//! `fullgamestate` builder. Plays, scoring and timers follow in MG-CH2 and
//! MG-CH3, so the state is not read yet. `create()` keeps the placeholder
//! until MG-CH3 flips `PORTED` (C4).
#![allow(dead_code)] // Read by MG-CH2 and MG-CH3; no caller until PORTED flips.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

mod moves;
mod setup;

#[cfg(test)]
#[path = "tests_moves.rs"]
mod tests_moves;
#[cfg(test)]
#[path = "tests_setup.rs"]
mod tests_setup;

/// Flipped to `true` by MG-CH3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;

/// The placeholder until `PORTED` is set (C4): the port is not reachable yet.
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
}

/// Countdown at join. The server starts 0.25 s ahead of the client's 30 s.
const INITIAL_TIME: f64 = 30.25;
/// Seconds between NPC moves (the first move after a deal is 3 s in).
const NPC_TIMER: f64 = 3.0;
/// Seconds per hand before the hand ends on its own.
const HAND_TIMER: f64 = 6.0;

/// Who ended the game: the player's victory, or a defeat (timeout, `failure`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Victory,
    Defeat,
}

/// One ConverseBasicHumanoid session. Each side has a hand, the containers it
/// fills and the plays it has made; the timers run against `time_remaining`.
pub(crate) struct ConverseHumanoidGame {
    entity_id: u32,
    player_id: i32,
    rng: StdRng,
    /// Index into the game-type table (`setup::GAME_TYPES`).
    game_type: usize,
    npc_hand: Vec<f64>,
    player_hand: Vec<f64>,
    /// `true` while the container is empty, one entry per container.
    npc_empty: Vec<bool>,
    player_empty: Vec<bool>,
    npc_contest_plays: Vec<f64>,
    player_contest_plays: Vec<f64>,
    npc_total_plays: u32,
    player_total_plays: u32,
    npc_score: f64,
    player_score: f64,
    time_remaining: f64,
    timer_state: u32,
    npc_last_time: f64,
    npc_timer: f64,
    hand_last_time: f64,
    hand_timer: f64,
    contests_without_accord: u32,
    num_resets: u32,
    /// One flag per entry in `setup::PLAYER_VICTORIES`.
    player_victory_used: Vec<bool>,
    outcome: Option<Outcome>,
    /// The player's play into a container the NPC has not filled: the
    /// container and the suit the NPC answers (spec section 8).
    npc_focus: Option<(usize, f64)>,
    /// Shown in refusal logs, paired with `entity_id`.
    player_name: Option<String>,
    intelligence: u32,
    tech_competency: u32,
    player_level: u32,
    abilities_mask: u32,
}

impl ConverseHumanoidGame {
    /// A game for `session`. Nothing is dealt until `started()`, so the RNG
    /// draws begin with the first deal.
    pub(crate) fn new(session: &MinigameSession) -> Self {
        Self {
            entity_id: session.entity_id,
            player_id: session.player_id,
            rng: StdRng::seed_from_u64(session.seed as u64),
            game_type: setup::FIRST_GAME_TYPE,
            npc_hand: Vec::new(),
            player_hand: Vec::new(),
            npc_empty: Vec::new(),
            player_empty: Vec::new(),
            npc_contest_plays: Vec::new(),
            player_contest_plays: Vec::new(),
            npc_total_plays: 0,
            player_total_plays: 0,
            npc_score: 0.0,
            player_score: 0.0,
            time_remaining: INITIAL_TIME,
            timer_state: 1,
            npc_last_time: INITIAL_TIME,
            npc_timer: NPC_TIMER,
            hand_last_time: INITIAL_TIME,
            hand_timer: HAND_TIMER,
            contests_without_accord: 0,
            num_resets: 0,
            player_victory_used: vec![false; setup::PLAYER_VICTORIES.len()],
            outcome: None,
            npc_focus: None,
            player_name: session.player_name.clone(),
            intelligence: session.intelligence,
            tech_competency: session.tech_competency,
            player_level: session.player_level,
            abilities_mask: session.abilities_mask,
        }
    }
}

impl MinigameInstance for ConverseHumanoidGame {
    fn started(&mut self) -> Vec<GameOutput> {
        self.deal();
        vec![GameOutput::Send(self.full_game_state())]
    }

    /// Plays and the `newhand`, `processreset` and `activatevictory` commands
    /// arrive in MG-CH2 and MG-CH3. Until then every message is ignored.
    fn message(&mut self, _cmd: &str, _params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        Vec::new()
    }

    /// The countdown and the NPC and hand timers arrive in MG-CH3.
    fn tick(&mut self) -> Vec<GameOutput> {
        Vec::new()
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        Vec::new()
    }

    fn needs_tick(&self) -> bool {
        false
    }
}
