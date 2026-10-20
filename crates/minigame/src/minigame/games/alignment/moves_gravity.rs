//! Alignment gravity (spec section 6, `doCascade`, MG-AL2b): the six slide
//! walks that close the gaps a cascade's matches left. Each walk visits its
//! lines in order. An empty cell opens a gap; a piece after a gap slides back
//! along the line by the gaps seen so far, and the move is recorded at the
//! piece's original cell.

use super::moves::Cell;
use super::piece::Piece;

/// One piece that slid: its original cell, the steps it moved and the piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Slide {
    pub(super) gaps: u32,
    pub(super) cell: Cell,
    pub(super) piece: Piece,
}

/// A position that may lie off the board: a walk steps past an edge before the
/// step is checked.
pub(super) type Pos = (i64, i64);

/// Whether row `y` is even (the walks alternate on row parity).
fn is_even(y: i64) -> bool {
    y.rem_euclid(2) == 0
}

/// The board cell at `pos`, or `None` when it lies off the board.
pub(super) fn cell_at(board: &[Vec<Piece>], (x, y): Pos) -> Option<Cell> {
    let cell = (usize::try_from(x).ok()?, usize::try_from(y).ok()?);
    board.get(cell.0)?.get(cell.1).map(|_| cell)
}

/// One slide step of direction `d` from `pos` (spec section 6 table). A slide
/// always moves back along its walk, toward the cells already visited.
pub(super) fn slide_step(d: u8, (x, y): Pos) -> Pos {
    let even = is_even(y);
    match d {
        0 => (x + 1, y),
        1 if even => (x, y - 1),
        1 => (x + 1, y - 1),
        2 if even => (x - 1, y - 1),
        2 => (x, y - 1),
        3 => (x - 1, y),
        4 if even => (x - 1, y + 1),
        4 => (x, y + 1),
        // Direction 5 is the last one, so it is also the fallback.
        5 if even => (x, y + 1),
        _ => (x + 1, y + 1),
    }
}

/// One walk step of direction `d` from `pos`: the cell whose slide step lands
/// on `pos`. Walking the inverse of the slide keeps a line and its slides on
/// the same cells; the parity of the row walked into picks the x change.
pub(super) fn walk_step(d: u8, (x, y): Pos) -> Pos {
    match d {
        0 => (x - 1, y),
        1 => {
            let ny = y + 1;
            if is_even(ny) {
                (x, ny)
            } else {
                (x - 1, ny)
            }
        }
        2 => {
            let ny = y + 1;
            if is_even(ny) {
                (x + 1, ny)
            } else {
                (x, ny)
            }
        }
        3 => (x + 1, y),
        4 => {
            let ny = y - 1;
            if is_even(ny) {
                (x + 1, ny)
            } else {
                (x, ny)
            }
        }
        _ => {
            let ny = y - 1;
            if is_even(ny) {
                (x, ny)
            } else {
                (x - 1, ny)
            }
        }
    }
}

/// The start cell of every line of direction `d` on a `w` x `h` board, in the
/// order of the spec table. A start may lie off the board: its line is walked
/// anyway and the off-board cells are skipped.
fn starts(d: u8, w: i64, h: i64) -> Vec<Pos> {
    match d {
        0 => (0..h).rev().map(|y| (w - 1, y)).collect(),
        1 => (0..=h + 3).map(|s| (s, 0)).collect(),
        2 => (-3..=w - 1).map(|s| (s, 0)).collect(),
        3 => (0..h).map(|y| (0, y)).collect(),
        4 => (-4..=w - 1).rev().map(|s| (s, h)).collect(),
        _ => (0..=w + 3).rev().map(|s| (s, 8)).collect(),
    }
}

/// The cell `gaps` slide steps from `pos`, or `None` when a step leaves the
/// board. The slide parity is read from each row the piece passes.
fn slide_by(board: &[Vec<Piece>], d: u8, pos: Pos, gaps: u32) -> Option<Cell> {
    let mut at = pos;
    for _ in 0..gaps {
        at = slide_step(d, at);
        cell_at(board, at)?;
    }
    cell_at(board, at)
}

/// `doCascade`'s gravity for drop direction `d` (spec section 6). Every line is
/// walked in order; an off-board cell neither counts as a gap nor moves. The
/// board is updated as the walk goes, so later lines see the earlier moves. The
/// slides are returned in walk order.
pub(super) fn gravity(board: &mut [Vec<Piece>], d: u8) -> Vec<Slide> {
    let w = board.len() as i64;
    let h = board.first().map_or(0, Vec::len) as i64;
    // Longer than any line on the board: the walk runs off the edge first.
    let steps = w + h + 8;
    let mut slides = Vec::new();
    for start in starts(d, w, h) {
        let mut gaps = 0;
        let mut pos = start;
        for _ in 0..steps {
            if let Some(cell) = cell_at(board, pos) {
                let piece = board[cell.0][cell.1];
                if piece.kind == 0 {
                    gaps += 1;
                } else if gaps > 0 {
                    if let Some(dest) = slide_by(board, d, pos, gaps) {
                        board[dest.0][dest.1] = piece;
                        board[cell.0][cell.1] = Piece::plain(0);
                        slides.push(Slide { gaps, cell, piece });
                    }
                }
            }
            pos = walk_step(d, pos);
        }
    }
    slides
}
