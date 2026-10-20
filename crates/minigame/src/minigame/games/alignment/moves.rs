//! Alignment swaps and matching (spec sections 3, 5 and 6, MG-AL2a): the
//! `swappieces` triangle checks, the spin check, the power-up expansion and
//! `getMatchPower`. The cascade and gravity wait for MG-AL2b.

use std::collections::HashMap;

use super::piece::Piece;
use super::setup::{check_for_matches, check_neighbor_matches, neighbour, number, number_list};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// A board cell as `(x, y)`.
pub(super) type Cell = (usize, usize);

/// Why a `swappieces` request is refused (D-MG2). The original validates
/// nothing; the port refuses and logs the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SwapReject {
    OutOfBounds,
    NotTriangle,
    Blocker,
    TypeMismatch,
}

impl SwapReject {
    /// The `reason` field of the WARN line.
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::OutOfBounds => "swap_out_of_bounds",
            Self::NotTriangle => "swap_not_triangle",
            Self::Blocker => "swap_blocker",
            Self::TypeMismatch => "swap_type_mismatch",
        }
    }
}

/// What a match produced: the spin count, the power-up values in visit order
/// and the removed cells with the piece each held.
#[derive(Debug, PartialEq)]
pub(super) struct SwapOutcome {
    pub(super) spin_count: u8,
    pub(super) power_values: Vec<u8>,
    pub(super) matches: Vec<(Cell, Piece)>,
}

/// Reads the three cells of a `swappieces` request. `row<n>` is the board `x`
/// and `col<n>` is the row `y`: only that reading makes the spec's transcript A
/// types match the fixture (section 9).
pub(super) fn read_cells(params: &HashMap<String, SfsValue>) -> Result<[Cell; 3], SwapReject> {
    let mut cells = [(0, 0); 3];
    for (n, cell) in cells.iter_mut().enumerate() {
        let x = coordinate(params, &format!("row{}", n + 1))?;
        let y = coordinate(params, &format!("col{}", n + 1))?;
        *cell = (x, y);
    }
    Ok(cells)
}

/// A whole, non-negative request field that fits a board index, or
/// `OutOfBounds` when it is missing, fractional, non-finite or negative. The
/// check is exact: `as_i32` would truncate `1.9` to cell 1 and map NaN to 0.
fn coordinate(params: &HashMap<String, SfsValue>, key: &str) -> Result<usize, SwapReject> {
    params
        .get(key)
        .and_then(SfsValue::as_f64)
        .filter(|value| {
            value.is_finite()
                && value.fract() == 0.0
                && *value >= 0.0
                && *value <= f64::from(u16::MAX)
        })
        .map(|value| value as usize)
        .ok_or(SwapReject::OutOfBounds)
}

/// The D-MG2 checks of a swap, in the spec's order: the cells are on the
/// board, form a triangle (the first cell with two neighbours that neighbour
/// each other), hold no blocker, and the client's `type1`-`type3` equal the
/// board's pieces. The client types are only compared, never used.
pub(super) fn check_swap(
    board: &[Vec<Piece>],
    cells: [Cell; 3],
    params: &HashMap<String, SfsValue>,
) -> Result<(), SwapReject> {
    let on_board = cells
        .iter()
        .all(|&(x, y)| board.get(x).and_then(|column| column.get(y)).is_some());
    if !on_board {
        return Err(SwapReject::OutOfBounds);
    }
    let [first, second, third] = cells;
    let triangle = first != second
        && first != third
        && second != third
        && are_neighbours(board, first, second)
        && are_neighbours(board, first, third)
        && are_neighbours(board, second, third);
    if !triangle {
        return Err(SwapReject::NotTriangle);
    }
    if cells.iter().any(|&(x, y)| board[x][y].kind == 9) {
        return Err(SwapReject::Blocker);
    }
    for (n, &(x, y)) in cells.iter().enumerate() {
        let client = params
            .get(&format!("type{}", n + 1))
            .and_then(SfsValue::as_f64);
        match client {
            Some(value) if (value - board[x][y].wire()).abs() < 1e-6 => {}
            _ => return Err(SwapReject::TypeMismatch),
        }
    }
    Ok(())
}

/// Whether `b` is one of the six neighbours of `a`.
fn are_neighbours(board: &[Vec<Piece>], a: Cell, b: Cell) -> bool {
    (0..6).any(|n| neighbour(board, a.0, a.1, n) == Some(b))
}

/// The rotation `spin` (1 or 2) of the triangle's pieces (spec section 5,
/// `swapPieces`): rotation 1 puts type3 on the first cell, type1 on the second
/// and type2 on the third; rotation 2 puts type2, type3, type1.
fn rotated(pieces: [Piece; 3], spin: u8) -> [Piece; 3] {
    let [p1, p2, p3] = pieces;
    if spin == 1 {
        [p3, p1, p2]
    } else {
        [p2, p3, p1]
    }
}

/// Writes rotation `spin` of the triangle into `board`.
fn apply_spin(board: &mut [Vec<Piece>], cells: [Cell; 3], spin: u8) {
    let pieces = cells.map(|(x, y)| board[x][y]);
    for (&(x, y), piece) in cells.iter().zip(rotated(pieces, spin)) {
        board[x][y] = piece;
    }
}

/// `spinCheck` (spec section 8): the first rotation (1, then 2) of the
/// triangle that makes a match on the board, or `None` when neither does.
pub(super) fn spin_check(board: &[Vec<Piece>], cells: [Cell; 3]) -> Option<u8> {
    (1u8..=2).find(|&spin| {
        let mut trial = board.to_vec();
        apply_spin(&mut trial, cells, spin);
        !check_for_matches(&trial).is_empty()
    })
}

/// Applies a legal swap (spec section 5, `swapPieces`). Returns the new board
/// with the matched cells emptied, and the outcome. `None` means no rotation
/// matches: the board is unchanged and the reply has `spinCount 3`.
pub(super) fn swap_pieces(
    board: &[Vec<Piece>],
    cells: [Cell; 3],
) -> Option<(Vec<Vec<Piece>>, SwapOutcome)> {
    let spin = spin_check(board, cells)?;
    let mut trial = board.to_vec();
    apply_spin(&mut trial, cells, spin);
    let flood = check_neighbor_matches(&trial, &check_for_matches(&trial));
    let (matched, power_values) = check_power_up_matches(&trial, &flood);
    let matches: Vec<(Cell, Piece)> = matched
        .iter()
        .map(|&(x, y)| ((x, y), trial[x][y]))
        .collect();
    for &((x, y), _) in &matches {
        trial[x][y] = Piece::plain(0);
    }
    let outcome = SwapOutcome {
        spin_count: spin,
        power_values,
        matches,
    };
    Some((trial, outcome))
}

/// `checkPowerUpMatches` (spec section 6): grows the match set by the shape of
/// every power-up in it. The set is re-read on every pass, so a cell a
/// power-up pulls in fires its own shape, and no cell is added twice. Returns
/// the expanded set and the power values in visit order.
pub(super) fn check_power_up_matches(board: &[Vec<Piece>], flood: &[Cell]) -> (Vec<Cell>, Vec<u8>) {
    let mut set = flood.to_vec();
    let mut values = Vec::new();
    let mut visit = 0;
    while visit < set.len() {
        let (x, y) = set[visit];
        visit += 1;
        let power = board[x][y].power;
        if power == 0 {
            continue;
        }
        values.push(power);
        for cell in power_shape(board, (x, y), power) {
            if !set.contains(&cell) {
                set.push(cell);
            }
        }
    }
    (set, values)
}

/// The cells one power-up adds around its centre (spec section 6 table):
/// 1 the neighbours, 2 the row, 3 the radius-2 area, 4 and 5 one diagonal
/// each, 6 the row and both diagonals.
fn power_shape(board: &[Vec<Piece>], centre: Cell, power: u8) -> Vec<Cell> {
    let mut cells = Vec::new();
    if matches!(power, 2 | 6) {
        cells.extend(row_cells(board, centre.1));
    }
    if matches!(power, 4 | 6) {
        cells.extend(diagonal(board, centre, [1, 4]));
    }
    if matches!(power, 5 | 6) {
        cells.extend(diagonal(board, centre, [2, 5]));
    }
    match power {
        1 => cells.extend(within(board, centre, 1)),
        3 => cells.extend(within(board, centre, 2)),
        _ => {}
    }
    cells
}

/// Every in-bounds cell of row `y`, for every `x`.
fn row_cells(board: &[Vec<Piece>], y: usize) -> Vec<Cell> {
    (0..board.len())
        .filter(|&x| y < board[x].len())
        .map(|x| (x, y))
        .collect()
}

/// The cells of one diagonal through `start`, walked from `start` in both
/// neighbour directions `dirs` until the board ends. `start` is not included.
fn diagonal(board: &[Vec<Piece>], start: Cell, dirs: [usize; 2]) -> Vec<Cell> {
    let mut cells = Vec::new();
    for dir in dirs {
        let mut current = start;
        while let Some(next) = neighbour(board, current.0, current.1, dir) {
            cells.push(next);
            current = next;
        }
    }
    cells
}

/// Every in-bounds cell within `radius` steps of `centre`, centre excluded.
/// Radius 1 is the six neighbours; radius 2 adds the twelve outer cells.
fn within(board: &[Vec<Piece>], centre: Cell, radius: usize) -> Vec<Cell> {
    let mut seen = vec![centre];
    let mut frontier = vec![centre];
    for _ in 0..radius {
        let mut next = Vec::new();
        for &(x, y) in &frontier {
            for n in 0..6 {
                if let Some(cell) = neighbour(board, x, y, n) {
                    if !seen.contains(&cell) {
                        seen.push(cell);
                        next.push(cell);
                    }
                }
            }
        }
        frontier = next;
    }
    seen.remove(0);
    seen
}

/// `getMatchPower` (spec section 6): the power a new cell of type `kind`
/// gets for match power `mp` (8 or 12), as the wire digit `p` of `kind.p`.
/// Type 7 under `mp` 12 has no branch: the original yields `NaN`, which the
/// port reports as `None`.
pub(super) fn get_match_power(kind: u8, mp: u8) -> Option<u8> {
    match (mp, kind) {
        (8, 1..=6) => Some(1),
        (8, 7) => Some(2),
        (8, 8) => Some(3),
        (12, 4) => Some(2),
        (12, 5) => Some(4),
        (12, 6) => Some(5),
        (12, 1..=3 | 8) => Some(6),
        _ => None,
    }
}

/// The `feedback` message: sent before every match reply (spec section 5).
pub(super) fn feedback(outcome: &SwapOutcome) -> HashMap<String, SfsValue> {
    sfs_vars! {
        "_cmd" => SfsValue::String("feedback".to_string()),
        "spinCount" => number(outcome.spin_count),
        "powerUpValues" => number_list(&outcome.power_values),
    }
}

/// The `hexspinner` message. A match lists its removed cells; no match sends
/// an empty list with `spinCount 3`. The request fields are echoed back.
pub(super) fn hexspinner(
    params: &HashMap<String, SfsValue>,
    matches: &[(Cell, Piece)],
    spin_count: u8,
) -> HashMap<String, SfsValue> {
    let cells = SfsValue::list(
        matches
            .iter()
            .map(|&((x, y), piece)| cell_value(x, y, piece)),
    );
    let mut vars: HashMap<String, SfsValue> = ECHO_KEYS
        .iter()
        .filter_map(|&key| {
            params
                .get(key)
                .map(|value| (key.to_string(), value.clone()))
        })
        .collect();
    vars.extend(sfs_vars! {
        "_cmd" => SfsValue::String("hexspinner".to_string()),
        "matches" => cells,
        "spinCount" => number(spin_count),
    });
    vars
}

/// The request fields `hexspinner` echoes (spec section 5).
const ECHO_KEYS: [&str; 9] = [
    "xPos",
    "yPos",
    "orientation",
    "row1",
    "row2",
    "row3",
    "col1",
    "col2",
    "col3",
];

/// A board cell as `{x, y, type}`, all Numbers.
fn cell_value(x: usize, y: usize, piece: Piece) -> SfsValue {
    SfsValue::Object(sfs_vars! {
        "x" => number(x as u32),
        "y" => number(y as u32),
        "type" => number(piece.wire()),
    })
}
