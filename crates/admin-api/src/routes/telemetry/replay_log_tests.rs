//! The replayed body of a launcher log row (C-LF5, D-LU4): its message, once.

use serde_json::json;

use super::dto::ClientNativeEvent;
use super::replay_native::replay_client_native;
use super::replay_tests::{capture, claims};

fn row(target: &str, fields: serde_json::Value) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms: 1_700_000_000_000,
        seq: 3,
        target: target.into(),
        level: "info".into(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

/// **A replayed log row's body is its message, and the message is not
/// repeated in the attributes.** The other fields stay attributes.
#[test]
fn a_log_rows_body_is_its_message() {
    let rows = capture(|| {
        replay_client_native(
            &log_claims(),
            row(
                "client.launcher.log",
                json!({
                    "message": "adoption finished",
                    "upload_id": "0123456789abcdef",
                    "log_level": "INFO"
                }),
            ),
        );
    });
    let r = rows.iter().find(|r| r.target == "client.native").unwrap();
    assert_eq!(r.fields["message"], "adoption finished");
    assert_eq!(r.fields["client_target"], "client.launcher.log");
    assert!(r.fields["fields"].contains("0123456789abcdef"));
    assert!(r.fields["fields"].contains("INFO"));
    assert!(
        !r.fields["fields"].contains("adoption finished"),
        "the message is the body, not an attribute too: {}",
        r.fields["fields"]
    );
}

/// **The game log target takes the same shape.**
#[test]
fn a_game_log_rows_body_is_its_message() {
    let rows = capture(|| {
        replay_client_native(
            &log_claims(),
            row(
                "client.launcher.game_log",
                json!({ "message": "zone loaded", "upload_id": "0123456789abcdef" }),
            ),
        );
    });
    let r = rows.iter().find(|r| r.target == "client.native").unwrap();
    assert_eq!(r.fields["message"], "zone loaded");
}

/// **An event row keeps its target name as the body** and its `message` field,
/// if it has one, stays an attribute.
#[test]
fn an_event_rows_body_stays_its_target() {
    let rows = capture(|| {
        replay_client_native(
            &claims(Some("launcher")),
            row("client.launcher.adoption", json!({ "message": "kept" })),
        );
    });
    let r = rows.iter().find(|r| r.target == "client.native").unwrap();
    assert_eq!(r.fields["message"], "client.launcher.adoption");
    assert!(r.fields["fields"].contains("kept"));
}

/// **A log target from a non-log session is not replayed** (C-LF4, second guard).
#[test]
fn a_log_target_from_a_non_log_session_is_dropped() {
    for c in [claims(Some("launcher")), claims(None)] {
        let rows = capture(|| {
            replay_client_native(
                &c,
                row(
                    "client.launcher.log",
                    json!({ "message": "forged", "upload_id": "0123456789abcdef" }),
                ),
            );
        });
        assert!(rows.iter().all(|r| r.target != "client.native"), "{rows:?}");
    }
}

/// A log session's claims: a launcher token with the logs purpose.
fn log_claims() -> crate::routes::dev_session::TokenClaims {
    crate::routes::dev_session::TokenClaims {
        scope: vec!["telemetry.launcher.write".into()],
        purpose: Some("logs".into()),
        ..claims(Some("launcher"))
    }
}
