//! Alignment: the port lands in MG-AL1..3 (docs/analysis/minigames/spec-alignment.md).
//!
//! MG-AL1 is the setup and the state shell; MG-AL2a adds the server-validated
//! swap and matching (`moves.rs`); MG-AL2b adds the cascade, the gravity and
//! the refill (`cascade.rs`, `moves_gravity.rs`). `create()` still hands the
//! name to the placeholder: `PORTED` stays `false` until MG-AL3, so nothing
//! here is reachable from content yet. The shell is exercised by the tests only.
#![allow(dead_code)] // unreachable until MG-AL3 flips PORTED; the tests build it

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

mod cascade;
mod moves;
mod moves_gravity;
mod piece;
mod setup;
#[cfg(test)]
mod tests_cascade;
#[cfg(test)]
mod tests_moves;
#[cfg(test)]
mod tests_refill;
#[cfg(test)]
mod tests_setup;

use piece::Piece;

/// Flipped to `true` by MG-AL3 only, once victory is server-decided.
pub(crate) const PORTED: bool = false;

/// The most characters of a client string a log line keeps.
const LOG_CAP: usize = 64;

/// The first `LOG_CAP` characters of a client string, for a log line.
fn log_text(text: &str) -> &str {
    match text.char_indices().nth(LOG_CAP) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

/// Where the game is in the port's phase machine (spec section 5). `Ready`
/// takes a swap; a match moves to `AwaitCascade`, `cascade` moves to
/// `AwaitCheck`, and `checkForCascade` either loops back to `AwaitCascade` or
/// moves to `AwaitScore` (`doscore` is MG-AL3a's). `Ended` is set by
/// `aborted()` and closes the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Ready,
    AwaitCascade,
    AwaitCheck,
    AwaitScore,
    Ended,
}

/// Alignment game state for one session.
pub(crate) struct AlignmentGame {
    entity_id: u32,
    player_id: i32,
    player_name: Option<String>,
    tech_competency: u32,
    abilities_mask: u32,
    /// Tier 1-4, clamped from the session's difficulty.
    tier: u8,
    /// Goal level 0-10, from `techcomp`.
    level: u8,
    rng: StdRng,
    /// Pieces indexed `[x][y]`.
    board: Vec<Vec<Piece>>,
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
    /// The cells of the current match, power-up-expanded (spec section 6).
    current_matches: Vec<(moves::Cell, Piece)>,
    /// The pieces scored so far in this turn (`currentScores`).
    current_scores: Vec<Piece>,
    /// Refused commands so far in this game; only some are logged (F4).
    refusals: u32,
    /// Cascade steps taken after the swap; the re-roll pass runs when it is
    /// above 0.
    cascade_count: u32,
    /// The gravity direction index, 0-5 (`dropIndex`).
    drop_index: u8,
}

impl AlignmentGame {
    /// Builds the game for a session and draws its board. The RNG is seeded from
    /// the OS, not from the session seed: a 32-bit seed would let a client solve
    /// the board by brute force (authority F2).
    pub(crate) fn new(session: &MinigameSession) -> Self {
        let tier = setup::clamp_tier(
            session.difficulty,
            session.entity_id,
            session.player_name.as_deref(),
        );
        let mut rng = StdRng::from_rng(&mut rand::rng());
        let board = setup::select_icons(&mut rng, tier);
        Self::build(session, rng, board)
    }

    /// Test-only: builds the game with an injected board of plain piece ids, so
    /// tests can predict every move (spec section 9).
    #[cfg(test)]
    pub(crate) fn with_board(session: &MinigameSession, board: Vec<Vec<u8>>) -> Self {
        let pieces = board
            .iter()
            .map(|column| column.iter().map(|&kind| Piece::plain(kind)).collect())
            .collect();
        Self::with_pieces(session, pieces)
    }

    /// Test-only: builds the game with an injected board that may hold power-ups.
    #[cfg(test)]
    pub(crate) fn with_pieces(session: &MinigameSession, board: Vec<Vec<Piece>>) -> Self {
        let rng = StdRng::seed_from_u64(u64::from(session.seed));
        Self::build(session, rng, board)
    }

    fn build(session: &MinigameSession, rng: StdRng, board: Vec<Vec<Piece>>) -> Self {
        let tier = setup::clamp_tier(
            session.difficulty,
            session.entity_id,
            session.player_name.as_deref(),
        );
        let level = setup::level_for(
            session.tech_competency,
            session.entity_id,
            session.player_name.as_deref(),
        );
        let spec = setup::tier_spec(tier);
        let goal_totals = setup::goal_totals(tier, level);
        Self {
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            tech_competency: session.tech_competency,
            abilities_mask: session.abilities_mask,
            tier,
            level,
            rng,
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
            current_matches: Vec::new(),
            current_scores: Vec::new(),
            refusals: 0,
            cascade_count: 0,
            drop_index: 0,
        }
    }

    /// `swappieces` (spec section 5): validates the triangle (D-MG2), then
    /// replies `feedback` and `hexspinner` on a match, or a `hexspinner` with
    /// `spinCount 3` and no cells when no rotation matches.
    fn swap(&mut self, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        if self.phase != Phase::Ready {
            self.refuse("swappieces", "not_ready");
            return vec![];
        }
        let checked = moves::read_cells(params)
            .and_then(|cells| moves::check_swap(&self.board, cells, params).map(|()| cells));
        let cells = match checked {
            Ok(cells) => cells,
            Err(reject) => {
                self.refuse("swappieces", reject.reason());
                return vec![];
            }
        };
        match moves::swap_pieces(&self.board, cells) {
            Some((board, outcome)) => {
                self.board = board;
                self.phase = Phase::AwaitCascade;
                self.current_matches = outcome.flood.clone();
                self.current_scores.clear();
                self.cascade_count = 0;
                vec![
                    GameOutput::Send(moves::feedback(&outcome)),
                    GameOutput::Send(moves::hexspinner(
                        params,
                        &outcome.matches,
                        outcome.spin_count,
                    )),
                ]
            }
            None => vec![GameOutput::Send(moves::hexspinner(params, &[], 3))],
        }
    }

    /// `cascade` (spec section 6, `doCascade`): the gravity slide and the refill
    /// of drop direction `dropIndex`, then the re-roll pass after a scored step.
    /// Replies `hexmove` and waits for `checkForCascade`.
    fn cascade(&mut self) -> Vec<GameOutput> {
        if self.phase != Phase::AwaitCascade {
            self.refuse("cascade", "not_ready");
            return vec![];
        }
        let d = self.drop_index;
        let slides = moves_gravity::gravity(&mut self.board, d);
        let mut new_hexes = cascade::refill(
            &mut self.rng,
            &mut self.board,
            &self.current_matches,
            d,
            self.tier,
        );
        if self.cascade_count > 0 {
            let goal_icons = setup::tier_spec(self.tier).goal_icons;
            cascade::reroll(
                &mut self.rng,
                &mut self.board,
                &mut new_hexes,
                self.cascade_count,
                goal_icons,
            );
        }
        self.phase = Phase::AwaitCheck;
        vec![GameOutput::Send(cascade::hexmove(d, &slides, &new_hexes))]
    }

    /// `checkForCascade` (spec section 6): a board with a match removes it with
    /// `bypasshexspinner` and waits for the next `cascade`. Without a match the
    /// drop index moves on and `cascadecheckcomplete` ends the step; the score
    /// is awaited next.
    fn check_for_cascade(&mut self) -> Vec<GameOutput> {
        if self.phase != Phase::AwaitCheck {
            self.refuse("checkForCascade", "not_ready");
            return vec![];
        }
        let mut scores = self.current_scores.clone();
        scores.extend(cascade::types(&self.current_matches));
        let seeds = setup::check_for_matches(&self.board);
        self.current_scores = scores;
        if seeds.is_empty() {
            let turned = !self.current_scores.is_empty();
            self.drop_index = match self.drop_index {
                5 => 0,
                index if turned => index + 1,
                index => index,
            };
            self.phase = Phase::AwaitScore;
            return vec![GameOutput::Send(cascade::cascadecheckcomplete())];
        }
        let matches = cascade::expanded_matches(&self.board, &seeds);
        for &((x, y), _) in &matches {
            self.board[x][y] = Piece::plain(0);
        }
        self.current_matches = matches;
        self.cascade_count += 1;
        self.phase = Phase::AwaitCascade;
        vec![GameOutput::Send(cascade::bypasshexspinner(
            &self.current_matches,
        ))]
    }

    /// Counts a refused client command and logs it at WARN when `logs_refusal`
    /// says so (the board is left as it was).
    fn refuse(&mut self, cmd: &str, reason: &str) {
        self.refusals = self.refusals.saturating_add(1);
        if !logs_refusal(self.refusals) {
            return;
        }
        tracing::warn!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            cmd,
            reason,
            refusals = self.refusals,
            "Alignment: command refused"
        );
    }
}

/// The first refusals of a game are logged; after that one in every
/// `REFUSAL_LOG_EVERY` is, so a client cannot flood the log (authority F4).
const REFUSAL_LOG_FIRST: u32 = 16;
const REFUSAL_LOG_EVERY: u32 = 256;

/// Whether the refusal numbered `n` (counted from 1) is logged.
fn logs_refusal(n: u32) -> bool {
    n <= REFUSAL_LOG_FIRST || n.is_multiple_of(REFUSAL_LOG_EVERY)
}

impl MinigameInstance for AlignmentGame {
    fn started(&mut self) -> Vec<GameOutput> {
        tracing::info!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            tier = self.tier,
            level = self.level,
            "Alignment started"
        );
        vec![GameOutput::Send(self.full_game_state())]
    }

    fn message(&mut self, cmd: &str, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        // The command is client text: it is cut before it reaches a log line.
        let logged = log_text(cmd);
        if self.phase == Phase::Ended {
            self.refuse(logged, "game_ended");
            return vec![];
        }
        match cmd {
            // The reset is refused, as the CrystalGame ruling does: no reply
            // and no change, so a reset cannot abort a cascade or add time.
            "processreset" => {
                self.refuse("processreset", "reset_refused");
                vec![]
            }
            "swappieces" => self.swap(params),
            "cascade" => self.cascade(),
            "checkForCascade" => self.check_for_cascade(),
            // The hint timer (`resethinttimer`) only feeds the hint, which is
            // MG-AL3's; the client expects no reply.
            "resethinttimer" => vec![],
            _ => {
                // Every other command belongs to a later slice (AL2, AL3). A
                // client `victory` lands here too: the server decides victory.
                let reason = if cmd == "victory" {
                    "client_victory_refused"
                } else {
                    "unknown_command"
                };
                self.refuse(logged, reason);
                vec![]
            }
        }
    }

    fn tick(&mut self) -> Vec<GameOutput> {
        vec![]
    }

    /// The client socket is already gone when an abort arrives, so the game
    /// ends silently (D-MG17): no frame is sent.
    fn aborted(&mut self) -> Vec<GameOutput> {
        if self.phase == Phase::Ended {
            return vec![];
        }
        self.phase = Phase::Ended;
        self.outcome = Some("canceled");
        vec![]
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
