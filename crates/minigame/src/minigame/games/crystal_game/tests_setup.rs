//! Tests for the CrystalGame board model and its `fullgamestate`.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::SeedableRng;

use super::setup::{draw_clicks, fullgamestate, Board, HEIGHT, WIDTH};
use super::CrystalGame;
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;

/// First seed whose scramble clicks are (0,0) then (6,6) (T1). Found by
/// [`finds_seed_for_clicks`].
const T1_SEED: u32 = 15435;
/// First seed whose scramble clicks are (13,5) twice (T2).
const T2_SEED: u32 = 6331;

fn rng(seed: u32) -> StdRng {
    StdRng::seed_from_u64(u64::from(seed))
}

fn number(map: &HashMap<String, SfsValue>, key: &str) -> f64 {
    map[key].as_f64().expect("a number field")
}

/// The cells of `board` that are not solved.
fn unsolved(board: &Board) -> usize {
    board.cells.iter().filter(|&&c| c != 3).count()
}

/// Finds the seeds for the T1 and T2 goldens. Run once, by hand:
/// `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-minigame --run-ignored only finds_seed_for_clicks --no-capture`
#[test]
#[ignore]
fn finds_seed_for_clicks() {
    let clicks = |seed: u32| draw_clicks(&mut rng(seed));
    let t1 = (0..100_000u32).find(|&s| clicks(s) == [(0, 0), (6, 6)]);
    let t2 = (0..100_000u32).find(|&s| clicks(s) == [(13, 5), (13, 5)]);
    println!("T1_SEED={t1:?} T2_SEED={t2:?}");
    assert!(t1.is_some() && t2.is_some(), "no seed in 0..100000");
}

#[test]
fn board_is_14_by_13_and_solved_at_start() {
    let board = Board::solved();
    assert_eq!((board.w, board.h), (14, 13));
    assert_eq!(board.cells.len(), WIDTH * HEIGHT);
    assert_eq!(unsolved(&board), 0);
}

#[test]
fn two_presses_on_solved_board_change_expected_cells() {
    let mut board = Board::solved();
    assert_eq!(board.press(0, 0), vec![(0, 0, 5), (1, 0, 4), (0, 1, 4)]);
    assert_eq!(
        board.press(6, 6),
        vec![(6, 6, 5), (5, 6, 4), (6, 5, 4), (7, 6, 4), (6, 7, 4)]
    );
    // Corner click: 3 cells. Interior click: 5 cells. No overlap.
    assert_eq!(unsolved(&board), 8);
    assert_eq!(board.cells[0], 5);
    assert_eq!(board.cells[6 * HEIGHT + 6], 5);
}

#[test]
fn press_in_corner_on_edge_and_inside_changes_3_4_and_5_cells_in_order() {
    let corner = Board::solved().press(0, 0);
    assert_eq!(corner, vec![(0, 0, 5), (1, 0, 4), (0, 1, 4)]);

    let edge = Board::solved().press(0, 5);
    assert_eq!(edge, vec![(0, 5, 5), (0, 4, 4), (1, 5, 4), (0, 6, 4)]);

    let inside = Board::solved().press(6, 6);
    assert_eq!(
        inside,
        vec![(6, 6, 5), (5, 6, 4), (6, 5, 4), (7, 6, 4), (6, 7, 4)]
    );

    let far_corner = Board::solved().press(13, 12);
    assert_eq!(far_corner, vec![(13, 12, 5), (12, 12, 4), (13, 11, 4)]);
}

#[test]
fn draws_are_x1_y1_x2_y2_from_the_seed() {
    // Fixed values: swapping the draw order or the axes moves these.
    assert_eq!(draw_clicks(&mut rng(T1_SEED)), [(0, 0), (6, 6)]);
    assert_eq!(draw_clicks(&mut rng(T2_SEED)), [(13, 5), (13, 5)]);
}

#[test]
fn t1_seed_board_is_the_two_presses() {
    let mut board = Board::solved();
    board.scramble(&mut rng(T1_SEED));
    let mut expected = Board::solved();
    expected.press(0, 0);
    expected.press(6, 6);
    assert_eq!(board, expected);
}

#[test]
fn t2_seed_overlapping_clicks_add_up() {
    let mut board = Board::solved();
    board.scramble(&mut rng(T2_SEED));
    // Two clicks on (13,5): the centre gets +4, each in-bounds neighbour +2.
    let at = |x: usize, y: usize| board.cells[x * HEIGHT + y];
    assert_eq!(at(13, 5), 1);
    assert_eq!(at(12, 5), 5);
    assert_eq!(at(13, 4), 5);
    assert_eq!(at(13, 6), 5);
    assert_eq!(unsolved(&board), 4);
}

#[test]
fn scrambled_board_is_never_solved() {
    for seed in 0..2000 {
        let mut board = Board::solved();
        board.scramble(&mut rng(seed));
        assert!(unsolved(&board) > 0, "seed {seed} solved the board");
    }
}

#[test]
fn fullgamestate_has_14_columns_of_13_cells() {
    let map = fullgamestate(&Board::solved());
    let SfsValue::Object(columns) = &map["gameArray"] else {
        panic!("gameArray is not a list");
    };
    assert_eq!(columns.len(), 14);
    for column in columns.values() {
        let SfsValue::Object(cells) = column else {
            panic!("column is not a list");
        };
        assert_eq!(cells.len(), 13);
    }
}

#[test]
fn fullgamestate_carries_the_clock_and_no_cheats() {
    let map = fullgamestate(&Board::solved());
    assert_eq!(map["_cmd"].as_str(), Some("fullgamestate"));
    assert_eq!(number(&map, "playfieldWidth"), 14.0);
    assert_eq!(number(&map, "playfieldHeight"), 13.0);
    assert_eq!(number(&map, "timeRemaining"), 16.0);
    assert_eq!(number(&map, "timerState"), 0.0);
    assert_eq!(number(&map, "instcc"), -1.0);
    for key in ["CA0", "CA1", "CA2", "CA3", "CA4", "abilityBitfield"] {
        assert_eq!(number(&map, key), 0.0, "{key}");
    }
}

#[test]
fn started_sends_one_fullgamestate() {
    let mut game = CrystalGame {
        board: Board::solved(),
    };
    let outputs = game.started();
    assert_eq!(outputs.len(), 1);
    let GameOutput::Send(payload) = &outputs[0] else {
        panic!("expected GameOutput::Send, got {:?}", outputs[0]);
    };
    assert_eq!(payload["_cmd"].as_str(), Some("fullgamestate"));
}
