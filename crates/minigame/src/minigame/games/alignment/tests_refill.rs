//! Tests for the Alignment refill and re-roll (MG-AL2b, spec section 6): the
//! power-up grant count, the refill scan start of each direction, the re-roll
//! trigger and its draw cap, and every gravity slide's walk.

use rand::rngs::StdRng;
use rand::SeedableRng;

use super::cascade::{refill, reroll, reroll_cell, REROLL_DRAWS};
use super::moves::Cell;
use super::moves_gravity::{cell_at, gravity, slide_step};
use super::*;

/// An 8x8 board of type 2 pieces.
fn board_of_twos() -> Vec<Vec<Piece>> {
    vec![vec![Piece::plain(2); 8]; 8]
}

/// A power-up in a match is not counted as its base type: seven plain 4s and one
/// `4.1` make seven plain 4s, so no grant (the listing switches on the exact
/// type).
#[test]
fn a_power_up_in_the_match_is_not_counted_for_a_grant() {
    let mut board = vec![vec![Piece::plain(0); 8]; 8];
    let mut matches: Vec<(Cell, Piece)> = (0..7).map(|i| ((i, 0), Piece::plain(4))).collect();
    matches.push(((7, 0), Piece { kind: 4, power: 1 }));
    let mut rng = StdRng::seed_from_u64(1);
    let new_hexes = refill(&mut rng, &mut board, &matches, 0, 1);
    assert_eq!(new_hexes[0].1.power, 0);
}

/// Each direction's refill starts at the first cell of its scan order (spec
/// section 6 table). On an empty board that cell is the first new one.
#[test]
fn refill_scan_starts_at_the_direction_corner() {
    let starts = [
        (1u8, (7, 0)),
        (2, (0, 0)),
        (3, (0, 0)),
        (4, (0, 7)),
        (5, (7, 7)),
    ];
    for (d, start) in starts {
        let mut board = vec![vec![Piece::plain(0); 8]; 8];
        let mut rng = StdRng::seed_from_u64(1);
        let new_hexes = refill(&mut rng, &mut board, &[], d, 1);
        assert_eq!(new_hexes[0].0, start, "direction {d}");
    }
}

/// A new cell that sits in a triangle of its own type is re-rolled: `(2, 2)`
/// with `(1, 1)` and `(1, 2)` of type 4 is a triangle (pair 5, 0).
#[test]
fn reroll_changes_a_new_cell_in_its_own_triangle() {
    let mut board = board_of_twos();
    for (x, y) in [(1, 1), (1, 2), (2, 2)] {
        board[x][y] = Piece::plain(4);
    }
    let mut new_hexes = vec![((2, 2), Piece::plain(4))];
    let mut rng = StdRng::seed_from_u64(1);
    reroll(&mut rng, &mut board, &mut new_hexes, 0, &[]);
    assert_ne!(new_hexes[0].1, Piece::plain(4));
    assert_eq!(board[2][2], new_hexes[0].1);
}

/// A new cell that only touches a match is left alone: `(4, 6)` is next to the
/// triangle `(5, 5)`, `(5, 6)`, `(6, 6)` of type 4 but in no triangle itself.
#[test]
fn reroll_leaves_a_cell_that_only_touches_a_match() {
    let mut board = board_of_twos();
    for (x, y) in [(5, 5), (5, 6), (6, 6), (4, 6)] {
        board[x][y] = Piece::plain(4);
    }
    let mut new_hexes = vec![((4, 6), Piece::plain(4))];
    let mut rng = StdRng::seed_from_u64(1);
    reroll(&mut rng, &mut board, &mut new_hexes, 0, &[]);
    assert_eq!(new_hexes[0].1, Piece::plain(4));
    assert_eq!(board[4][6], Piece::plain(4));
}

/// F1: a draw that never clears the triangle stops at `REROLL_DRAWS` draws and
/// keeps the last one. The closure panics past 10 000 draws, so a missing cap
/// fails fast instead of hanging.
#[test]
fn reroll_cell_stops_at_the_draw_cap() {
    let board = vec![vec![Piece::plain(1); 8]; 8];
    let mut draws = 0;
    let piece = reroll_cell(
        || {
            draws += 1;
            assert!(draws <= 10_000, "no draw cap");
            1
        },
        &board,
        (3, 3),
    );
    assert_eq!(draws, REROLL_DRAWS);
    assert_eq!(piece, Piece::plain(1));
}

/// Every slide is walked: for each direction and each cell `p`, a gap on the
/// cell `p` slides to in that direction makes `p`'s piece slide one step. Skips
/// the gaps the port never walks (spec-alignment section 6, D1): the piece at
/// `(0, 9)` in direction 2, and every gap in row 9 in direction 5.
#[test]
fn every_slide_is_walked_in_every_direction() {
    for d in 0..=5u8 {
        for x in 0..10usize {
            for y in 0..10usize {
                let p = (x as i64, y as i64);
                let mut board = vec![vec![Piece::plain(4); 10]; 10];
                let Some(q) = cell_at(&board, slide_step(d, p)) else {
                    continue;
                };
                if (d, x, y) == (2, 0, 9) || (d == 5 && q.1 == 9) {
                    continue;
                }
                board[q.0][q.1] = Piece::plain(0);
                let slides = gravity(&mut board, d);
                assert!(
                    slides.iter().any(|s| s.cell == (x, y) && s.gaps == 1),
                    "direction {d}, cell {:?}: {slides:?}",
                    (x, y)
                );
            }
        }
    }
}
