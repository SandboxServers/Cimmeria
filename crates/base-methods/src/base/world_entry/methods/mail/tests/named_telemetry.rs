//! Rule 6 guard for the mail router's events that need no database: the
//! caller's session names (`player_name`, `entity_name`, `account_name`) ride
//! next to the ids. The DB-backed events (`mail.sent`, `mail.cash_taken`,
//! `mail.cod_paid`, `mail.op_refused`) assert the same pairs in their own
//! live-DB tests through `assert_actor_names` and `assert_refused`.
//!
//! Fails if `Caller::identity` or the pairing on the event is dropped.

use std::time::Instant;

use super::packets::Client;
use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn no_pool_event_names_the_caller() {
    let capture = LogCapture::install();
    let c = Client::new(0x7300_1F01, 0x7300_1F02, 55_190, "NamedMailCaller");

    c.op(
        MailOp::RequestHeaders { b_archive: 0 },
        None,
        Instant::now(),
    )
    .await;

    let ev = capture
        .all()
        .into_iter()
        .find(|e| e.message_contains("Mail request: no DB pool available"))
        .expect("the no-pool event");
    assert_actor_names(&ev, "NamedMailCaller");
    assert!(
        ev.has_field("account_id", &0x7300_0001.to_string()),
        "{ev:?}"
    );
}

/// A caller whose session is gone is named by nothing: the name fields are
/// left off, not written as an empty string.
#[tokio::test]
async fn a_caller_without_a_session_has_no_names() {
    let capture = LogCapture::install();
    let c = Client::new(0x7300_1F03, 0x7300_1F04, 55_191, "GoneCaller");
    c.connected.lock().unwrap().clear();

    c.op(
        MailOp::RequestHeaders { b_archive: 0 },
        None,
        Instant::now(),
    )
    .await;

    let ev = capture
        .all()
        .into_iter()
        .find(|e| {
            e.message_contains("Mail request: no DB pool available")
                && e.has_field("entity_id", &0x7300_1F03.to_string())
        })
        .expect("the no-pool event");
    for key in ["player_name", "entity_name", "account_name"] {
        assert!(!ev.fields.contains_key(key), "{key} must be absent: {ev:?}");
    }
}
