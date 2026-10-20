//! Alignment cascade (spec section 6, MG-AL2b): the refill after a gravity
//! slide (the power-up grants A and B, the blocker and clone rolls), the
//! `getNewType` table, the re-roll pass, and the `hexmove`, `bypasshexspinner`
//! and `cascadecheckcomplete` messages. The gravity walks are in `moves_gravity`.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::RngExt;

use super::moves::{cell_value, check_power_up_matches, get_match_power, Cell};
use super::moves_gravity::{cell_at, Pos, Slide};
use super::piece::Piece;
use super::setup::{check_for_matches, check_neighbor_matches, number, BONUS_ICONS};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// `getNewType` reads `difficulty` and then overwrites it with 1 (spec section
/// 6, a shipped quirk), so every refill rolls the tier 1 table. The tier 2-4
/// tables exist in the client but are unreachable; they are not ported.
const NEW_TYPE_USES_TIER: bool = false;

/// The blocker type a refill can place (spec section 6, 8% of the rolls).
const BLOCKER: u8 = 9;

/// `getNewType` (spec section 6): the new piece type of a refill cell. The
/// tier argument is inert until `NEW_TYPE_USES_TIER` is turned on.
pub(super) fn get_new_type(rng: &mut StdRng, tier: u8) -> u8 {
    let table = if NEW_TYPE_USES_TIER { tier } else { 1 };
    debug_assert_eq!(table, 1, "only the tier 1 table is ported");
    match rng.random_range(1..=100) {
        1..=12 => 1,
        13..=20 => 2,
        21..=25 => 3,
        26..=58 => 4,
        59..=83 => 5,
        _ => 6,
    }
}

/// The power-up grant of type `kind` for match power `mp`: `kind.p` with the
/// power from `getMatchPower`, or no power where the table has no branch.
fn grant(kind: u8, mp: u8) -> Piece {
    Piece {
        kind,
        power: get_match_power(kind, mp).unwrap_or(0),
    }
}

/// The refill grants (spec section 6): `A` holds the types matched 8 to 11
/// times and `B` the types matched 12 or more, each in ascending type order.
fn grant_lists(matches: &[(Cell, Piece)]) -> (Vec<Piece>, Vec<Piece>) {
    let mut a = Vec::new();
    let mut b = Vec::new();
    for kind in 1..=8u8 {
        let count = matches.iter().filter(|(_, piece)| piece.kind == kind).count();
        if (8..=11).contains(&count) {
            a.push(grant(kind, 8));
        } else if count >= 12 {
            b.push(grant(kind, 12));
        }
    }
    (a, b)
}

/// The refill scan order of direction `d` on a `w` x `h` board (spec section
/// 6 table): the cells as `(x, y)`.
fn scan_order(d: u8, w: usize, h: usize) -> Vec<Cell> {
    let xs: Vec<usize> = match d {
        0 | 1 | 5 => (0..w).rev().collect(),
        _ => (0..w).collect(),
    };
    let ys: Vec<usize> = match d {
        4 | 5 => (0..h).rev().collect(),
        _ => (0..h).collect(),
    };
    xs.iter()
        .flat_map(|&x| ys.iter().map(move |&y| (x, y)))
        .collect()
}

/// The clone test and copy source of a refill cell in direction `d` (spec
/// section 6 table). They differ only for directions 2 and 5: the test may read
/// a different cell than the copy comes from.
fn clone_sources(d: u8, (x, y): Pos) -> (Pos, Pos) {
    let even = y.rem_euclid(2) == 0;
    match d {
        0 => ((x + 1, y), (x + 1, y)),
        1 if even => ((x, y - 1), (x, y - 1)),
        1 => ((x - 1, y - 1), (x - 1, y - 1)),
        2 => {
            let test = if even { (x - 1, y - 1) } else { (x, y - 1) };
            (test, (x, y - 1))
        }
        3 => ((x - 1, y), (x - 1, y)),
        4 if even => ((x - 1, y + 1), (x - 1, y + 1)),
        4 => ((x, y + 1), (x, y + 1)),
        5 => {
            let copy = if even { (x, y + 1) } else { (x + 1, y + 1) };
            ((x + 1, y), copy)
        }
        _ => ((x, y), (x, y)),
    }
}

/// The piece a clone copies into the refill cell, or `None` when there is no
/// clone: the test cell must be on the board and hold a type, and the copy
/// source must be on the board.
fn clone_piece(board: &[Vec<Piece>], d: u8, pos: Pos) -> Option<Piece> {
    let (test, copy) = clone_sources(d, pos);
    let test = cell_at(board, test)?;
    if board[test.0][test.1].kind == 0 {
        return None;
    }
    let copy = cell_at(board, copy)?;
    Some(board[copy.0][copy.1])
}

/// The refill of direction `d` (spec section 6): every empty cell, in the
/// direction's scan order, takes a grant, a blocker, a clone or a new type. The
/// board is updated as it goes and the new cells are returned in scan order.
pub(super) fn refill(
    rng: &mut StdRng,
    board: &mut [Vec<Piece>],
    matches: &[(Cell, Piece)],
    d: u8,
    tier: u8,
) -> Vec<(Cell, Piece)> {
    let (mut a, mut b) = grant_lists(matches);
    let (w, h) = (board.len(), board.first().map_or(0, Vec::len));
    let mut new_hexes = Vec::new();
    for (x, y) in scan_order(d, w, h) {
        if board[x][y].kind != 0 {
            continue;
        }
        let piece = if !a.is_empty() {
            a.remove(0)
        } else if !b.is_empty() {
            b.remove(0)
        } else {
            // Both rolls are drawn before the branch, as the listing does.
            let r28 = rng.random_range(1..=100);
            let r29 = rng.random_range(1..=100);
            let clone = if r28 <= 10 {
                clone_piece(board, d, (x as i64, y as i64))
            } else {
                None
            };
            if r29 <= 8 {
                Piece::plain(BLOCKER)
            } else if let Some(piece) = clone {
                piece
            } else {
                Piece::plain(get_new_type(rng, tier))
            }
        };
        board[x][y] = piece;
        new_hexes.push(((x, y), piece));
    }
    new_hexes
}

/// The re-roll pass (spec section 6, only after a scored step): a new cell that
/// completes a match is re-rolled from the bonus and goal icons, and the board
/// takes the new type too. The pass stops early when the first new cell needed
/// no re-roll, which the listing does and the port keeps.
pub(super) fn reroll(
    rng: &mut StdRng,
    board: &mut [Vec<Piece>],
    new_hexes: &mut [(Cell, Piece)],
    cascade_count: u32,
    goal_icons: &[u8],
) {
    let pool: Vec<u8> = BONUS_ICONS.iter().chain(goal_icons).copied().collect();
    for _ in 0..=cascade_count {
        let mut rerolled_first = false;
        for i in (0..new_hexes.len()).rev() {
            let (x, y) = new_hexes[i].0;
            while completes_match(board, (x, y)) {
                let piece = Piece::plain(pool[rng.random_range(0..pool.len())]);
                board[x][y] = piece;
                new_hexes[i].1 = piece;
                if i == 0 {
                    rerolled_first = true;
                }
            }
        }
        if !rerolled_first {
            break;
        }
    }
}

/// Whether `cell` is part of a match on the current board.
fn completes_match(board: &[Vec<Piece>], cell: Cell) -> bool {
    let seeds = check_for_matches(board);
    check_neighbor_matches(board, &seeds).contains(&cell)
}

/// The `hexmove` reply to `cascade` (spec section 5): the slides in walk order
/// and the new cells.
pub(super) fn hexmove(
    direction: u8,
    slides: &[Slide],
    new_hexes: &[(Cell, Piece)],
) -> HashMap<String, SfsValue> {
    let moves = SfsValue::list(slides.iter().map(|slide| {
        SfsValue::Object(sfs_vars! {
            "moveTotal" => number(slide.gaps),
            "x" => number(slide.cell.0 as u32),
            "y" => number(slide.cell.1 as u32),
            "type" => number(slide.piece.wire()),
        })
    }));
    sfs_vars! {
        "_cmd" => SfsValue::String("hexmove".to_string()),
        "direction" => number(direction),
        "hexesToMove" => moves,
        "newHexes" => cell_list(new_hexes),
    }
}

/// The `bypasshexspinner` reply to a cascade match (spec section 5): the cells
/// the cascade removes, without the spinner.
pub(super) fn bypasshexspinner(matches: &[(Cell, Piece)]) -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("bypasshexspinner".to_string()),
        "matches" => cell_list(matches),
    }
}

/// The `cascadecheckcomplete` reply: the cascade has no match left (spec
/// section 5). It has no fields.
pub(super) fn cascadecheckcomplete() -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("cascadecheckcomplete".to_string()),
    }
}

/// A list of board cells as `{x, y, type}` objects.
fn cell_list(cells: &[(Cell, Piece)]) -> SfsValue {
    SfsValue::list(
        cells
            .iter()
            .map(|&((x, y), piece)| cell_value(x, y, piece)),
    )
}

/// The piece types of `matches`, in order: the `scores` list of spec section 6.
pub(super) fn types(matches: &[(Cell, Piece)]) -> Vec<u8> {
    matches.iter().map(|(_, piece)| piece.kind).collect()
}

/// The expanded cascade set (spec section 6): the flood of the seeds, grown by
/// the power-ups it holds, with each cell's piece read from `board`.
pub(super) fn expanded_matches(board: &[Vec<Piece>], seeds: &[Cell]) -> Vec<(Cell, Piece)> {
    let flood = check_neighbor_matches(board, seeds);
    let (set, _) = check_power_up_matches(board, &flood);
    set.into_iter()
        .map(|(x, y)| ((x, y), board[x][y]))
        .collect()
}
