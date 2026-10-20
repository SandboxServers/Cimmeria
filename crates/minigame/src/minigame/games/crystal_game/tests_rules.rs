//! Tests for CrystalGame's end of game: the three goldens, the clock, the
//! server-decided victory, the timeout and the validation flag.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::setup::{Board, HEIGHT};
use super::{CrystalGame, REFUSAL_LOG_CAP};
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::repl::check_script;
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

/// One click at (0, 0) solves this board: the centre is 1 + 2 and both
/// neighbours are 2 + 1, all mod 6 to 3.
fn one_click_from_solved() -> Board {
    board_with(&[((0, 0), 1), ((1, 0), 2), ((0, 1), 2)])
}

/// A `processmove` parameter map for `(x, y)`.
fn click(x: f64, y: f64) -> HashMap<String, SfsValue> {
    HashMap::from([
        ("x".to_string(), SfsValue::Number(x)),
        ("y".to_string(), SfsValue::Number(y)),
    ])
}

/// Each output as its `_cmd`, or `Victory` / `Failure` for the terminal ones.
fn labels(outputs: &[GameOutput]) -> Vec<String> {
    outputs
        .iter()
        .map(|output| match output {
            GameOutput::Send(map) => match &map["_cmd"] {
                SfsValue::String(s) => s.clone(),
                _ => panic!("_cmd is not a string"),
            },
            GameOutput::Victory => "Victory".to_string(),
            GameOutput::Failure => "Failure".to_string(),
        })
        .collect()
}

#[test]
fn goldens_check_clean() {
    for golden in [
        include_str!("../../../../tests/scripts/crystal_game/t1_solved.mgs"),
        include_str!("../../../../tests/scripts/crystal_game/t2_timeout.mgs"),
        include_str!("../../../../tests/scripts/crystal_game/t3_validation.mgs"),
    ] {
        assert_eq!(check_script(golden), Ok(()));
    }
}

#[test]
fn clock_does_not_run_before_the_first_click() {
    let mut game = game_on(Board::solved());
    for _ in 0..100 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(game.time_remaining, 16.0);
    assert_eq!(game.timer_state, 0);
    assert_eq!(game.outcome, None);
}

#[test]
fn timeout_comes_on_tick_64_after_the_first_click() {
    let mut game = game_on(Board::solved());
    let first = game.message("processmove", &click(13.0, 5.0));
    assert_eq!(labels(&first)[0], "timerupdate");
    for _ in 0..63 {
        assert!(game.tick().is_empty());
    }
    let out = game.tick();
    assert_eq!(
        labels(&out),
        vec!["failure".to_string(), "Failure".to_string()]
    );
    assert_eq!(game.outcome, Some("defeat"));
    assert_eq!(game.timer_state, 0);
    assert!(game.message("processmove", &click(0.0, 0.0)).is_empty());
}

#[test]
fn victory_stops_the_clock_so_no_failure_follows() {
    let mut game = game_on(one_click_from_solved());
    let out = game.message("processmove", &click(0.0, 0.0));
    assert_eq!(
        labels(&out),
        vec![
            "timerupdate",
            "updateslot",
            "updateslot",
            "updateslot",
            "victory",
            "Victory",
        ]
    );
    assert_eq!(game.outcome, Some("victory"));
    assert_eq!(game.timer_state, 0);
    // The clock would reach zero on tick 64 if the stop were missing.
    for _ in 0..100 {
        assert!(game.tick().is_empty());
    }
}

#[test]
fn client_victory_is_refused_and_the_server_decides() {
    let mut game = game_on(one_click_from_solved());
    assert!(game.message("victory", &HashMap::new()).is_empty());
    assert_eq!(game.outcome, None);
    let out = game.message("processmove", &click(0.0, 0.0));
    assert!(matches!(out.last(), Some(GameOutput::Victory)));
}

#[test]
fn needs_tick_is_on_until_the_game_ends() {
    let mut game = game_on(one_click_from_solved());
    // The session samples this once, before the first tick, so it is on
    // while idle; the idle tick does nothing.
    assert!(game.needs_tick());
    game.message("processmove", &click(0.0, 0.0));
    assert!(!game.needs_tick());
}

#[test]
fn abort_stops_the_clock() {
    let mut game = game_on(Board::solved());
    game.message("processmove", &click(13.0, 5.0));
    assert!(game.aborted().is_empty());
    assert!(!game.needs_tick());
    for _ in 0..64 {
        assert!(game.tick().is_empty());
    }
}

#[test]
fn crystal_game_is_server_validated_with_no_ability_bits() {
    assert_eq!(crate::minigame::games::validation("CrystalGame"), "server");
    assert_eq!(crate::minigame::games::ability_bits("CrystalGame"), 0);
}

#[test]
fn a_click_after_the_clock_deadline_loses_and_does_not_win() {
    // The tick count is still far from zero, but the 16 s of wall time has
    // passed: the late click is refused and the game is lost, not won.
    let mut game = game_on(one_click_from_solved());
    game.timer_state = 1;
    game.clock_deadline = Some(Instant::now() - Duration::from_secs(1));
    assert_eq!(
        labels(&game.message("processmove", &click(0.0, 0.0))),
        ["failure", "Failure"]
    );
    assert_eq!(game.outcome, Some("defeat"));
    assert!(game.board == one_click_from_solved());
}

#[test]
fn a_tick_after_the_clock_deadline_loses_even_with_ticks_left() {
    let mut game = game_on(Board::solved());
    game.timer_state = 1;
    game.time_remaining = 10.0;
    game.clock_deadline = Some(Instant::now() - Duration::from_secs(1));
    assert_eq!(labels(&game.tick()), ["failure", "Failure"]);
}

#[test]
fn junk_frames_do_not_extend_the_wait_for_a_first_click() {
    let mut game = game_on(one_click_from_solved());
    game.started();
    game.idle_deadline = Some(Instant::now() - Duration::from_secs(1));
    assert_eq!(
        labels(&game.message("bogus", &HashMap::new())),
        ["failure", "Failure"]
    );
    assert!(game.board == one_click_from_solved());
}

#[test]
fn refusals_are_logged_only_up_to_the_cap() {
    // Through `message` and `reject`, so the gate in `reject` is what is
    // counted: the WARN rows, not the refusal counter.
    let capture = crate::test_support::LogCapture::install();
    let mut game = game_on(Board::solved());
    for _ in 0..REFUSAL_LOG_CAP + 8 {
        assert!(game.message("bogus", &HashMap::new()).is_empty());
    }
    let rows = capture
        .all()
        .iter()
        .filter(|row| row.level == tracing::Level::WARN && row.message_contains("command refused"))
        .count();
    assert_eq!(rows, REFUSAL_LOG_CAP as usize);
}

#[test]
fn the_first_click_sets_the_16_s_deadline_and_started_sets_the_60_s_wait() {
    // Nothing here sets the deadlines by hand: they come from `started` and
    // from the first valid click, so deleting either assignment fails.
    let mut game = game_on(Board::solved());
    assert!(game.clock_deadline.is_none());
    assert!(game.idle_deadline.is_none());
    game.started();
    let idle = game.idle_deadline.expect("started sets the pre-click wait");
    assert!(idle > Instant::now() + Duration::from_secs(59));
    assert!(idle <= Instant::now() + Duration::from_secs(60));
    game.message("processmove", &click(14.0, 0.0));
    assert!(
        game.clock_deadline.is_none(),
        "a refused click must not start the clock"
    );
    game.message("processmove", &click(0.0, 0.0));
    let first = game
        .clock_deadline
        .expect("the first valid click sets the deadline");
    assert!(first > Instant::now() + Duration::from_secs(15));
    assert!(first <= Instant::now() + Duration::from_secs(16));
    game.message("processmove", &click(1.0, 0.0));
    assert_eq!(
        game.clock_deadline,
        Some(first),
        "a later click must not move the deadline"
    );
}
