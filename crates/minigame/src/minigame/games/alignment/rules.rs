//! Alignment's rules after the cascade (MG-AL3a, spec sections 6 and 8): the
//! score (`doScore`), the server-decided victory, the countdown and its wall
//! clock, the idle hint prompt, and `callhint`. The client's score fields and
//! its clock are never read: every number here is held by the server.

use std::collections::HashMap;
use std::time::Instant;

use super::moves::{self, Cell};
use super::setup::{neighbour, number, number_list};
use super::{AlignmentGame, Phase};
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// One countdown step: the stored time drops by a quarter second per tick.
const TICK: f64 = 0.25;
/// The offset of the stored time: every outbound copy subtracts it.
const TIME_OFFSET: f64 = 0.25;
/// Seconds the wall clock allows past the countdown, for the client's latency.
/// `started` anchors the clock with this margin.
const CLOCK_GRACE: f64 = 2.0;
/// The seconds `callhint` costs.
const HINT_COST: f64 = 5.0;
/// The idle gap after a prompt, a score, or a hint timer reset.
const HINT_GAP: f64 = 10.0;
/// The first idle prompt comes this long after the start (`initGame`).
pub(super) const FIRST_HINT_GAP: f64 = 5.0;
/// The score slots (spec section 6): types 4, 5, 6 and 8. Types 1, 2, 3 and 7
/// have no slot and never score.
const SCORE_SLOTS: [(u8, usize); 4] = [(4, 0), (5, 1), (6, 2), (8, 3)];
/// `doScore`'s multiplier and time bonus, per `min(cascadeCount, 4)`.
const MULTIPLIER: [f64; 5] = [1.0, 1.5, 3.0, 5.0, 10.0];
const BONUS: [f64; 5] = [0.0, 2.0, 5.0, 8.0, 12.0];

/// A frame that carries only its command name.
fn command(name: &str) -> HashMap<String, SfsValue> {
    sfs_vars! { "_cmd" => SfsValue::String(name.to_string()) }
}

/// The scan order of `getHint` (spec section 8) as `(x descending, y
/// descending)`, for `hintIndex` 0-3.
fn scan_order(index: u8) -> (bool, bool) {
    match index {
        0 => (false, false),
        1 => (false, true),
        2 => (true, true),
        _ => (true, false),
    }
}

impl AlignmentGame {
    /// The time shown to the client: the stored value less the offset.
    pub(super) fn shown_time(&self) -> f64 {
        self.time_remaining - TIME_OFFSET
    }

    /// Changes the countdown and the wall clock by the same amount, so a bonus
    /// or a cost moves both (a hint costs 5 s, a score adds its bonus).
    pub(super) fn adjust_time(&mut self, delta: f64) {
        self.time_remaining += delta;
        self.clock_budget += delta;
    }

    /// Anchors the wall clock at the start: the countdown plus the grace.
    pub(super) fn start_clock(&mut self) {
        self.clock_start = Instant::now();
        self.clock_budget = self.shown_time() + CLOCK_GRACE;
    }

    /// Whether the server's wall clock has run out, whatever the ticks say.
    pub(super) fn clock_expired(&self) -> bool {
        self.clock_start.elapsed().as_secs_f64() >= self.clock_budget
    }

    /// The `timerupdate` frame: the shown time and the timer state.
    fn timer_update(&self) -> GameOutput {
        GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("timerupdate".to_string()),
            "timeRemaining" => number(self.shown_time()),
            "timerState" => number(self.timer_state),
        })
    }

    /// The time is up: the timer stops, the game ends and the client gets
    /// `failure` (spec section 6, Loss). Shared by `tick` and the wall clock.
    pub(super) fn time_out(&mut self) -> Vec<GameOutput> {
        self.timer_state = 0;
        self.outcome = Some("failure");
        self.phase = Phase::Ended;
        tracing::info!(
            entity_id = self.entity_id,
            entity_name = self.player_name.as_deref(),
            player_id = self.player_id,
            player_name = self.player_name.as_deref(),
            "Alignment: time is up"
        );
        vec![GameOutput::Send(command("failure")), GameOutput::Failure]
    }

    /// One countdown step (the 250 ms tick) and the Loss check; then the idle
    /// hint prompt. Runs in every phase, so a stuck board still ends.
    pub(super) fn run_tick(&mut self) -> Vec<GameOutput> {
        if self.clock_expired() {
            return self.time_out();
        }
        if self.timer_state == 1 {
            self.time_remaining -= TICK;
        }
        if self.timer_state >= 1 && self.shown_time() <= 0.0 {
            return self.time_out();
        }
        self.idle_prompt()
    }

    /// `clicktohint` when the idle gap has passed (spec section 8). It is
    /// silent while a swap is pending (`hint_pause`).
    fn idle_prompt(&mut self) -> Vec<GameOutput> {
        let shown = self.shown_time();
        let due = shown <= self.hint_last_time - self.hint_timer;
        if !due || shown <= 0.0 || self.timer_state != 1 || self.hint_pause {
            return vec![];
        }
        self.hint_last_time = shown;
        self.hint_timer = HINT_GAP;
        vec![GameOutput::Send(command("clicktohint"))]
    }

    /// `resethinttimer`: the client's hint gap starts again. No reply.
    pub(super) fn reset_hint_timer(&mut self) {
        self.hint_last_time = self.shown_time();
        self.hint_timer = HINT_GAP;
    }

    /// `checkForVictory`: sends `activate` the first time every goal is met.
    fn check_for_victory(&mut self) -> Vec<GameOutput> {
        if self.armed {
            return vec![];
        }
        let met = self
            .goal_scores
            .iter()
            .zip(&self.goal_totals)
            .all(|(score, total)| score >= total);
        if !met {
            return vec![];
        }
        self.armed = true;
        vec![GameOutput::Send(command("activate"))]
    }

    /// `doscore` (spec section 6, `doScore`): awaited once per cascade. It
    /// scores the server's own `currentScores` (power-ups never count), checks
    /// the victory, sends `updatescore` and the time bonus, and reopens the
    /// board. The client's fields are ignored. A second call finds the phase
    /// `Ready` and is refused.
    pub(super) fn do_score(&mut self) -> Vec<GameOutput> {
        if self.phase != Phase::AwaitScore {
            self.refuse("doscore", "not_awaiting_score");
            return vec![];
        }
        let step = (self.cascade_count as usize).min(MULTIPLIER.len() - 1);
        let mult = MULTIPLIER[step];
        for (kind, slot) in SCORE_SLOTS {
            let count = self
                .current_scores
                .iter()
                .filter(|piece| piece.power == 0 && piece.kind == kind)
                .count();
            let points = (count as f64 * mult).floor() as u32;
            if let Some(goal) = self.goal_scores.get_mut(slot) {
                *goal += points;
            }
        }
        let mut out = self.check_for_victory();
        out.push(GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("updatescore".to_string()),
            "goalScores" => number_list(&self.goal_scores),
            "goalTotals" => number_list(&self.goal_totals),
            "goalIcons" => number_list(&self.goal_icons),
        }));
        self.current_scores.clear();
        self.current_matches.clear();
        self.cascade_count = 0;
        self.adjust_time(BONUS[step]);
        out.push(self.timer_update());
        self.reset_hint_timer();
        self.hint_pause = false;
        self.phase = Phase::Ready;
        out
    }

    /// `dovictory` (spec section 6, Win): only after `activate` and in `Ready`.
    /// Sends `victory` and ends the game; any other call is refused.
    pub(super) fn do_victory(&mut self) -> Vec<GameOutput> {
        if !self.armed {
            self.refuse("dovictory", "victory_not_earned");
            return vec![];
        }
        if self.phase != Phase::Ready {
            self.refuse("dovictory", "not_ready");
            return vec![];
        }
        self.outcome = Some("victory");
        self.phase = Phase::Ended;
        vec![GameOutput::Send(command("victory")), GameOutput::Victory]
    }

    /// `callhint` (spec section 8, `getHint`): only in `Ready`. The scan uses
    /// the current `hintIndex`, then the index moves on (spec section 9 step 2:
    /// `hintIndex 0` finds `(1,1,4)` on transcript A's board). A found hint is
    /// sent as `hint`; the call costs 5 s either way, and the countdown is resent.
    pub(super) fn call_hint(&mut self) -> Vec<GameOutput> {
        if self.phase != Phase::Ready {
            self.refuse("callhint", "not_ready");
            return vec![];
        }
        let index = self.hint_index;
        self.hint_index = (self.hint_index + 1) % 4;
        let mut out = Vec::new();
        if let Some(cells) = self.find_hint(index) {
            let value = |(x, y): Cell| moves::cell_value(x, y, self.board[x][y]);
            out.push(GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("hint".to_string()),
                "hex1" => value(cells[0]),
                "hex2" => value(cells[1]),
                "hex3" => value(cells[2]),
            }));
        }
        self.adjust_time(-HINT_COST);
        out.push(self.timer_update());
        out
    }

    /// The first cell, in the scan order `index`, whose `(n0, n1)` triangle
    /// makes a match. Both neighbours must be on the board (the n1 bounds fix).
    pub(super) fn find_hint(&self, index: u8) -> Option<[Cell; 3]> {
        let (x_desc, y_desc) = scan_order(index);
        let mut columns: Vec<usize> = (0..self.board.len()).collect();
        if x_desc {
            columns.reverse();
        }
        for x in columns {
            let mut rows: Vec<usize> = (0..self.board[x].len()).collect();
            if y_desc {
                rows.reverse();
            }
            for y in rows {
                let (Some(n0), Some(n1)) = (
                    neighbour(&self.board, x, y, 0),
                    neighbour(&self.board, x, y, 1),
                ) else {
                    continue;
                };
                let cells = [(x, y), n0, n1];
                if moves::spin_check(&self.board, cells).is_some() {
                    return Some(cells);
                }
            }
        }
        None
    }
}
