//! ORG-04: a squad chat line the base refuses before the cell forward (the
//! SS-00 flood limit, the D-SS12 text rules) is also one `squad.chat`
//! outcome row on the `squad` target, so every squad chat refusal is found
//! under one event. Other channels get no `squad.chat` row.

use std::time::{Duration, Instant};

use super::chat_flood_limit::Harness;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use cimmeria_wire::cell::chat::{CHAN_SAY, CHAN_SQUAD};
use tracing::Level;

fn squad_chat_rows(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "squad" && c.has_field("event", "squad.chat"))
        .collect()
}

#[tokio::test]
async fn squad_line_with_a_forbidden_character_logs_text_invalid() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    h.speak(CHAN_SQUAD, "bad\u{202E}line", Instant::now()).await;

    assert!(
        h.forwarded().is_empty(),
        "a refused line never reaches the cell"
    );
    let rows = squad_chat_rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row.level, Level::INFO);
    assert!(row.has_field("outcome", "rejected"), "{:?}", row.fields);
    assert!(row.has_field("reason", "text_invalid"), "{:?}", row.fields);
    assert!(row.has_field("player_id", "77"), "{:?}", row.fields);
    assert!(row.has_field("recipients", "0"), "{:?}", row.fields);
    assert!(row.has_field("text_units", "8"), "{:?}", row.fields);
}

/// The same refusal on say is chat's business only.
#[tokio::test]
async fn say_line_refusal_logs_no_squad_row() {
    let capture = LogCapture::install();
    let h = Harness::new(0);
    h.speak(CHAN_SAY, "bad\u{202E}line", Instant::now()).await;
    assert!(squad_chat_rows(&capture).is_empty());
}

/// A flood on the squad channel: the sixth line in a second is dropped and
/// logged once as `rate_limited`; the seventh, inside the five-second
/// feedback throttle, is dropped without another row.
#[tokio::test]
async fn squad_flood_logs_rate_limited_once_per_notice() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    let t0 = Instant::now();
    for i in 0..7u64 {
        h.speak(CHAN_SQUAD, "hi", t0 + Duration::from_millis(i * 100))
            .await;
    }
    assert_eq!(h.forwarded().len(), 5);
    let rows = squad_chat_rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(
        rows[0].has_field("reason", "rate_limited"),
        "{:?}",
        rows[0].fields
    );
    assert!(rows[0].has_field("outcome", "rejected"));
}
