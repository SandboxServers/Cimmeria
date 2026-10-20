//! Tests for the client-declared victory floor (D-MG13, MG-04).

use super::*;
use crate::test_support::LogCapture;
use tracing::Level;

fn has_victory(outputs: &[GameOutput]) -> bool {
    outputs.iter().any(|o| matches!(o, GameOutput::Victory))
}

#[tokio::test(start_paused = true)]
async fn victory_before_min_play_is_refused() {
    let mut game = PlaceholderGame::new("Hack");
    game.started();

    let outputs = game.message("victory", &HashMap::new());

    assert!(!has_victory(&outputs), "an instant victory must be refused");
    assert!(outputs.is_empty(), "a refused victory sends nothing");
}

#[tokio::test(start_paused = true)]
async fn victory_after_min_play_counts() {
    let mut game = PlaceholderGame::new("Hack");
    game.started();
    tokio::time::advance(CLIENT_DECLARED_MIN_PLAY).await;

    let outputs = game.message("victory", &HashMap::new());

    assert!(has_victory(&outputs), "a victory after 5 s must count");
}

#[tokio::test(start_paused = true)]
async fn victory_before_start_is_refused() {
    let mut game = PlaceholderGame::new("Hack");

    let outputs = game.message("victory", &HashMap::new());

    assert!(
        !has_victory(&outputs),
        "a victory before started() must be refused"
    );
    assert!(!victory_allowed(None, Instant::now()));
}

#[tokio::test(start_paused = true)]
async fn refused_victory_logs_warn_with_reason() {
    let capture = LogCapture::install();
    let mut game = PlaceholderGame::new("Hack");
    game.started();

    let _ = game.message("victory", &HashMap::new());

    let event = capture
        .find_message(
            Level::WARN,
            "Client-declared minigame victory refused: under the minimum play time",
        )
        .expect("a refused victory must log a WARN row");
    assert!(
        event.has_field("reason", "min_play_time"),
        "the refusal carries its reason: {event:#?}"
    );
    assert!(
        event.has_field("game", "Hack"),
        "the refusal names the game: {event:#?}"
    );
}

// ── shell countdown (MG-64) ──────────────────────────────────────────

/// The payload of the `Send` output at `index`, panicking on any other kind.
fn send_payload(outputs: &[GameOutput], index: usize) -> &HashMap<String, SfsValue> {
    match &outputs[index] {
        GameOutput::Send(payload) => payload,
        other => panic!("output {index} must be a Send, got {other:?}"),
    }
}

fn cmd_of(payload: &HashMap<String, SfsValue>) -> Option<&str> {
    match payload.get("_cmd") {
        Some(SfsValue::String(cmd)) => Some(cmd.as_str()),
        _ => None,
    }
}

#[tokio::test(start_paused = true)]
async fn started_sends_the_timer_for_the_shell() {
    let mut game = PlaceholderGame::new("Hack");

    let outputs = game.started();

    assert_eq!(
        outputs.len(),
        2,
        "started sends fullgamestate then timerupdate"
    );
    let full = send_payload(&outputs, 0);
    assert_eq!(cmd_of(full), Some("fullgamestate"));
    assert!(
        matches!(full.get("timerState"), Some(SfsValue::Number(n)) if *n == 0.0),
        "fullgamestate carries timerState 0: {full:?}"
    );
    assert!(
        matches!(full.get("timeRemaining"), Some(SfsValue::Number(n)) if *n == 60.0),
        "fullgamestate carries timeRemaining 60: {full:?}"
    );
    let update = send_payload(&outputs, 1);
    assert_eq!(cmd_of(update), Some("timerupdate"));
    assert!(
        matches!(update.get("timerState"), Some(SfsValue::Number(n)) if *n == 1.0),
        "timerupdate starts the timer: {update:?}"
    );
    assert!(
        matches!(update.get("timeRemaining"), Some(SfsValue::Number(n)) if *n == 60.0),
        "timerupdate carries timeRemaining 60: {update:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn shell_needs_ticks_for_its_countdown() {
    let game = PlaceholderGame::new("Hack");

    assert!(
        game.needs_tick(),
        "the countdown is driven by session ticks"
    );
}

#[tokio::test(start_paused = true)]
async fn countdown_sends_nothing_for_239_ticks_then_fails_on_the_240th() {
    let mut game = PlaceholderGame::new("Hack");
    game.started();

    for tick in 1..240 {
        assert!(
            game.tick().is_empty(),
            "tick {tick} is inside the 60 s countdown and sends nothing"
        );
    }
    let outputs = game.tick();

    assert_eq!(
        outputs.len(),
        2,
        "the expiring tick sends failure then Failure"
    );
    assert_eq!(cmd_of(send_payload(&outputs, 0)), Some("failure"));
    assert!(
        matches!(outputs[1], GameOutput::Failure),
        "the expiring tick returns GameOutput::Failure"
    );
}

#[tokio::test(start_paused = true)]
async fn countdown_fails_only_once() {
    let mut game = PlaceholderGame::new("Hack");
    game.started();
    for _ in 0..240 {
        game.tick();
    }

    assert!(
        game.tick().is_empty(),
        "a tick after the timeout sends nothing"
    );
}

#[tokio::test(start_paused = true)]
async fn countdown_does_not_run_before_start() {
    let mut game = PlaceholderGame::new("Hack");

    for _ in 0..240 {
        assert!(game.tick().is_empty(), "no countdown before started()");
    }
}

#[tokio::test(start_paused = true)]
async fn victory_at_4_seconds_is_still_refused() {
    let mut game = PlaceholderGame::new("Hack");
    game.started();
    tokio::time::advance(Duration::from_secs(4)).await;

    let outputs = game.message("victory", &HashMap::new());

    assert!(
        !has_victory(&outputs),
        "a victory at 4 s is under the 5 s floor"
    );
    assert!(outputs.is_empty(), "a refused victory sends nothing");
}
