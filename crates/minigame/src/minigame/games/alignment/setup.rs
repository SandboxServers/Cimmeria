//! Alignment setup: the tier and goal tables (spec section 4), the board
//! generation (section 3) and the `fullgamestate` builder (section 5).

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::RngExt;

use super::piece::Piece;
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
pub(super) fn clamp_tier(difficulty: u32, entity_id: u32, entity_name: Option<&str>) -> u8 {
    let tier = difficulty.clamp(1, 4);
    if tier != difficulty {
        tracing::info!(
            entity_id,
            entity_name,
            difficulty,
            tier,
            "Alignment: difficulty clamped to a tier"
        );
    }
    tier as u8
}

/// The goal level for a `techcomp`, `floor((techcomp - 1) / 5)` clamped to
/// 0-10, logging when it clamps.
pub(super) fn level_for(techcomp: u32, entity_id: u32, entity_name: Option<&str>) -> u8 {
    let raw = (i64::from(techcomp) - 1).div_euclid(5);
    let level = raw.clamp(0, 10);
    if level != raw {
        tracing::info!(
            entity_id,
            entity_name,
            techcomp,
            level,
            "Alignment: goal level clamped"
        );
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
pub(super) fn neighbour(
    board: &[Vec<Piece>],
    x: usize,
    y: usize,
    n: usize,
) -> Option<(usize, usize)> {
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

/// Whether cell `(x, y)` sits in a triangle of three equal types: some pair of
/// its neighbours is on the board and shares its type. Blockers never match.
fn in_triangle(board: &[Vec<Piece>], x: usize, y: usize) -> bool {
    let kind = board[x][y].kind;
    if kind == 9 {
        return false;
    }
    TRIANGLE_PAIRS.iter().any(|&(a, b)| {
        let (Some(first), Some(second)) = (neighbour(board, x, y, a), neighbour(board, x, y, b))
        else {
            return false;
        };
        board[first.0][first.1].kind == kind && board[second.0][second.1].kind == kind
    })
}

/// The triangle seeds of spec section 3 (`checkForMatches`): every cell in a
/// triangle, in `x` then `y` order, each once. Matching uses the piece type, so
/// a power-up matches its base type.
pub(super) fn check_for_matches(board: &[Vec<Piece>]) -> Vec<(usize, usize)> {
    let mut seeds = Vec::new();
    for x in 0..board.len() {
        for y in 0..board[x].len() {
            if in_triangle(board, x, y) {
                seeds.push((x, y));
            }
        }
    }
    seeds
}

/// The flood of spec section 3 (`checkNeighborMatches`): a breadth-first walk
/// from `seeds` through every neighbour of the same type. Returns the cells in
/// visit order, seeds first.
pub(super) fn check_neighbor_matches(
    board: &[Vec<Piece>],
    seeds: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let mut seen: Vec<Vec<bool>> = board
        .iter()
        .map(|column| vec![false; column.len()])
        .collect();
    let mut group = Vec::new();
    for &(x, y) in seeds {
        if !seen[x][y] {
            seen[x][y] = true;
            group.push((x, y));
        }
    }
    // `group` doubles as the queue: `head` is the next cell to expand.
    let mut head = 0;
    while head < group.len() {
        let (x, y) = group[head];
        head += 1;
        let kind = board[x][y].kind;
        for n in 0..6 {
            let Some((nx, ny)) = neighbour(board, x, y, n) else {
                continue;
            };
            if !seen[nx][ny] && board[nx][ny].kind == kind {
                seen[nx][ny] = true;
                group.push((nx, ny));
            }
        }
    }
    group
}

/// Fills a square board for a tier, before any re-roll (spec section 3). The
/// first quarter of the cells, in `x` then `y` order, are bonus icons; the rest
/// are goal icons. Each bonus cell draws its type, then a 5% roll for a
/// power-up (`t + 0.1`); goal cells draw no roll.
pub(super) fn fill_board(rng: &mut StdRng, tier: u8) -> Vec<Vec<Piece>> {
    let size = TIER_SIZE[usize::from(tier) - 1];
    let goal_icons = tier_spec(tier).goal_icons;
    let bonus_count = ((size * size) as f64 * 0.25).round() as usize;
    let mut board = vec![vec![Piece::plain(0); size]; size];
    let mut k = 0;
    for column in board.iter_mut() {
        for cell in column.iter_mut() {
            *cell = if k < bonus_count {
                let kind = BONUS_ICONS[rng.random_range(0..BONUS_ICONS.len())];
                let power = u8::from(rng.random_range(1..=100) <= 5);
                Piece { kind, power }
            } else {
                Piece::plain(goal_icons[rng.random_range(0..goal_icons.len())])
            };
            k += 1;
        }
    }
    board
}

/// Re-rolls the triangle seeds until no match remains (spec section 3). A seed
/// that is a plain bonus icon is re-rolled from the bonus icons; anything else,
/// including a power-up, is re-rolled from the goal icons and loses its power.
pub(super) fn reroll_matches(rng: &mut StdRng, board: &mut [Vec<Piece>], goal_icons: &[u8]) {
    loop {
        let seeds = check_for_matches(board);
        if seeds.is_empty() {
            return;
        }
        for (x, y) in seeds {
            let cell = board[x][y];
            let pool: &[u8] = if cell.power == 0 && BONUS_ICONS.contains(&cell.kind) {
                &BONUS_ICONS
            } else {
                goal_icons
            };
            board[x][y] = Piece::plain(pool[rng.random_range(0..pool.len())]);
        }
    }
}

/// Generates the initial board for a tier with no match left on it.
pub(super) fn select_icons(rng: &mut StdRng, tier: u8) -> Vec<Vec<Piece>> {
    let mut board = fill_board(rng, tier);
    reroll_matches(rng, &mut board, tier_spec(tier).goal_icons);
    board
}

/// A number value; every SFS number is a double.
pub(super) fn number<T: Into<f64>>(value: T) -> SfsValue {
    SfsValue::Number(value.into())
}

/// A list of numbers.
pub(super) fn number_list<T: Into<f64> + Copy>(items: &[T]) -> SfsValue {
    SfsValue::list(items.iter().map(|&item| number(item)))
}

impl AlignmentGame {
    /// The `fullgamestate` message (spec section 5), sent on start.
    /// The item cheats are off (`instcc -1`, `CA0`-`CA4 0`) until AL3b.
    pub(super) fn full_game_state(&self) -> HashMap<String, SfsValue> {
        let size = self.board.len();
        let game_array = SfsValue::list(
            self.board
                .iter()
                .map(|column| SfsValue::list(column.iter().map(|piece| number(piece.wire())))),
        );
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
