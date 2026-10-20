//! GoauldCrystals power: the breadth-first green path from the start goal
//! and the `powerstatus` builder. Spec section 3 "Power" and section 5.

use std::collections::{HashMap, VecDeque};

use super::{Cell, Goal};
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Colour of a green crystal, the only colour the powered path follows.
const GREEN: u8 = 1;

/// Recomputes the powered path. Every `power` flag is cleared first. If the
/// start cell is green, a breadth-first search from it follows green cells
/// through 4-neighbours in the order `x-1, y-1, x+1, y+1`, sets `power` on
/// each one, and returns them in visit order.
pub(super) fn update_power(board: &mut [Vec<Cell>], start: Goal) -> Vec<(usize, usize)> {
    for cell in board.iter_mut().flatten() {
        cell.power = false;
    }
    let mut visited = Vec::new();
    let Some(start_cell) = board.get(start.x).and_then(|column| column.get(start.y)) else {
        return visited;
    };
    if start_cell.color != GREEN {
        return visited;
    }
    let w = board.len();
    let h = board.first().map_or(0, Vec::len);
    board[start.x][start.y].power = true;
    visited.push((start.x, start.y));
    let mut queue = VecDeque::from([(start.x, start.y)]);
    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in neighbours(x, y, w, h) {
            let cell = &mut board[nx][ny];
            if cell.color == GREEN && !cell.power {
                cell.power = true;
                visited.push((nx, ny));
                queue.push_back((nx, ny));
            }
        }
    }
    visited
}

/// The in-bounds 4-neighbours of `(x, y)` in the order `x-1, y-1, x+1, y+1`.
pub(super) fn neighbours(
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> impl Iterator<Item = (usize, usize)> {
    let left = x.checked_sub(1).map(|nx| (nx, y));
    let up = y.checked_sub(1).map(|ny| (x, ny));
    let right = (x + 1 < w).then_some((x + 1, y));
    let down = (y + 1 < h).then_some((x, y + 1));
    [left, up, right, down].into_iter().flatten()
}

/// True when the finish goal cell is on the powered path. The finish is
/// always on the board, so the index is in bounds.
pub(super) fn finish_powered(board: &[Vec<Cell>], finish: Goal) -> bool {
    board[finish.x][finish.y].power
}

/// Spec 5 `powerstatus`: `powerArray` lists the powered cells in visit order.
pub(super) fn build_power_status(visited: &[(usize, usize)]) -> HashMap<String, SfsValue> {
    let cells = visited.iter().map(|&(x, y)| {
        SfsValue::Object(sfs_vars! {
            "xPos" => SfsValue::Number(x as f64),
            "yPos" => SfsValue::Number(y as f64),
        })
    });
    sfs_vars! {
        "_cmd" => SfsValue::String("powerstatus".to_string()),
        "powerArray" => SfsValue::list(cells),
    }
}
