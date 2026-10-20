//! Alignment setup: the tier and goal tables (spec section 4), the board
//! generation (section 3) and the `fullgamestate` builder (section 5).

use std::collections::{HashMap, VecDeque};

use rand::rngs::StdRng;
use rand::RngExt;

use super::AlignmentGame;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Piece ids of the bonus icons (cash, experience, achievement).
pub(super) const BONUS_ICONS: [u8; 3] = [1, 2, 3];

/// Board side per tier, indexed by `tier - 1`. Boards are square.
const TIER_SIZE: [usize; 4] = [8, 8, 9, 10];

/// Time limit per tier in seconds, indexed by `tier - 1`.
const TIER_TIME: [u32; 4] = [120, 120, 100, 80];

/// Goal icons per tier, indexed by `tier - 1`.
const TIER_GOAL_ICONS: [&[u8]; 4] = [&[4, 5, 6], &[4, 5, 6, 7], &[4, 5, 6, 7], &[4, 5, 6, 8, 7]];

/// Goal totals for tiers 1, indexed by level 0-10 (power, force, code).
const TOTALS_TIER1: [[u32; 3]; 11] = [
    [45, 15, 6],
    [45, 23, 12],
    [63, 30, 18],
    [72, 38, 18],
    [81, 38, 24],
    [90, 50, 24],
    [105, 63, 30],
    [120, 75, 30],
    [135, 88, 40],
    [150, 100, 48],
    [180, 125, 60],
];

/// Goal totals for tiers 2 and 3, indexed by level 0-10 (power, force, code).
const TOTALS_TIER23: [[u32; 3]; 11] = [
    [63, 30, 18],
    [72, 38, 18],
    [81, 38, 24],
    [90, 50, 24],
    [105, 63, 30],
    [120, 75, 30],
    [135, 88, 40],
    [150, 100, 48],
    [180, 125, 60],
    [198, 130, 66],
    [216, 140, 72],
];

/// Goal totals for tier 4, indexed by level 0-10 (power, force, code, cipher).
const TOTALS_TIER4: [[u32; 4]; 11] = [
    [63, 30, 23, 6],
    [72, 38, 23, 12],
    [81, 38, 30, 18],
    [90, 50, 30, 24],
    [105, 63, 38, 30],
    [120, 75, 45, 30],
    [135, 88, 53, 36],
    [150, 100, 60, 42],
    [180, 125, 75, 48],
    [198, 130, 83, 54],
    [216, 140, 90, 60],
];

/// The fixed per-tier values of spec section 4.
pub(super) struct TierSpec {
    /// Time limit in seconds.
    pub(super) time: u32,
    /// Goal icon piece ids.
    pub(super) goal_icons: &'static [u8],
}

/// The spec's values for a tier (1-4).
pub(super) fn tier_spec(tier: u8) -> TierSpec {
    let index = usize::from(tier) - 1;
    TierSpec {
        time: TIER_TIME[index],
        goal_icons: TIER_GOAL_ICONS[index],
    }
}

/// Clamps a difficulty to the tier range 1-4, logging when it clamps.
pub(super) fn clamp_tier(difficulty: u32) -> u8 {
    let tier = difficulty.clamp(1, 4);
    if tier != difficulty {
        tracing::info!(difficulty, tier, "Alignment: difficulty clamped to a tier");
    }
    tier as u8
}

/// The goal level for a `techcomp`, `floor((techcomp - 1) / 5)` clamped to
/// 0-10, logging when it clamps.
pub(super) fn level_for(techcomp: u32) -> u8 {
    let raw = (i64::from(techcomp) - 1).div_euclid(5);
    let level = raw.clamp(0, 10);
    if level != raw {
        tracing::info!(techcomp, level, "Alignment: goal level clamped");
    }
    level as u8
}

/// The goal totals for a tier and level, one per goal score slot.
pub(super) fn goal_totals(tier: u8, level: u8) -> Vec<u32> {
    let level = usize::from(level);
    match tier {
        1 => TOTALS_TIER1[level].to_vec(),
        2 | 3 => TOTALS_TIER23[level].to_vec(),
        _ => TOTALS_TIER4[level].to_vec(),
    }
}

/// The triangle partner pairs of spec section 3, as neighbour indices.
const TRIANGLE_PAIRS: [(usize, usize); 6] = [(5, 0), (4, 5), (3, 4), (0, 1), (1, 2), (2, 3)];

/// Neighbour offsets n0-n5 for an even column `y`.
const OFFSETS_EVEN: [(i32, i32); 6] = [(-1, 0), (-1, 1), (0, 1), (1, 0), (0, -1), (-1, -1)];

/// Neighbour offsets n0-n5 for an odd column `y`.
const OFFSETS_ODD: [(i32, i32); 6] = [(-1, 0), (0, 1), (1, 1), (1, 0), (1, -1), (0, -1)];

/// Neighbour `n` (0-5) of cell `(x, y)`, or `None` when it is off the board.
fn neighbour(board: &[Vec<u8>], x: usize, y: usize, n: usize) -> Option<(usize, usize)> {
    let offsets = if y.is_multiple_of(2) {
        OFFSETS_EVEN
    } else {
        OFFSETS_ODD
    };
    let nx = x as i32 + offsets[n].0;
    let ny = y as i32 + offsets[n].1;
    if nx < 0 || ny < 0 {
        return None;
    }
    let (nx, ny) = (nx as usize, ny as usize);
    let column = board.get(nx)?;
    (ny < column.len()).then_some((nx, ny))
}

/// The two neighbours that make a triangle with cell `(x, y)`, when both are
/// on the board and share its type (spec section 3, `checkForMatches`).
fn triangle_partners(
    board: &[Vec<u8>],
    x: usize,
    y: usize,
) -> Option<((usize, usize), (usize, usize))> {
    let kind = board[x][y];
    if kind == 9 {
        return None;
    }
    TRIANGLE_PAIRS.iter().find_map(|&(a, b)| {
        let first = neighbour(board, x, y, a)?;
        let second = neighbour(board, x, y, b)?;
        let same = board[first.0][first.1] == kind && board[second.0][second.1] == kind;
        same.then_some((first, second))
    })
}

/// Every cell that is matched on the board, in `x` then `y` order (spec section
/// 3). A triangle of three equal hexes is a seed, and a breadth-first flood
/// takes its whole connected group of the same type.
pub(super) fn check_for_matches(board: &[Vec<u8>]) -> Vec<(usize, usize)> {
    let mut matched: Vec<Vec<bool>> = board
        .iter()
        .map(|column| vec![false; column.len()])
        .collect();
    let mut queue = VecDeque::new();
    for x in 0..board.len() {
        for y in 0..board[x].len() {
            let Some((first, second)) = triangle_partners(board, x, y) else {
                continue;
            };
            for cell in [(x, y), first, second] {
                if !matched[cell.0][cell.1] {
                    matched[cell.0][cell.1] = true;
                    queue.push_back(cell);
                }
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let kind = board[x][y];
        for n in 0..6 {
            let Some((nx, ny)) = neighbour(board, x, y, n) else {
                continue;
            };
            if !matched[nx][ny] && board[nx][ny] == kind {
                matched[nx][ny] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    let mut cells = Vec::new();
    for (x, column) in matched.iter().enumerate() {
        for (y, &hit) in column.iter().enumerate() {
            if hit {
                cells.push((x, y));
            }
        }
    }
    cells
}

/// Fills a square board for a tier with no initial match (spec section 3).
/// The first quarter of the cells, in `x` then `y` order, are bonus icons; the
/// rest are goal icons. Matched cells are re-rolled until none remain.
///
/// The 5% power-up roll (`t + 0.p`) is not drawn: the board holds `u8` piece ids
/// (the packet's board type), which cannot carry a fractional power-up.
pub(super) fn select_icons(rng: &mut StdRng, tier: u8) -> Vec<Vec<u8>> {
    let size = TIER_SIZE[usize::from(tier) - 1];
    let goal_icons = tier_spec(tier).goal_icons;
    let bonus_count = ((size * size) as f64 * 0.25).round() as usize;
    let mut board = vec![vec![0u8; size]; size];
    let mut k = 0;
    for column in board.iter_mut() {
        for cell in column.iter_mut() {
            *cell = if k < bonus_count {
                BONUS_ICONS[rng.random_range(0..BONUS_ICONS.len())]
            } else {
                goal_icons[rng.random_range(0..goal_icons.len())]
            };
            k += 1;
        }
    }
    loop {
        let matches = check_for_matches(&board);
        if matches.is_empty() {
            return board;
        }
        for (x, y) in matches {
            let pool: &[u8] = if BONUS_ICONS.contains(&board[x][y]) {
                &BONUS_ICONS
            } else {
                goal_icons
            };
            board[x][y] = pool[rng.random_range(0..pool.len())];
        }
    }
}

/// A number value; every SFS number is a double.
fn number<T: Into<f64>>(value: T) -> SfsValue {
    SfsValue::Number(value.into())
}

/// A list of numbers.
fn number_list<T: Into<f64> + Copy>(items: &[T]) -> SfsValue {
    SfsValue::list(items.iter().map(|&item| number(item)))
}

impl AlignmentGame {
    /// The `fullgamestate` message (spec section 5), sent on start and reset.
    /// The item cheats are off (`instcc -1`, `CA0`-`CA4 0`) until AL3b.
    pub(super) fn full_game_state(&self) -> HashMap<String, SfsValue> {
        let size = self.board.len();
        let game_array = SfsValue::list(self.board.iter().map(|column| number_list(column)));
        sfs_vars! {
            "_cmd" => SfsValue::String("fullgamestate".to_string()),
            "numResets" => number(self.num_resets),
            "timeRemaining" => number(self.time_remaining - 0.25),
            "timerState" => number(self.timer_state),
            "isSpectator" => SfsValue::Bool(false),
            "gameArray" => game_array,
            "playfieldWidth" => number(size as u32),
            "playfieldHeight" => number(size as u32),
            "goalIcons" => number_list(&self.goal_icons),
            "goalTotals" => number_list(&self.goal_totals),
            "goalScores" => number_list(&self.goal_scores),
            "techcomp" => number(self.tech_competency),
            "difficulty" => number(self.tier),
            "minigamelevel" => number(self.level),
            "instcc" => number(-1.0),
            "CA0" => number(0.0),
            "CA1" => number(0.0),
            "CA2" => number(0.0),
            "CA3" => number(0.0),
            "CA4" => number(0.0),
            "abilityBitfield" => number(self.abilities_mask),
        }
    }
}
