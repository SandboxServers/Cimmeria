//! Alignment ability cheats (MG-AL3b, spec sections 6 and 7, bits 0-14). The
//! session's ability mask is the only source of cheats: `cheatNum` must be an
//! integer 0-14 whose bit is still held in `abilities_mask`. The bit is spent
//! before its effect runs, so each bit works once. Item cheats are not ported.

use std::collections::HashMap;

use rand::RngExt;

use super::cascade;
use super::moves::{self, Cell};
use super::piece::Piece;
use super::setup::number;
use super::{AlignmentGame, Phase};
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Ability bits defined by the game, bits 0 to 14.
const ABILITY_BITS: u32 = 15;
/// Seconds of a grant: the ability add-time (bit 11) and each power-up (bit 13).
const GRANT: f64 = 2.0;
/// Seconds bit 0 adds to the countdown.
const BONUS_BIT0: f64 = 5.0;
/// Seconds bit 1 and bit 9 take from the countdown.
const PENALTY: f64 = 10.0;
/// Seconds each matching swap adds under bits 3 and 8.
const MATCH_BONUS: f64 = 0.5;
/// The idle gap after bit 1 or 9, in seconds (`hintTimer`).
const CHEAT_HINT_GAP: f64 = 20.0;
/// Power-up conversions made by bit 1 or 9.
const CONVERSIONS: usize = 3;

/// The ability bit `cheatNum` names: an integer 0-14, or `None`.
fn cheat_number(params: &HashMap<String, SfsValue>) -> Option<u32> {
    match params.get("cheatNum") {
        Some(SfsValue::Number(value))
            if value.fract() == 0.0 && (0.0..f64::from(ABILITY_BITS)).contains(value) =>
        {
            Some(*value as u32)
        }
        _ => None,
    }
}

impl AlignmentGame {
    /// `activateabilitycheat` (spec section 7). A cheat whose number is not an
    /// integer 0-14, whose bit is not held, or that arrives outside `Ready` is
    /// refused with no reply and no change. The reply is `abilitycheatstate`.
    pub(super) fn activate_ability_cheat(
        &mut self,
        params: &HashMap<String, SfsValue>,
    ) -> Vec<GameOutput> {
        let Some(bit) = cheat_number(params) else {
            self.refuse("activateabilitycheat", "bad_cheat_number");
            return vec![];
        };
        if self.phase != Phase::Ready {
            self.refuse("activateabilitycheat", "not_ready");
            return vec![];
        }
        let mask = 1u32 << bit;
        if self.abilities_mask & mask == 0 {
            self.refuse("activateabilitycheat", "ability_not_held");
            return vec![];
        }
        self.abilities_mask &= !mask;
        // The add-time bonus depends on the state when the cheat started.
        let add_time = self.ability_add_time;
        let mut out = self.run_ability(bit);
        out.push(GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("abilitycheatstate".to_string()),
            "abilityBitfield" => number(self.abilities_mask),
        }));
        if add_time {
            self.adjust_time(GRANT);
            out.push(self.timer_update());
        }
        out
    }

    /// The effect of one ability bit (spec section 7 table). Bit 10 has nothing
    /// to restore: `instcc` is -1 for good, so the bit is spent and that is all.
    fn run_ability(&mut self, bit: u32) -> Vec<GameOutput> {
        match bit {
            0 => {
                self.adjust_time(BONUS_BIT0);
                vec![self.timer_update()]
            }
            1 | 9 => {
                self.adjust_time(-PENALTY);
                let mut out = vec![self.timer_update()];
                out.extend(self.convert_cells());
                self.hint_last_time = self.shown_time();
                self.hint_timer = CHEAT_HINT_GAP;
                out
            }
            2 => {
                self.stop_timer_match = true;
                vec![]
            }
            3 => {
                self.add_timer_match = true;
                vec![]
            }
            8 => {
                self.add_timer_match_move = true;
                vec![]
            }
            11 => {
                self.ability_add_time = true;
                vec![]
            }
            13 => {
                self.power_up_add_time = true;
                vec![]
            }
            4 | 6 | 7 | 12 => self.clear_least_common(),
            5 | 14 => self.clear_power_ups(),
            _ => vec![],
        }
    }

    /// Bits 1 and 9: a power-up on a random plain cell of type 1-8, three times.
    /// Each cell is taken at most once, and the power is `getMatchPower(t, 8)`.
    fn convert_cells(&mut self) -> Vec<GameOutput> {
        let mut candidates =
            self.cells_where(|piece| (1..=8).contains(&piece.kind) && piece.power == 0);
        let mut out = Vec::new();
        for _ in 0..CONVERSIONS {
            if candidates.is_empty() {
                break;
            }
            let index = self.rng.random_range(0..candidates.len());
            let (x, y) = candidates.swap_remove(index);
            let Some(power) = moves::get_match_power(self.board[x][y].kind, 8) else {
                continue;
            };
            self.board[x][y].power = power;
            out.push(GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("powerupconvert".to_string()),
                "x" => number(x as f64),
                "y" => number(y as f64),
                "powerUp" => number(f64::from(power) / 10.0),
            }));
        }
        out
    }

    /// Bits 4, 6, 7 and 12 (item cheat 2): clears every cell of the least common
    /// type 1-8. Ties go to the lower type.
    fn clear_least_common(&mut self) -> Vec<GameOutput> {
        let mut counts = [0usize; 10];
        for piece in self.board.iter().flatten() {
            counts[usize::from(piece.kind)] += 1;
        }
        let mut least: Option<(u8, usize)> = None;
        for kind in 1..=8u8 {
            let count = counts[usize::from(kind)];
            if count > 0 && least.is_none_or(|(_, best)| count < best) {
                least = Some((kind, count));
            }
        }
        let flood = match least {
            Some((kind, _)) => self.cells_where(|piece| piece.kind == kind),
            None => Vec::new(),
        };
        self.clear_flood(&flood)
    }

    /// Bits 5 and 14: clears every power-up cell.
    fn clear_power_ups(&mut self) -> Vec<GameOutput> {
        let flood = self.cells_where(|piece| piece.power != 0);
        self.clear_flood(&flood)
    }

    /// Expands `flood` as `checkPowerUpMatches` does, zeroes the matched cells
    /// and sends `bypasshexspinner`. The cascade that follows is armed with
    /// `cheat_cleared`, so these types never reach `currentScores`. An empty
    /// flood clears nothing: no reply, and no cascade to wait for.
    fn clear_flood(&mut self, flood: &[Cell]) -> Vec<GameOutput> {
        if flood.is_empty() {
            return Vec::new();
        }
        let (set, values) = moves::check_power_up_matches(&self.board, flood);
        let matches: Vec<(Cell, Piece)> = set
            .into_iter()
            .map(|(x, y)| ((x, y), self.board[x][y]))
            .collect();
        for &((x, y), _) in &matches {
            self.board[x][y] = Piece::plain(0);
        }
        let mut out = self.power_up_time(values.len());
        out.push(GameOutput::Send(cascade::bypasshexspinner(&matches)));
        self.current_matches = matches;
        self.cheat_cleared = true;
        self.phase = Phase::AwaitCascade;
        out
    }

    /// The cells of the board that `keep` accepts, as `(x, y)`.
    fn cells_where(&self, keep: impl Fn(&Piece) -> bool) -> Vec<Cell> {
        let mut cells = Vec::new();
        for (x, column) in self.board.iter().enumerate() {
            for (y, piece) in column.iter().enumerate() {
                if keep(piece) {
                    cells.push((x, y));
                }
            }
        }
        cells
    }

    /// `powerUpAddTime` (bit 13): +2 s and one `timerupdate` per power-up
    /// matched. Nothing happens until bit 13 is spent.
    pub(super) fn power_up_time(&mut self, count: usize) -> Vec<GameOutput> {
        if !self.power_up_add_time {
            return vec![];
        }
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            self.adjust_time(GRANT);
            out.push(self.timer_update());
        }
        out
    }

    /// `checkForMatchCheat` (spec section 7): bits 2, 3 and 8 after a matching
    /// swap, each only while the shown time is above 0. One `timerupdate` if
    /// any of them fired.
    pub(super) fn match_cheats(&mut self) -> Vec<GameOutput> {
        let mut fired = false;
        if self.stop_timer_match && self.shown_time() > 0.0 {
            self.timer_state = 0;
            fired = true;
        }
        if self.add_timer_match && self.shown_time() > 0.0 {
            self.adjust_time(MATCH_BONUS);
            fired = true;
        }
        if self.add_timer_match_move && self.shown_time() > 0.0 {
            self.adjust_time(MATCH_BONUS);
            fired = true;
        }
        if fired {
            vec![self.timer_update()]
        } else {
            vec![]
        }
    }
}
