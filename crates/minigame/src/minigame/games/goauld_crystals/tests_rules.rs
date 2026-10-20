//! Clock, timeout and reset tests (spec sections 5, 6 and 8): the countdown
//! per tick, pause and reverse, the monotonic deadline, the timeout, the
//! `processreset` and the refusals after the end.
//!
//! The tutorial board (`seed 1`, difficulty 2) is used throughout, so the
//! clock starts at 16 s. The deadline tests run on paused tokio time, so
//! `advance` moves the monotonic clock without any sleep.

use std::collections::HashMap;
use std::time::Duration;

use super::GoauldCrystalsGame;
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::protocol::SfsValue;
use crate::minigame::session::MinigameSession;
use crate::test_support::LogCapture;

fn make_session(abilities_mask: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "GoauldCrystals".to_string(),
        difficulty: 2,
        tech_competency: 1,
        seed: 1,
        abilities_mask,
        intelligence: 0,
        player_level: 1,
        ticket: String::new(),
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        player_name: None,
        client_ip: None,
        created_at: tokio::time::Instant::now(),
        connected: false,
        cancel: tokio::sync::watch::channel(false).0,
    }
}

/// A game that has joined (`started`), so the board is the tutorial board.
fn started(abilities_mask: u32) -> GoauldCrystalsGame {
    let mut game = GoauldCrystalsGame::new(&make_session(abilities_mask));
    game.started();
    game
}

/// Sends `playfieldclick` with the given coordinates.
fn click(game: &mut GoauldCrystalsGame, x: f64, y: f64) -> Vec<GameOutput> {
    let params = HashMap::from([
        ("x".to_string(), SfsValue::Number(x)),
        ("y".to_string(), SfsValue::Number(y)),
    ]);
    game.message("playfieldclick", &params)
}

/// One name per output: the `_cmd` of a message, or the result variant.
fn names(out: &[GameOutput]) -> Vec<String> {
    out.iter()
        .map(|output| match output {
            GameOutput::Send(map) => map["_cmd"].as_str().expect("_cmd is a string").to_string(),
            GameOutput::Victory => "Victory".to_string(),
            GameOutput::Failure => "Failure".to_string(),
        })
        .collect()
}

#[test]
fn a_click_starts_the_clock_and_each_tick_takes_a_quarter_second() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    assert_eq!(game.timer_state, 1);
    assert_eq!(game.max_time, 16.0);
    for _ in 0..8 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(game.time_remaining, 14.0);
}

#[test]
fn sixty_four_ticks_end_the_game_in_failure() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    for _ in 0..63 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(names(&game.tick()), ["failure", "Failure"]);
    assert_eq!(game.outcome, Some("failure"));
    assert_eq!(game.timer_state, 0);
    assert!(!game.needs_tick());
}

#[test]
fn a_pause_and_a_reverse_each_end_with_one_timerupdate() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    game.tick();
    let before = game.time_remaining;
    game.timer_state = 2;
    game.timer_ticks_left = 20;
    for _ in 0..19 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(names(&game.tick()), ["timerupdate"]);
    assert_eq!(game.timer_state, 1);
    assert_eq!(game.time_remaining, before);
    game.timer_state = 3;
    game.timer_ticks_left = 20;
    for _ in 0..19 {
        assert!(game.tick().is_empty());
    }
    assert_eq!(names(&game.tick()), ["timerupdate"]);
    assert_eq!(game.time_remaining, before + 5.0);
}

#[tokio::test(start_paused = true)]
async fn a_click_after_the_deadline_is_refused_before_any_tick() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    tokio::time::advance(Duration::from_secs(16)).await;
    // No tick has run, so only the deadline can end the game here.
    assert_eq!(game.time_remaining, 16.0);
    let out = click(&mut game, 1.0, 0.0);
    assert_eq!(names(&out), ["failure", "Failure"]);
    assert_eq!(game.outcome, Some("failure"));
    // The click was not acted on: the selection from the first click stays.
    assert_eq!(game.selected, Some((0, 0)));
}

#[tokio::test(start_paused = true)]
async fn a_tick_past_the_deadline_ends_the_game_even_when_the_countdown_lags() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    tokio::time::advance(Duration::from_secs(16)).await;
    assert_eq!(game.time_remaining, 16.0);
    assert_eq!(names(&game.tick()), ["failure", "Failure"]);
}

#[tokio::test(start_paused = true)]
async fn a_game_never_clicked_is_lost_after_the_pre_click_limit() {
    // The clock is stopped before the first click, so only the pre-click
    // limit can end this game: nothing before 60 s, the loss at 61 s.
    let mut game = started(0);
    tokio::time::advance(Duration::from_secs(59)).await;
    assert!(game.tick().is_empty());
    assert_eq!(game.outcome, None);
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_eq!(names(&game.tick()), ["failure", "Failure"]);
    assert_eq!(game.outcome, Some("failure"));
}

#[tokio::test(start_paused = true)]
async fn a_pause_holds_no_deadline_and_a_resume_sets_a_fresh_one() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    game.timer_state = 2;
    game.timer_ticks_left = 3;
    assert!(game.tick().is_empty());
    assert!(game.deadline.is_none());
    // A long pause must not count against the clock once it resumes.
    tokio::time::advance(Duration::from_secs(100)).await;
    assert!(game.tick().is_empty());
    assert_eq!(names(&game.tick()), ["timerupdate"]);
    assert!(game.deadline.is_some());
    tokio::time::advance(Duration::from_millis(15_500)).await;
    assert!(game.tick().is_empty());
    tokio::time::advance(Duration::from_millis(500)).await;
    assert_eq!(names(&game.tick()), ["failure", "Failure"]);
}

#[test]
fn processreset_is_refused_and_leaves_the_clock_and_the_ability_bits_alone() {
    let mut game = started(0b100);
    click(&mut game, 0.0, 0.0);
    game.ability_bitfield = 0;
    let out = game.message("processreset", &HashMap::new());
    assert!(out.is_empty());
    assert_eq!(game.timer_state, 1);
    assert_eq!(game.selected, Some((0, 0)));
    assert_eq!(game.ability_bitfield, 0);
    assert!(game.deadline.is_some());
}

/// A reset after the wall-clock end is still refused: the deadline ends the
/// game first, and the clock is not refilled. Moving the `processreset` arm
/// above the deadline check gives no output here, and this fails.
#[tokio::test(start_paused = true)]
async fn a_reset_after_the_deadline_is_refused_with_a_failure() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    tokio::time::advance(Duration::from_secs(16)).await;
    // No tick has run, so only the deadline can end the game here.
    let out = game.message("processreset", &HashMap::new());
    assert_eq!(names(&out), ["failure", "Failure"]);
    assert_eq!(game.outcome, Some("failure"));
    assert_eq!(game.time_remaining, 16.0);
}

#[test]
fn nothing_is_acted_on_after_a_timeout() {
    let mut game = started(0);
    click(&mut game, 0.0, 0.0);
    game.time_remaining = 0.25;
    assert_eq!(names(&game.tick()), ["failure", "Failure"]);
    let none = HashMap::new();
    assert!(game.message("processreset", &none).is_empty());
    assert!(game.message("cheat", &none).is_empty());
    assert!(click(&mut game, 1.0, 0.0).is_empty());
    assert_eq!(game.selected, Some((0, 0)));
}

#[test]
fn a_reset_is_refused_after_a_victory() {
    let mut game = started(0);
    game.outcome = Some("victory");
    assert!(game.message("processreset", &HashMap::new()).is_empty());
    assert_eq!(game.refusals, 1);
}

/// Refusals are logged in full for the first eight, then one in a hundred.
/// Dropping the sampling logs all 1000 and fails this.
#[test]
fn refused_commands_are_logged_in_full_then_sampled() {
    let capture = LogCapture::install();
    let mut game = started(0);
    for _ in 0..1000 {
        assert!(game.message("cheat", &HashMap::new()).is_empty());
    }
    assert_eq!(game.refusals, 1000);
    let rows = capture
        .all()
        .iter()
        .filter(|row| row.message_contains("GoauldCrystals: command refused"))
        .count();
    // The first eight, then the hundreds 100 to 1000.
    assert_eq!(rows, 8 + 10);
}

/// A bad `playfieldclick` is refused through `warn_refused`, so it is counted
/// and sampled like every other refusal. Reverting it to a bare warn leaves
/// `refusals` at 0 and logs all 1000 rows, and this fails.
#[test]
fn bad_click_coordinates_are_counted_and_sampled() {
    let capture = LogCapture::install();
    let mut game = started(0);
    for _ in 0..1000 {
        assert!(click(&mut game, -1.0, 0.0).is_empty());
    }
    assert_eq!(game.refusals, 1000);
    let rows = capture
        .all()
        .iter()
        .filter(|row| row.message_contains("GoauldCrystals: command refused"))
        .count();
    assert_eq!(rows, 8 + 10);
}

#[test]
fn the_ticker_is_requested_from_the_start_until_the_end() {
    let mut game = started(0);
    assert!(
        game.needs_tick(),
        "the loop samples this before the first click"
    );
    click(&mut game, 0.0, 0.0);
    assert!(game.needs_tick());
    game.time_remaining = 0.25;
    game.tick();
    assert!(!game.needs_tick());
}
