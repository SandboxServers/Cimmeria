//! Tests for CrystalGame's client moves: the validated click, the reset and
//! the refused commands.

use std::collections::HashMap;

use super::setup::{Board, HEIGHT};
use super::CrystalGame;
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;

/// A game whose board is `board`, with the clock idle.
fn game_on(board: Board) -> CrystalGame {
    let mut game = CrystalGame::new(&MinigameSession::offline("CrystalGame", 1, 1));
    game.board = board;
    game
}

/// A solved board with the given cells set to the given colours.
fn board_with(cells: &[((usize, usize), u8)]) -> Board {
    let mut board = Board::solved();
    for &((x, y), colour) in cells {
        board.cells[x * HEIGHT + y] = colour;
    }
    board
}

/// A `processmove` parameter map for `(x, y)`.
fn click(x: f64, y: f64) -> HashMap<String, SfsValue> {
    HashMap::from([
        ("x".to_string(), SfsValue::Number(x)),
        ("y".to_string(), SfsValue::Number(y)),
    ])
}

/// The `Send` payloads of `outputs`, in order.
fn sent(outputs: &[GameOutput]) -> Vec<&HashMap<String, SfsValue>> {
    outputs
        .iter()
        .map(|output| match output {
            GameOutput::Send(map) => map,
            _ => panic!("expected a Send"),
        })
        .collect()
}

/// The `_cmd` of a payload.
fn cmd_of(map: &HashMap<String, SfsValue>) -> &str {
    match &map["_cmd"] {
        SfsValue::String(s) => s.as_str(),
        _ => panic!("_cmd is not a string"),
    }
}

fn number(map: &HashMap<String, SfsValue>, key: &str) -> f64 {
    map[key].as_f64().expect("a number field")
}

/// The `(x, y, num)` of an `updateslot` payload.
fn slot(map: &HashMap<String, SfsValue>) -> (u8, u8, u8) {
    (
        number(map, "x") as u8,
        number(map, "y") as u8,
        number(map, "num") as u8,
    )
}

fn is_solved(board: &Board) -> bool {
    board.cells.iter().all(|&c| c == 3)
}

#[test]
fn first_click_starts_clock_then_sends_slots_in_press_order() {
    let mut game = game_on(Board::solved());
    let out = game.message("processmove", &click(0.0, 0.0));
    let out = sent(&out);
    // Corner: centre (0,0) 3+2=5, right (1,0) 3+1=4, down (0,1) 3+1=4.
    assert_eq!(out.len(), 4);
    assert_eq!(cmd_of(out[0]), "timerupdate");
    assert_eq!(number(out[0], "timeRemaining"), 16.0);
    assert_eq!(number(out[0], "timerState"), 1.0);
    assert_eq!(cmd_of(out[1]), "updateslot");
    assert_eq!(slot(out[1]), (0, 0, 5));
    assert_eq!(slot(out[2]), (1, 0, 4));
    assert_eq!(slot(out[3]), (0, 1, 4));
}

#[test]
fn a_later_click_sends_no_timerupdate() {
    let mut game = game_on(Board::solved());
    game.message("processmove", &click(0.0, 0.0));
    let out = game.message("processmove", &click(0.0, 0.0));
    assert!(sent(&out).iter().all(|m| cmd_of(m) == "updateslot"));
}

#[test]
fn edge_and_inside_clicks_send_four_and_five_slots() {
    let mut edge = game_on(Board::solved());
    let out = edge.message("processmove", &click(0.0, 5.0));
    assert_eq!(out.len(), 1 + 4);
    let mut inside = game_on(Board::solved());
    let out = inside.message("processmove", &click(5.0, 5.0));
    assert_eq!(out.len(), 1 + 5);
}

#[test]
fn centre_wraps_mod_six() {
    // 4 + 2 = 6, which is 0 mod 6.
    let mut game = game_on(board_with(&[((0, 0), 4)]));
    let out = game.message("processmove", &click(0.0, 0.0));
    assert_eq!(slot(sent(&out)[1]), (0, 0, 0));
}

#[test]
fn neighbour_wraps_mod_six() {
    // 5 + 1 = 6, which is 0 mod 6.
    let mut game = game_on(board_with(&[((1, 0), 5)]));
    let out = game.message("processmove", &click(0.0, 0.0));
    assert_eq!(slot(sent(&out)[2]), (1, 0, 0));
}

#[test]
fn six_presses_on_one_cell_are_the_identity() {
    let mut game = game_on(Board::solved());
    for _ in 0..6 {
        game.message("processmove", &click(3.0, 3.0));
    }
    assert!(is_solved(&game.board));
}

#[test]
fn refused_coordinates_send_nothing_and_do_not_start_the_clock() {
    let refused = [
        click(14.0, 0.0),
        click(0.0, 13.0),
        click(-1.0, 0.0),
        click(0.0, -1.0),
        click(1.5, 0.0),
        click(f64::NAN, 0.0),
        HashMap::from([("y".to_string(), SfsValue::Number(0.0))]),
        HashMap::from([
            ("x".to_string(), SfsValue::String("3".to_string())),
            ("y".to_string(), SfsValue::Number(0.0)),
        ]),
    ];
    for params in &refused {
        let mut game = game_on(Board::solved());
        assert!(game.message("processmove", params).is_empty());
        assert!(is_solved(&game.board));
        assert_eq!(game.timer_state, 0);
    }
}

#[test]
fn reset_rescrambles_and_idles_the_clock() {
    let mut game = game_on(Board::solved());
    game.message("processmove", &click(0.0, 0.0));
    assert_eq!(game.timer_state, 1);

    let out = game.message("processreset", &HashMap::new());
    let out = sent(&out);
    assert_eq!(out.len(), 1);
    assert_eq!(cmd_of(out[0]), "fullgamestate");
    assert_eq!(number(out[0], "timerState"), 0.0);
    assert_eq!(number(out[0], "timeRemaining"), 16.0);
    assert!(!is_solved(&game.board));
    assert_eq!(game.num_resets, 1);
    assert_eq!(game.timer_state, 0);

    // The next valid click starts the clock again.
    let out = game.message("processmove", &click(1.0, 1.0));
    assert_eq!(cmd_of(sent(&out)[0]), "timerupdate");
}

#[test]
fn cheat_commands_send_nothing() {
    let mut game = game_on(Board::solved());
    let params = HashMap::from([("cheatNum".to_string(), SfsValue::Number(2.0))]);
    assert!(game.message("activateitemcheat", &params).is_empty());
    assert!(game.message("activateabilitycheat", &params).is_empty());
    assert!(is_solved(&game.board));
    assert_eq!(game.timer_state, 0);
}

#[test]
fn client_victory_and_unknown_commands_send_nothing() {
    let mut game = game_on(Board::solved());
    assert!(game.message("victory", &HashMap::new()).is_empty());
    assert!(game.message("nosuchcommand", &HashMap::new()).is_empty());
    assert!(is_solved(&game.board));
}

#[test]
fn after_abort_moves_and_resets_send_nothing() {
    let mut game = game_on(Board::solved());
    assert!(game.aborted().is_empty());
    assert_eq!(game.outcome, Some("canceled"));
    assert!(game.message("processmove", &click(0.0, 0.0)).is_empty());
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert!(is_solved(&game.board));
    assert_eq!(game.num_resets, 0);
}
