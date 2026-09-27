//! ORG-09: a team, command or officer line the base refuses before the
//! organization chat runs (the SS-00 flood limit, the D-SS12 text rules) is
//! also one `org.chat` outcome row on the `org` target, so every org chat
//! refusal is found under one event. Other channels get no `org.chat` row,
//! and no org line reaches the cell.

use std::time::{Duration, Instant};

use super::chat_flood_limit::Harness;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use cimmeria_wire::cell::chat::{CHAN_COMMAND, CHAN_OFFICER, CHAN_SQUAD, CHAN_TEAM};
use tracing::Level;

fn org_chat_rows(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", "org.chat"))
        .collect()
}

#[tokio::test]
async fn org_line_with_a_forbidden_character_logs_text_invalid() {
    for channel in [CHAN_TEAM, CHAN_COMMAND, CHAN_OFFICER] {
        let capture = LogCapture::install();
        let mut h = Harness::new(0);
        h.speak(channel, "bad\u{202E}line", Instant::now()).await;
        assert!(h.forwarded().is_empty(), "channel {channel}");
        let rows = org_chat_rows(&capture);
        assert_eq!(rows.len(), 1, "{rows:#?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::INFO);
        assert!(row.has_field("outcome", "rejected"), "{:?}", row.fields);
        assert!(row.has_field("reason", "text_invalid"), "{:?}", row.fields);
        assert!(row.has_field("channel", &channel.to_string()));
        assert!(row.has_field("player_id", "77"), "{:?}", row.fields);
        assert!(row.has_field("recipients", "0"), "{:?}", row.fields);
        assert!(row.has_field("text_units", "8"), "{:?}", row.fields);
    }
}

/// The same refusal on squad is the squad row's business only.
#[tokio::test]
async fn squad_line_refusal_logs_no_org_row() {
    let capture = LogCapture::install();
    let h = Harness::new(0);
    h.speak(CHAN_SQUAD, "bad\u{202E}line", Instant::now()).await;
    assert!(org_chat_rows(&capture).is_empty());
}

/// A flood on the team channel: the sixth line in a second is dropped and
/// logged once as `rate_limited`; the seventh, inside the feedback
/// throttle, is dropped without another row. The five that pass go to the
/// organization chat (here with no database: `no_db`), never to the cell.
#[tokio::test]
async fn team_flood_logs_rate_limited_once_per_notice() {
    let capture = LogCapture::install();
    let mut h = Harness::new(0);
    let t0 = Instant::now();
    for i in 0..7u64 {
        h.speak(CHAN_TEAM, "hi", t0 + Duration::from_millis(i * 100))
            .await;
    }
    assert!(h.forwarded().is_empty(), "org lines never reach the cell");
    let rows = org_chat_rows(&capture);
    let limited: Vec<_> = rows
        .iter()
        .filter(|r| r.has_field("reason", "rate_limited"))
        .collect();
    assert_eq!(limited.len(), 1, "{rows:#?}");
    let no_db = rows
        .iter()
        .filter(|r| r.has_field("reason", "no_db"))
        .count();
    assert_eq!(no_db, 5, "{rows:#?}");
}
