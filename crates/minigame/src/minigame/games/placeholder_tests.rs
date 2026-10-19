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
