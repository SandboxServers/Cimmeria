//! ConverseBasicHumanoid: a timed card contest against a server-side NPC
//! (docs/analysis/minigames/spec-conversebasichumanoid.md).
//!
//! [`setup`] holds the table, the deal and `fullgamestate`, [`moves`] the
//! plays, the NPC and hand scoring, and [`rules`] the timers, the deal flow,
//! `activatevictory` and the result. This file holds the struct, the command
//! dispatch and the [`MinigameInstance`] impl.
//!
//! The `contest_total` column of the game-type table is kept for the spec
//! table; it is the only unread item.

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
#[path = "tests_moves.rs"]
mod tests_moves;
#[cfg(test)]
#[path = "tests_rules.rs"]
mod tests_rules;
#[cfg(test)]
#[path = "tests_setup.rs"]
mod tests_setup;

/// Set by MG-CH3: victory is server-decided and the port is reachable.
pub(crate) const PORTED: bool = true;

/// The real game once `PORTED` is set (C4); the placeholder shell otherwise.
pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
    if PORTED {
        Box::new(ConverseHumanoidGame::new(session))
    } else {
        Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
    }
}

/// Countdown at join. The server starts 0.25 s ahead of the client's 30 s.
const INITIAL_TIME: f64 = 30.25;
/// Seconds before the first NPC move of a deal (the later moves are 1.5 s apart).
const NPC_TIMER: f64 = 3.0;
/// Seconds per hand before the hand ends on its own.
const HAND_TIMER: f64 = 6.0;
/// The longest client command name written to a log row. The client chooses
/// `cmd`, so an unbounded value would let one connection flood the log.
const MAX_LOGGED_CMD: usize = 32;

/// How the game ended: the player's victory, a defeat (timeout, `failure`), or
/// the client leaving (`aborted`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Victory,
    Defeat,
    Canceled,
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
    /// True from the newdeal reply until processreset: the NPC and hand timers are held.
    awaiting_reset: bool,
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
            awaiting_reset: false,
            npc_focus: None,
            player_name: session.player_name.clone(),
            intelligence: session.intelligence,
            tech_competency: session.tech_competency,
            player_level: session.player_level,
            abilities_mask: session.abilities_mask,
        }
    }

    /// Logs a refused command at WARN with the player's identity and the
    /// `reason` the packet's message rules name. The caller sends nothing.
    fn reject(&self, cmd: &str, reason: &'static str) -> Vec<GameOutput> {
        let logged_cmd: String = cmd.chars().take(MAX_LOGGED_CMD).collect();
        tracing::warn!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            cmd = logged_cmd.as_str(),
            reason,
            "ConverseBasicHumanoid: command refused"
        );
        Vec::new()
    }

    /// `playermove` with its fields read. A missing or non-numeric field is
    /// refused before any state changes.
    fn player_move_command(&mut self, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        let Some(num) = number(params, "num") else {
            return self.reject("playermove", "bad_num");
        };
        let Some(suit) = number(params, "suit") else {
            return self.reject("playermove", "bad_suit");
        };
        let container = match params.get("container") {
            None => None,
            Some(value) => match value.as_f64() {
                Some(container) => Some(container),
                None => return self.reject("playermove", "bad_container"),
            },
        };
        self.player_move(num, suit, container)
    }
}

/// A numeric field of a client command, if it is present.
fn number(params: &HashMap<String, SfsValue>, key: &str) -> Option<f64> {
    params.get(key).and_then(SfsValue::as_f64)
}

impl MinigameInstance for ConverseHumanoidGame {
    fn started(&mut self) -> Vec<GameOutput> {
        self.deal();
        vec![GameOutput::Send(self.full_game_state())]
    }

    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        // After the end every command is ignored (the message rules).
        if self.outcome.is_some() {
            return self.reject(cmd, "game_over");
        }
        match cmd {
            "playermove" => self.player_move_command(params),
            "newhand" => self.new_hand(),
            "processreset" => self.process_reset(),
            "activatevictory" => match number(params, "victoryNum") {
                Some(victory) => self.activate_victory(victory),
                None => self.reject(cmd, "bad_victory_num"),
            },
            // A client `failure` is a defeat: the client can only hurt itself.
            "failure" => self.fail_message(),
            // A client `victory` is refused: the server decides victory (D-MG17).
            "victory" => self.reject(cmd, "client_victory_refused"),
            // ConverseBasicHumanoid has no item cheats (D-MG17).
            "activateitemcheat" => self.reject(cmd, "item_cheat_unavailable"),
            _ => self.reject(cmd, "unknown_command"),
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        self.tick_timers()
    }

    fn aborted(&mut self) -> Vec<GameOutput> {
        // The client has gone (D-MG17): end the game silently.
        self.outcome = Some(Outcome::Canceled);
        Vec::new()
    }

    fn needs_tick(&self) -> bool {
        self.outcome.is_none()
    }
}
