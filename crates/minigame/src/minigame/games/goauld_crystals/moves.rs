//! GoauldCrystals clicks and moves: the `playfieldclick` handler (spec
//! section 5, steps 1 and 3-9) and the victory test that runs after every
//! move (section 6). Step 2 (armed item cheats) and path mode 2 are not
//! ported: item cheats are gone (M8).

use std::collections::{HashMap, VecDeque};

use super::power::{build_power_status, finish_powered, neighbours, update_power};
use super::{Cell, GoauldCrystalsGame};
use crate::minigame::game::GameOutput;
use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Cell colours, as stored in `Cell::color`.
pub(super) const EMPTY: u8 = 0;
pub(super) const GREEN: u8 = 1;
pub(super) const YELLOW: u8 = 2;
pub(super) const ORANGE: u8 = 3;
pub(super) const RED: u8 = 4;

/// A board position as `(x, y)`.
pub(super) type Pos = (usize, usize);

impl GoauldCrystalsGame {
    /// Spec 5 `playfieldclick`. Validates the click, then runs steps 1 and
    /// 3-9 in order. Input after the end is refused by dispatch.
    pub(super) fn playfield_click(
        &mut self,
        params: &HashMap<String, SfsValue>,
    ) -> Vec<GameOutput> {
        let (Some(x), Some(y)) = (
            read_coordinate(params, "x", self.w),
            read_coordinate(params, "y", self.h),
        ) else {
            self.warn_refused("playfieldclick", "bad_coordinates");
            return vec![];
        };
        let mut out = Vec::new();
        if self.timer_state == 0 {
            self.timer_state = 1;
            out.push(GameOutput::Send(self.build_timer_update()));
        }
        self.apply_red_cheats((x, y), &mut out);
        let Some(selected) = self.selected else {
            self.selected = Some((x, y));
            out.push(selected_update(Some((x, y))));
            out.push(trigger_sound("select"));
            return out;
        };
        out.extend(self.try_move(selected, (x, y)));
        out
    }

    /// Spec 5 steps 3 and 4: while `reds_to_yellows` lasts a red cell becomes
    /// yellow, then while `reds_to_remove` lasts it becomes empty. Only green
    /// cells power the path, so neither step can win and neither sends
    /// `powerstatus`.
    fn apply_red_cheats(&mut self, (x, y): Pos, out: &mut Vec<GameOutput>) {
        if self.reds_to_yellows > 0 && self.board[x][y].color == RED {
            self.reds_to_yellows -= 1;
            self.board[x][y].color = YELLOW;
            out.push(square_state((x, y), &self.board[x][y]));
        }
        if self.reds_to_remove > 0 && self.board[x][y].color == RED {
            self.reds_to_remove -= 1;
            self.board[x][y] = Cell::default();
            out.push(square_state((x, y), &self.board[x][y]));
        }
    }

    /// Spec 5 steps 6-9 for a click on `dst` with `sel` selected. A move
    /// runs the victory test; a refused move sends `invalid`.
    fn try_move(&mut self, sel: Pos, dst: Pos) -> Vec<GameOutput> {
        let path_mode = self.is_path_mode(sel, dst);
        let moved = if path_mode {
            self.path_reaches(sel, dst)
        } else {
            self.is_adjacent_swap(sel, dst)
        };
        let mut out = Vec::new();
        if !moved {
            self.selected = None;
            out.push(selected_update(None));
            out.push(trigger_sound("invalid"));
            // Step 9 runs after every click that reaches step 8, so a refused
            // click also reverts. The same cell is passed twice: nothing moves.
            self.revert_reds(sel, sel, &mut out);
            return out;
        }
        self.swap_cells(sel, dst);
        if !path_mode {
            self.yellow_to_orange(dst);
            self.yellow_to_orange(sel);
        }
        out.extend(self.after_board_change());
        if self.outcome.is_some() {
            return out;
        }
        let (dst_cell, sel_cell) = (self.board[dst.0][dst.1], self.board[sel.0][sel.1]);
        out.push(square_state(dst, &dst_cell));
        out.push(square_state(sel, &sel_cell));
        out.push(trigger_sound("action"));
        self.selected = None;
        out.push(selected_update(None));
        self.revert_reds(sel, dst, &mut out);
        out
    }

    /// True for a green and an empty cell, in either order: the click is a
    /// path move, never an adjacent swap.
    fn is_path_mode(&self, a: Pos, b: Pos) -> bool {
        let (ca, cb) = (self.board[a.0][a.1].color, self.board[b.0][b.1].color);
        (ca == GREEN && cb == EMPTY) || (ca == EMPTY && cb == GREEN)
    }

    /// Step 7's test: Manhattan distance 1 and both colours 2 or less.
    fn is_adjacent_swap(&self, a: Pos, b: Pos) -> bool {
        let distance = a.0.abs_diff(b.0) + a.1.abs_diff(b.1);
        distance == 1
            && self.board[a.0][a.1].color <= YELLOW
            && self.board[b.0][b.1].color <= YELLOW
    }

    /// Step 6's search: a breadth-first walk from `from` over empty cells in
    /// the order `x-1, y-1, x+1, y+1`. A neighbour equal to `to` ends the
    /// search whatever its colour; other neighbours are queued only if they
    /// are empty and unvisited.
    fn path_reaches(&self, from: Pos, to: Pos) -> bool {
        let mut seen = vec![vec![false; self.h]; self.w];
        seen[from.0][from.1] = true;
        let mut queue = VecDeque::from([from]);
        while let Some((x, y)) = queue.pop_front() {
            for (nx, ny) in neighbours(x, y, self.w, self.h) {
                if (nx, ny) == to {
                    return true;
                }
                if self.board[nx][ny].color == EMPTY && !seen[nx][ny] {
                    seen[nx][ny] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
        false
    }

    /// Swaps the two cell objects, so a cell's `var` travels with it.
    fn swap_cells(&mut self, a: Pos, b: Pos) {
        let first = self.board[a.0][a.1];
        self.board[a.0][a.1] = self.board[b.0][b.1];
        self.board[b.0][b.1] = first;
    }

    /// Step 7's yellow rule for one cell of a swap: a yellow that moves
    /// spends one `yellows_without_orange` if any is owed, else turns orange.
    fn yellow_to_orange(&mut self, pos: Pos) {
        if self.board[pos.0][pos.1].color != YELLOW {
            return;
        }
        if self.yellows_without_orange > 0 {
            self.yellows_without_orange -= 1;
        } else {
            self.board[pos.0][pos.1].color = ORANGE;
        }
    }

    /// Spec 6 victory test, run after every move. Recomputes the powered
    /// path. A powered finish ends the game: the clock stops, `fullgamestate`
    /// and `victory` go out and the game returns `GameOutput::Victory`.
    /// Otherwise the new path goes out as `powerstatus`.
    pub(super) fn after_board_change(&mut self) -> Vec<GameOutput> {
        let visited = update_power(&mut self.board, self.start);
        if !finish_powered(&self.board, self.finish) {
            return vec![GameOutput::Send(build_power_status(&visited))];
        }
        self.outcome = Some("victory");
        self.timer_state = 0;
        vec![
            GameOutput::Send(self.build_full_game_state()),
            GameOutput::Send(sfs_vars! {
                "_cmd" => SfsValue::String("victory".to_string()),
            }),
            GameOutput::Victory,
        ]
    }

    /// Spec 5 step 9 (ability 13): the tracked reds follow the swap just
    /// made, then every tracked cell that is still green turns back to red.
    /// Called after any click that reaches step 8 and did not win. A refused
    /// click passes the same cell twice, so no tracked cell moves. The revert
    /// only removes green, so it cannot create a win.
    fn revert_reds(&mut self, a: Pos, b: Pos, out: &mut Vec<GameOutput>) {
        let Some(tracked) = self.reds_to_revert.take() else {
            return;
        };
        for pos in tracked {
            let pos = if pos == a {
                b
            } else if pos == b {
                a
            } else {
                pos
            };
            if self.board[pos.0][pos.1].color == GREEN {
                self.board[pos.0][pos.1].color = RED;
            }
        }
        let visited = update_power(&mut self.board, self.start);
        out.push(GameOutput::Send(build_power_status(&visited)));
        out.push(GameOutput::Send(self.build_full_game_state()));
    }
}

/// Reads a board coordinate. The value must be an integer in `0..limit`;
/// anything else (missing, fractional, negative, outside the board) is `None`.
pub(super) fn read_coordinate(
    params: &HashMap<String, SfsValue>,
    key: &str,
    limit: usize,
) -> Option<usize> {
    let value = params.get(key)?.as_f64()?;
    if value.fract() != 0.0 || value < 0.0 {
        return None;
    }
    let index = value as usize;
    (index < limit).then_some(index)
}

/// `squarestate {xPos, yPos, info}` for one cell.
pub(super) fn square_state(pos: Pos, cell: &Cell) -> GameOutput {
    let info = sfs_vars! {
        "color" => SfsValue::Number(cell.color as f64),
        "var" => SfsValue::Number(cell.var as f64),
        "power" => SfsValue::Bool(cell.power),
    };
    GameOutput::Send(sfs_vars! {
        "_cmd" => SfsValue::String("squarestate".to_string()),
        "xPos" => SfsValue::Number(pos.0 as f64),
        "yPos" => SfsValue::Number(pos.1 as f64),
        "info" => SfsValue::Object(info),
    })
}

/// `selectedupdate {xPos, yPos}`; `-1, -1` when nothing is selected.
pub(super) fn selected_update(selected: Option<Pos>) -> GameOutput {
    let (x, y) = selected.map_or((-1.0, -1.0), |(x, y)| (x as f64, y as f64));
    GameOutput::Send(sfs_vars! {
        "_cmd" => SfsValue::String("selectedupdate".to_string()),
        "xPos" => SfsValue::Number(x),
        "yPos" => SfsValue::Number(y),
    })
}

/// `triggersound {soundName}`.
pub(super) fn trigger_sound(name: &str) -> GameOutput {
    GameOutput::Send(sfs_vars! {
        "_cmd" => SfsValue::String("triggersound".to_string()),
        "soundName" => SfsValue::String(name.to_string()),
    })
}
