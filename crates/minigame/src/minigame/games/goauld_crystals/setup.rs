//! GoauldCrystals setup: the difficulty table, goal placement, board
//! generation and the `fullgamestate` / `timerupdate` builders.
//!
//! Spec sections 3 to 5 of `docs/analysis/minigames/spec-goauldcrystals.md`.
//! Every random draw comes from `self.rng`, so a transcript is reproducible.

use std::collections::HashMap;

use rand::RngExt;

use super::power::{finish_powered, update_power};
use super::{Cell, Goal, GoauldCrystalsGame};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Highest difficulty-table row; higher tech competency is clamped to it.
pub(super) const MAX_LEVEL: usize = 10;

/// Draws allowed for the finish goal before the last draw is kept.
const FINISH_DRAWS: u32 = 100;

/// One difficulty-table entry: board size, time in seconds, obstacle count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Tier {
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) time: u32,
    pub(super) obstacles: u32,
}

const fn tier(w: usize, h: usize, time: u32, obstacles: u32) -> Tier {
    Tier {
        w,
        h,
        time,
        obstacles,
    }
}

/// Final values, indexed `[L][difficulty - 1]` (spec section 4).
pub(super) const TABLE: [[Tier; 4]; MAX_LEVEL + 1] = [
    [
        tier(4, 4, 24, 2),
        tier(4, 4, 16, 2),
        tier(5, 5, 16, 3),
        tier(7, 5, 16, 5),
    ],
    [
        tier(5, 5, 16, 3),
        tier(5, 5, 16, 3),
        tier(7, 5, 16, 4),
        tier(7, 7, 16, 6),
    ],
    [
        tier(7, 5, 24, 4),
        tier(7, 5, 20, 4),
        tier(7, 7, 18, 6),
        tier(8, 7, 20, 8),
    ],
    [
        tier(7, 7, 28, 5),
        tier(7, 7, 24, 5),
        tier(8, 7, 20, 7),
        tier(8, 8, 24, 9),
    ],
    [
        tier(8, 7, 32, 6),
        tier(8, 7, 25, 6),
        tier(8, 8, 22, 8),
        tier(9, 8, 25, 11),
    ],
    [
        tier(8, 8, 36, 7),
        tier(8, 8, 28, 7),
        tier(9, 8, 24, 10),
        tier(9, 9, 28, 12),
    ],
    [
        tier(9, 8, 40, 8),
        tier(9, 8, 29, 8),
        tier(11, 8, 25, 11),
        tier(11, 9, 29, 14),
    ],
    [
        tier(11, 8, 44, 9),
        tier(11, 8, 32, 9),
        tier(9, 9, 27, 12),
        tier(11, 11, 32, 16),
    ],
    [
        tier(9, 9, 48, 10),
        tier(9, 9, 33, 10),
        tier(11, 9, 28, 14),
        tier(12, 11, 33, 18),
    ],
    [
        tier(11, 9, 48, 11),
        tier(11, 9, 36, 11),
        tier(11, 9, 30, 15),
        tier(12, 12, 36, 20),
    ],
    [
        tier(11, 11, 48, 12),
        tier(11, 11, 40, 12),
        tier(12, 11, 32, 16),
        tier(14, 12, 40, 22),
    ],
];

/// Table row for a tech competency: `floor((techcomp - 1) / 5)`, clamped to
/// 0-10. Competency above the table logs an INFO line and uses row 10.
pub(super) fn clamp_level(tech_competency: u32) -> usize {
    let level = (tech_competency.saturating_sub(1) / 5) as usize;
    if level > MAX_LEVEL {
        tracing::info!(
            tech_competency,
            level,
            "GoauldCrystals: tech competency above the table, clamped to row 10"
        );
        MAX_LEVEL
    } else {
        level
    }
}

/// Difficulty column, clamped to 1-4. A value outside that range logs an
/// INFO line; the table has no column for it.
pub(super) fn clamp_difficulty(difficulty: u32) -> u32 {
    let clamped = difficulty.clamp(1, 4);
    if clamped != difficulty {
        tracing::info!(
            difficulty,
            clamped,
            "GoauldCrystals: difficulty outside 1-4, clamped"
        );
    }
    clamped
}

/// Goal on `side` at `offset`, placed on the edge of a `w` x `h` board.
pub(super) fn goal_on(side: u8, offset: usize, w: usize, h: usize) -> Goal {
    let (x, y) = match side {
        0 => (offset, 0),
        1 => (w - 1, offset),
        2 => (offset, h - 1),
        _ => (0, offset),
    };
    Goal { side, offset, x, y }
}

/// Squared distance between two goals, in whole cells.
fn distance_squared(a: Goal, b: Goal) -> usize {
    a.x.abs_diff(b.x).pow(2) + a.y.abs_diff(b.y).pow(2)
}

/// The seed-1 4x4 tutorial board indexed `[x][y]`; the rows below are `y`.
fn tutorial_board() -> Vec<Vec<Cell>> {
    const ROWS: [[u8; 4]; 4] = [[1, 4, 2, 2], [1, 1, 1, 1], [0, 0, 0, 4], [0, 0, 0, 1]];
    (0..4)
        .map(|x| {
            (0..4)
                .map(|y| Cell {
                    color: ROWS[y][x],
                    var: 0,
                    power: false,
                })
                .collect()
        })
        .collect()
}

/// Wraps a count, coordinate or flag as an SFS value.
fn number(value: f64) -> SfsValue {
    SfsValue::Number(value)
}

/// One cell as the client reads it: `{color, var, power}`.
fn cell_value(cell: &Cell) -> SfsValue {
    SfsValue::Object(sfs_vars! {
        "color" => number(cell.color as f64),
        "var" => number(cell.var as f64),
        "power" => SfsValue::Bool(cell.power),
    })
}

impl GoauldCrystalsGame {
    /// Starts a game: a new board and goals, the clock stopped at the table
    /// time, and the cheat inventory back to the session value.
    pub(super) fn init_game(&mut self) {
        let tier = TABLE[self.level][self.difficulty as usize - 1];
        self.w = tier.w;
        self.h = tier.h;
        self.max_time = tier.time as f64;
        self.time_remaining = self.max_time;
        self.timer_state = 0;
        self.selected = None;
        self.ability_bitfield = self.abilities_mask;
        self.yellows_without_orange = 0;
        self.reds_to_yellows = 0;
        self.reds_to_remove = 0;
        self.reds_to_revert = None;
        self.item_cheats_add_time = false;
        self.outcome = None;
        self.populate_game_board(tier.obstacles);
    }

    /// Spec 3 `populateGameBoard`: the goals once, then boards until the
    /// finish is not powered, so a game never starts already won.
    fn populate_game_board(&mut self, obstacles: u32) {
        self.place_goals();
        loop {
            self.randomize_crystals(obstacles);
            update_power(&mut self.board, self.start);
            if !finish_powered(&self.board, self.finish) {
                break;
            }
        }
    }

    /// Spec 3 `placeGoals`. Seed 1 on 4x4 uses the fixed tutorial goals.
    /// Otherwise the finish is drawn until it is at least `max(w, h) - 1`
    /// from the start; after [`FINISH_DRAWS`] misses the last draw is kept.
    fn place_goals(&mut self) {
        if self.seed == 1 && self.w == 4 && self.h == 4 {
            self.start = goal_on(3, 0, self.w, self.h);
            self.finish = goal_on(1, 2, self.w, self.h);
            return;
        }
        let min_dist = self.w.max(self.h) - 1;
        self.start = self.draw_goal();
        let mut finish = self.draw_goal();
        let mut draws = 1;
        while distance_squared(self.start, finish) < min_dist * min_dist {
            if draws == FINISH_DRAWS {
                tracing::warn!(
                    entity_id = self.entity_id,
                    entity_name = self.player_name.as_deref(),
                    player_id = self.player_id,
                    player_name = self.player_name.as_deref(),
                    w = self.w,
                    h = self.h,
                    reason = "goal_distance_unmet",
                    "GoauldCrystals: finish goal too close to the start after 100 draws"
                );
                break;
            }
            finish = self.draw_goal();
            draws += 1;
        }
        self.finish = finish;
    }

    /// One goal draw: a side, then an offset along the side.
    fn draw_goal(&mut self) -> Goal {
        let side = self.rng.random_range(0..4u8);
        let offset = if side % 2 == 0 {
            self.rng.random_range(0..self.w)
        } else {
            self.rng.random_range(0..self.h)
        };
        goal_on(side, offset, self.w, self.h)
    }

    /// Spec 3 `randomizeCrystals`. Greens, yellows, oranges and reds are
    /// placed on empty cells; the optional counts shrink until the board's
    /// empty cells (`w * h - h`) cover the budget. Fractional counts place
    /// `ceil(count)` crystals.
    fn randomize_crystals(&mut self, obstacles: u32) {
        if self.seed == 1 && self.w == 4 && self.h == 4 {
            self.board = tutorial_board();
            return;
        }
        self.board = vec![vec![Cell::default(); self.h]; self.w];
        let greens = (self.start.x.abs_diff(self.finish.x)
            + self.start.y.abs_diff(self.finish.y)
            + 3) as f64;
        let mut yellows = (self.w + self.h) as f64 * 0.5 + 1.0;
        let mut oranges = 0.0_f64;
        let mut reds = (self.w * self.h) as f64 * 0.25 + obstacles as f64;
        let capacity = (self.w * self.h - self.h) as f64;
        let total = |yellows: f64, oranges: f64, reds: f64| greens + yellows + oranges + reds;
        while capacity < total(yellows, oranges, reds)
            && (reds > 0.0 || yellows > 0.0 || oranges > 0.0)
        {
            if reds > 0.0 {
                reds -= 1.0;
            }
            if capacity < total(yellows, oranges, reds) && yellows > 0.0 {
                yellows -= 1.0;
            }
            if capacity < total(yellows, oranges, reds) && oranges > 0.0 {
                oranges -= 1.0;
            }
        }
        for (color, mut count) in [(1u8, greens), (2, yellows), (3, oranges), (4, reds)] {
            while count > 0.0 {
                let x = self.rng.random_range(0..self.w);
                let y = self.rng.random_range(0..self.h);
                if self.board[x][y].color == 0 {
                    let var = self.rng.random_range(0..3u8);
                    self.board[x][y] = Cell {
                        color,
                        var,
                        power: false,
                    };
                    count -= 1.0;
                }
            }
        }
    }

    /// Spec 5 `fullgamestate`: every key the client reads on a full redraw.
    /// The cheat bars are sent as constants (no item cheats, no CA counts).
    pub(super) fn build_full_game_state(&self) -> HashMap<String, SfsValue> {
        let (sel_x, sel_y) = match self.selected {
            Some((x, y)) => (x as f64, y as f64),
            None => (-1.0, -1.0),
        };
        let columns: Vec<SfsValue> = self
            .board
            .iter()
            .map(|column| SfsValue::list(column.iter().map(cell_value)))
            .collect();
        sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "playfieldWidth" => number(self.w as f64),
            "playfieldHeight" => number(self.h as f64),
            "startGoalSide" => number(self.start.side as f64),
            "startGoalOffset" => number(self.start.offset as f64),
            "startGoalX" => number(self.start.x as f64),
            "startGoalY" => number(self.start.y as f64),
            "finishGoalSide" => number(self.finish.side as f64),
            "finishGoalOffset" => number(self.finish.offset as f64),
            "finishGoalX" => number(self.finish.x as f64),
            "finishGoalY" => number(self.finish.y as f64),
            "selectedXPos" => number(sel_x),
            "selectedYPos" => number(sel_y),
            "timeRemaining" => number(self.time_remaining),
            "maxTime" => number(self.max_time),
            "timerState" => number(self.timer_state as f64),
            "activeItemCheat" => number(-1.0),
            "instcc" => number(-1.0),
            "CA0" => number(0.0),
            "CA1" => number(0.0),
            "CA2" => number(0.0),
            "CA3" => number(0.0),
            "CA4" => number(0.0),
            "abilityBitfield" => number(self.ability_bitfield as f64),
            "gameArray" => SfsValue::list(columns),
        }
    }

    /// Spec 5 `timerupdate`: the clock value and its state.
    pub(super) fn build_timer_update(&self) -> HashMap<String, SfsValue> {
        sfs_vars! {
            "_cmd" => SfsValue::String("timerupdate".to_string()),
            "timeRemaining" => number(self.time_remaining),
            "timerState" => number(self.timer_state as f64),
        }
    }
}
