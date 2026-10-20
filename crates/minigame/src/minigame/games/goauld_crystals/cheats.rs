//! GoauldCrystals ability cheats (spec sections 5 and 7, bits 0-14).
//!
//! The session's ability mask is the only source of cheats: the client's
//! `cheatNum` must be an integer 0-14 whose bit is still set in
//! `ability_bitfield`. The bit is cleared before its effect runs, and the new
//! mask goes back in `abilitycheatstate`. Item cheats are not ported (M8).

use std::collections::HashMap;

use rand::RngExt;

use super::moves::{
    read_coordinate, selected_update, square_state, trigger_sound, Pos, EMPTY, GREEN, RED, YELLOW,
};
use super::setup::goal_on;
use super::{Cell, GoauldCrystalsGame};
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Ability bits defined by the game, bits 0 to 14 (spec 7).
pub(super) const ABILITY_BITS: usize = 15;

/// Ticks in a pause (bit 2) or a reverse (bit 3): 5000 ms at 0.25 s (spec 8).
const PAUSE_TICKS: u32 = 20;

/// Sound each bit plays (spec 5 and 7), indexed by bit.
const SOUNDS: [&str; ABILITY_BITS] = [
    "crystalMG/time/chng",
    "crystalMG/time/xchng",
    "crystalMG/time/stop",
    "crystalMG/time/alter",
    "crystalMG/plyfld/addremSpc",
    "crystalMG/plyfld/addremCont",
    "crystalMG/plyfld/alter",
    "crystalMG/mvprts/addrem",
    "crystalMG/mvprts/alter",
    "crystalMG/mvprts/xchng",
    "crystalMG/tools/alter",
    "crystalMG/tools/apply",
    "crystalMG/obst/addrem",
    "crystalMG/obst/alter",
    "crystalMG/obst/xchng",
];

/// A board edge, as the row or column a cheat removes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl GoauldCrystalsGame {
    /// Spec 5 `activateabilitycheat`. A cheat whose number is not an integer
    /// 0-14, or whose bit is not held, is refused with no reply.
    pub(super) fn activate_ability_cheat(
        &mut self,
        params: &HashMap<String, SfsValue>,
    ) -> Vec<GameOutput> {
        let Some(bit) = read_coordinate(params, "cheatNum", ABILITY_BITS) else {
            self.warn_refused("activateabilitycheat", "bad_cheat_number");
            return vec![];
        };
        let mask = 1u32 << bit;
        if self.ability_bitfield & mask == 0 {
            self.warn_refused("activateabilitycheat", "ability_not_held");
            return vec![];
        }
        self.ability_bitfield &= !mask;
        let mut out = Vec::new();
        self.start_clock(&mut out);
        out.push(trigger_sound(SOUNDS[bit]));
        self.run_ability(bit, &mut out);
        out.push(GameOutput::Send(sfs_vars! {
            "_cmd" => SfsValue::String("abilitycheatstate".to_string()),
            "abilityBitfield" => SfsValue::Number(self.ability_bitfield as f64),
        }));
        out
    }

    /// Starts the clock if it is stopped, with one `timerupdate` (spec 7).
    fn start_clock(&mut self, out: &mut Vec<GameOutput>) {
        if self.timer_state == 0 {
            self.timer_state = 1;
            out.push(GameOutput::Send(self.build_timer_update()));
        }
    }

    /// The effect of one ability bit (spec 7 table). Bits that change the
    /// board end with the victory test; the rest only touch the clock or the
    /// rule counters.
    fn run_ability(&mut self, bit: usize, out: &mut Vec<GameOutput>) {
        match bit {
            0 => {
                self.time_remaining += 5.0;
                out.push(GameOutput::Send(self.build_timer_update()));
            }
            1 => {
                self.time_remaining = (self.time_remaining - 5.0).max(0.0);
                out.push(GameOutput::Send(self.build_timer_update()));
                self.add_random_greens(2, false);
                self.finish_board_change(out);
            }
            2 => self.set_timer_state(2, out),
            3 => self.set_timer_state(3, out),
            4 => {
                self.remove_first_free_edge([Edge::Bottom, Edge::Top, Edge::Left, Edge::Right], out)
            }
            5 => {
                self.remove_first_free_edge([Edge::Top, Edge::Bottom, Edge::Right, Edge::Left], out)
            }
            6 | 8 => self.yellows_without_orange += 5,
            7 => {
                self.add_random_greens(2, false);
                self.finish_board_change(out);
            }
            9 => match self.selected_red() {
                Some((x, y)) => {
                    self.board[x][y].color = YELLOW;
                    self.reds_to_yellows += 1;
                    self.deselect_into(out, (x, y));
                }
                None => self.reds_to_yellows += 2,
            },
            // The instrument is not ported, so there is nothing to restore.
            10 => {}
            11 => self.item_cheats_add_time = true,
            12 => match self.selected_red() {
                Some((x, y)) => {
                    self.board[x][y] = Cell::default();
                    self.deselect_into(out, (x, y));
                }
                None => self.reds_to_remove += 1,
            },
            13 => {
                self.turn_reds_green();
                self.finish_board_change(out);
            }
            14 => {
                self.time_remaining += 5.0;
                out.push(GameOutput::Send(self.build_timer_update()));
                match self.selected_red() {
                    Some((x, y)) => {
                        self.board[x][y] = Cell::default();
                        self.reds_to_remove += 1;
                        self.deselect_into(out, (x, y));
                    }
                    None => self.reds_to_remove += 2,
                }
            }
            _ => {}
        }
    }

    /// Bits 2 and 3: the clock pauses or runs backwards for `PAUSE_TICKS`
    /// ticks, then the tick resumes it with one `timerupdate`.
    fn set_timer_state(&mut self, state: u32, out: &mut Vec<GameOutput>) {
        self.timer_state = state;
        self.timer_ticks_left = PAUSE_TICKS;
        out.push(GameOutput::Send(self.build_timer_update()));
    }

    /// The selected cell, when one is selected and it is red.
    fn selected_red(&self) -> Option<Pos> {
        self.selected
            .filter(|&(x, y)| self.board[x][y].color == RED)
    }

    /// Clears the selection after a cheat changed the selected cell at `pos`:
    /// one `squarestate` for the cell and `selectedupdate {-1, -1}`.
    fn deselect_into(&mut self, out: &mut Vec<GameOutput>, pos: Pos) {
        out.push(square_state(pos, &self.board[pos.0][pos.1]));
        self.selected = None;
        out.push(selected_update(None));
    }

    /// Bit 13: `ceil(reds / 3)` random reds turn green, with their positions
    /// recorded in `reds_to_revert` so the next move that does not win turns
    /// them back. Their `var` is kept.
    fn turn_reds_green(&mut self) {
        let mut reds = self.cells_where(|cell| cell.color == RED);
        let count = reds.len().div_ceil(3);
        let mut tracked = Vec::with_capacity(count);
        for _ in 0..count {
            let pick = self.rng.random_range(0..reds.len());
            let (x, y) = reds.swap_remove(pick);
            self.board[x][y].color = GREEN;
            tracked.push((x, y));
        }
        self.reds_to_revert = Some(tracked);
    }

    /// Bits 4 and 5: removes the first edge in `order` with no goal on it.
    /// When no edge qualifies, nothing happens and the bit stays spent.
    fn remove_first_free_edge(&mut self, order: [Edge; 4], out: &mut Vec<GameOutput>) {
        if let Some(edge) = order.into_iter().find(|&edge| self.edge_is_free(edge)) {
            self.remove_edge(edge);
            self.finish_board_change(out);
        }
    }

    /// True when neither goal is on `edge` (spec 7: for the bottom row,
    /// `h-1 > startGoalY` and `h-1 > finishGoalY`).
    fn edge_is_free(&self, edge: Edge) -> bool {
        let (w, h) = (self.w, self.h);
        [self.start, self.finish].iter().all(|goal| match edge {
            Edge::Top => goal.y != 0,
            Edge::Bottom => goal.y != h - 1,
            Edge::Left => goal.x != 0,
            Edge::Right => goal.x != w - 1,
        })
    }

    /// Removes one free edge: the cells, the goal offsets and coordinates,
    /// the selection and the tracked reds all move with it. The greens that
    /// were on the edge are re-added on random empty cells.
    fn remove_edge(&mut self, edge: Edge) {
        let vertical = matches!(edge, Edge::Top | Edge::Bottom);
        let (index, removed) = if vertical {
            let y = if edge == Edge::Top { 0 } else { self.h - 1 };
            let mut greens = 0;
            for column in &mut self.board {
                greens += usize::from(column[y].color == GREEN);
                column.remove(y);
            }
            self.h -= 1;
            (y, greens)
        } else {
            let x = if edge == Edge::Left { 0 } else { self.w - 1 };
            let greens = self.board[x].iter().filter(|c| c.color == GREEN).count();
            self.board.remove(x);
            self.w -= 1;
            (x, greens)
        };
        self.shift_goals(vertical, index);
        self.selected = self
            .selected
            .and_then(|pos| shift_pos(pos, vertical, index));
        if let Some(tracked) = self.reds_to_revert.take() {
            let kept = tracked
                .into_iter()
                .filter_map(|pos| shift_pos(pos, vertical, index))
                .collect();
            self.reds_to_revert = Some(kept);
        }
        self.add_random_greens(removed, true);
    }

    /// Moves both goals after a row or column at `index` was removed. A goal
    /// whose offset runs along the removed line shifts its offset; its
    /// coordinates are then rebuilt from the side and offset.
    fn shift_goals(&mut self, vertical: bool, index: usize) {
        let (w, h) = (self.w, self.h);
        for goal in [&mut self.start, &mut self.finish] {
            // Sides 1 and 3 run along y; sides 0 and 2 run along x.
            let offset_runs_along_y = goal.side == 1 || goal.side == 3;
            if offset_runs_along_y == vertical && goal.offset > index {
                goal.offset -= 1;
            }
            *goal = goal_on(goal.side, goal.offset, w, h);
        }
    }

    /// Spec 7 `addRandomGreenCrystals`: `n` greens on random empty cells,
    /// without repeats. When the empties run out and `overwrite` is set, random
    /// non-empty cells are overwritten; otherwise the rest are dropped. The
    /// caller runs the victory test.
    pub(super) fn add_random_greens(&mut self, n: usize, overwrite: bool) {
        let mut empties = self.cells_where(|cell| cell.color == EMPTY);
        let mut filled = if overwrite {
            self.cells_where(|cell| cell.color != EMPTY)
        } else {
            Vec::new()
        };
        for _ in 0..n {
            let pos = if !empties.is_empty() {
                let pick = self.rng.random_range(0..empties.len());
                empties.swap_remove(pick)
            } else if !filled.is_empty() {
                let pick = self.rng.random_range(0..filled.len());
                filled.swap_remove(pick)
            } else {
                break;
            };
            self.board[pos.0][pos.1] = Cell {
                color: GREEN,
                var: self.rng.random_range(0..3u8),
                power: false,
            };
        }
    }

    /// Every cell position whose cell matches `keep`, in column order.
    fn cells_where(&self, keep: impl Fn(&Cell) -> bool) -> Vec<Pos> {
        let mut found = Vec::new();
        for (x, column) in self.board.iter().enumerate() {
            for (y, cell) in column.iter().enumerate() {
                if keep(cell) {
                    found.push((x, y));
                }
            }
        }
        found
    }

    /// After a cheat changed the board: the victory test (which may end the
    /// game), then `fullgamestate` if the game goes on (spec 7).
    fn finish_board_change(&mut self, out: &mut Vec<GameOutput>) {
        out.extend(self.after_board_change());
        if self.outcome.is_none() {
            out.push(GameOutput::Send(self.build_full_game_state()));
        }
    }
}

/// Maps a position across the removal of the row or column at `index`.
/// `None` when the position is on the removed line.
fn shift_pos(pos: Pos, vertical: bool, index: usize) -> Option<Pos> {
    let coord = if vertical { pos.1 } else { pos.0 };
    if coord == index {
        return None;
    }
    let coord = if coord > index { coord - 1 } else { coord };
    Some(if vertical {
        (pos.0, coord)
    } else {
        (coord, pos.1)
    })
}
