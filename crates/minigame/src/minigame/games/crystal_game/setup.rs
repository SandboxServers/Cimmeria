//! CrystalGame board model: the 14 x 13 lights-out board, the click that
//! moves it, the seeded scramble and the `fullgamestate` builder.
//!
//! A cell holds a colour 0-5 and 3 is solved. A click adds 2 to the clicked
//! cell and 1 to each in-bounds neighbour, all mod 6
//! (docs/analysis/minigames/spec-crystalgame.md, section 3).

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::RngExt;

use crate::minigame::protocol::SfsValue;
use crate::sfs_vars;

/// Board width in cells (`initGame`: `playfieldWidth = 14`).
pub(super) const WIDTH: usize = 14;
/// Board height in cells (`initGame`: `playfieldHeight = 13`).
pub(super) const HEIGHT: usize = 13;
/// Colour of a solved cell.
const SOLVED: u8 = 3;
/// Number of colours; a cell's colour is taken mod this.
const COLOURS: u8 = 6;
/// Clicks that scramble a new board (`randomizePlayfield`).
const SCRAMBLE_CLICKS: usize = 2;
/// Seconds on the clock; `fullgamestate` always reports the full value.
pub(super) const TIME_LIMIT: u32 = 16;

/// The lights-out board, stored column-major: `cells[x * h + y]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Board {
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) cells: Vec<u8>,
}

impl Board {
    /// A board with every cell solved (colour 3).
    pub(super) fn solved() -> Self {
        Self {
            w: WIDTH,
            h: HEIGHT,
            cells: vec![SOLVED; WIDTH * HEIGHT],
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        x * self.h + y
    }

    /// Add `delta` to the cell at `(x, y)` mod 6 and return it as
    /// `(x, y, new colour)`.
    fn add(&mut self, x: usize, y: usize, delta: u8) -> (u8, u8, u8) {
        let i = self.index(x, y);
        self.cells[i] = (self.cells[i] + delta) % COLOURS;
        (x as u8, y as u8, self.cells[i])
    }

    /// One click at `(x, y)`. Returns the changed cells in the order centre,
    /// left, up, right, down, skipping neighbours that fall off the board.
    pub(super) fn press(&mut self, x: u8, y: u8) -> Vec<(u8, u8, u8)> {
        let (x, y) = (usize::from(x), usize::from(y));
        let mut changed = vec![self.add(x, y, 2)];
        if x > 0 {
            changed.push(self.add(x - 1, y, 1));
        }
        if y > 0 {
            changed.push(self.add(x, y - 1, 1));
        }
        if x + 1 < self.w {
            changed.push(self.add(x + 1, y, 1));
        }
        if y + 1 < self.h {
            changed.push(self.add(x, y + 1, 1));
        }
        changed
    }

    /// Apply the seeded scramble clicks from [`draw_clicks`].
    pub(super) fn scramble(&mut self, rng: &mut StdRng) {
        for (x, y) in draw_clicks(rng) {
            self.press(x, y);
        }
    }
}

/// The scramble clicks, drawn as x1, y1, x2, y2 from `rng`.
pub(super) fn draw_clicks(rng: &mut StdRng) -> [(u8, u8); SCRAMBLE_CLICKS] {
    let mut clicks = [(0, 0); SCRAMBLE_CLICKS];
    for click in &mut clicks {
        click.0 = rng.random_range(0..WIDTH) as u8;
        click.1 = rng.random_range(0..HEIGHT) as u8;
    }
    clicks
}

/// The `fullgamestate` message for `board`. The clock is at its full value
/// and the cheat fields carry the "no cheats" values `joinOK` uses.
pub(super) fn fullgamestate(board: &Board) -> HashMap<String, SfsValue> {
    let columns = (0..board.w).map(|x| {
        SfsValue::list(
            (0..board.h).map(|y| SfsValue::Number(f64::from(board.cells[board.index(x, y)]))),
        )
    });
    sfs_vars! {
        "_cmd" => SfsValue::String("fullgamestate".to_string()),
        "playfieldWidth" => SfsValue::Number(board.w as f64),
        "playfieldHeight" => SfsValue::Number(board.h as f64),
        "gameArray" => SfsValue::list(columns),
        "timeRemaining" => SfsValue::Number(f64::from(TIME_LIMIT)),
        "timerState" => SfsValue::Number(0.0),
        "instcc" => SfsValue::Number(-1.0),
        "CA0" => SfsValue::Number(0.0),
        "CA1" => SfsValue::Number(0.0),
        "CA2" => SfsValue::Number(0.0),
        "CA3" => SfsValue::Number(0.0),
        "CA4" => SfsValue::Number(0.0),
        "abilityBitfield" => SfsValue::Number(0.0),
    }
}
