//! CrystalGame client moves: `processmove` (a click, validated on the
//! server) and `processreset` (refused, see [`CrystalGame::process_reset`]).
//!
//! The first valid click starts the clock with `timerupdate` and then sends
//! one `updateslot` per changed cell, in the order `Board::press` gives
//! (docs/analysis/minigames/spec-crystalgame.md, sections 5 and 8). A refused
//! command logs a WARN and sends nothing.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::setup::{HEIGHT, TIME_LIMIT, WIDTH};
use super::CrystalGame;
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Reads `key` as a whole number in `0..limit`. A missing, fractional,
/// negative or out-of-range value gives `None`.
fn bounded(params: &HashMap<String, SfsValue>, key: &str, limit: usize) -> Option<u8> {
    match params.get(key) {
        Some(SfsValue::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n < limit as f64 => {
            Some(*n as u8)
        }
        _ => None,
    }
}

/// `timerupdate`: the clock's value and state.
fn timerupdate(time_remaining: u32, timer_state: u32) -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("timerupdate".to_string()),
        "timeRemaining" => SfsValue::Number(f64::from(time_remaining)),
        "timerState" => SfsValue::Number(f64::from(timer_state)),
    }
}

/// `updateslot`: one cell's new colour.
fn updateslot(x: u8, y: u8, num: u8) -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("updateslot".to_string()),
        "x" => SfsValue::Number(f64::from(x)),
        "y" => SfsValue::Number(f64::from(y)),
        "num" => SfsValue::Number(f64::from(num)),
    }
}

impl CrystalGame {
    /// `processmove {x, y}`: one click by the player. Refused after the end
    /// or for a coordinate that is not an integer in range. A valid click
    /// starts the clock on the first press only.
    pub(super) fn process_move(&mut self, params: &HashMap<String, SfsValue>) -> Vec<GameOutput> {
        if self.outcome.is_some() {
            self.reject("processmove", "game_over");
            return vec![];
        }
        let (Some(x), Some(y)) = (bounded(params, "x", WIDTH), bounded(params, "y", HEIGHT)) else {
            self.reject("processmove", "bad_coordinates");
            return vec![];
        };
        let mut out = Vec::new();
        if self.timer_state == 0 {
            self.timer_state = 1;
            self.clock_deadline = Some(Instant::now() + Duration::from_secs(u64::from(TIME_LIMIT)));
            out.push(GameOutput::Send(timerupdate(TIME_LIMIT, 1)));
        }
        out.extend(
            self.board
                .press(x, y)
                .into_iter()
                .map(|(x, y, num)| GameOutput::Send(updateslot(x, y, num))),
        );
        out
    }

    /// `processreset`: refused with no reply, so the board and the clock stay
    /// as they are. The original ignores it (spec-crystalgame.md, CG2 row and
    /// open question 11.3). A reset would let a player idle a running clock or
    /// re-roll the board without limit.
    pub(super) fn process_reset(&mut self) -> Vec<GameOutput> {
        self.reject("processreset", "reset_refused");
        vec![]
    }
}
