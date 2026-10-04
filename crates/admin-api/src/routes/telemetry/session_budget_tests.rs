//! The runaway-client guard: per-session totals, the window budget, the
//! priority bypass and the bounded session table.

use serde_json::json;

use super::dto::{ClientNativeEvent, DebugLogEvent, KeyDumpEvent, TelemetryEvent};
use super::session_budget::{is_priority, replay_budgeted, SessionLedger, EVENTS_PER_WINDOW};
use crate::routes::dev_session::TokenClaims;

fn native(target: &str, level: &str) -> TelemetryEvent {
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: target.into(),
        level: level.into(),
        fields: json!({}).as_object().cloned().unwrap(),
    })
}

#[test]
fn under_budget_everything_is_replayed_and_counted() {
    let mut l = SessionLedger::new(10, 60, 16);
    for _ in 0..10 {
        assert!(l.admit("s", false, 0));
    }
    let t = l.totals("s").unwrap();
    assert_eq!((t.accepted_total, t.suppressed_total), (10, 0));
}

/// Over budget, ordinary events are suppressed and counted; priority
/// events still get through; the next window starts fresh.
#[test]
fn over_budget_only_priority_events_are_replayed() {
    let mut l = SessionLedger::new(10, 60, 16);
    for _ in 0..10 {
        l.admit("s", false, 0);
    }
    assert!(!l.admit("s", false, 30));
    assert!(!l.admit("s", false, 59));
    assert!(l.admit("s", true, 59), "a warn/error is never suppressed");
    let t = l.totals("s").unwrap();
    assert_eq!((t.accepted_total, t.suppressed_total), (11, 2));

    assert!(l.admit("s", false, 60), "a new window resets the budget");
    assert_eq!(l.totals("s").unwrap().accepted_total, 12);
}

/// One runaway session does not spend another's budget.
#[test]
fn sessions_have_separate_budgets() {
    let mut l = SessionLedger::new(5, 60, 16);
    for _ in 0..100 {
        l.admit("runaway", false, 0);
    }
    assert!(l.admit("quiet", false, 0));
    assert_eq!(l.totals("runaway").unwrap().suppressed_total, 95);
}

/// With every window ended, the least recently seen session makes room.
#[test]
fn the_session_table_evicts_the_least_recently_seen() {
    let mut l = SessionLedger::new(5, 60, 3);
    l.admit("a", false, 1);
    l.admit("b", false, 2);
    l.admit("c", false, 3);
    l.admit("a", false, 4);
    l.admit("d", false, 70);
    assert_eq!(l.len(), 3);
    assert!(!l.tracks("b"), "b was the least recently seen");
    assert!(l.tracks("a") && l.tracks("d"));
}

/// A full table of sessions inside their window evicts nobody: the
/// newcomer shares the overflow entry, and a session that already spent
/// its budget does not get a fresh one back.
#[test]
fn a_full_table_never_resets_a_live_sessions_budget() {
    let mut l = SessionLedger::new(5, 60, 2);
    for _ in 0..5 {
        assert!(l.admit("a", false, 0));
    }
    assert!(!l.admit("a", false, 0), "a has spent its window");
    l.admit("b", false, 1);
    // Table full, both windows open: c goes to the overflow entry.
    assert!(l.admit("c", false, 2));
    assert!(l.tracks("a") && l.tracks("b") && !l.tracks("c"));
    assert!(!l.admit("a", false, 3), "a's window is still spent");
    let t = l.totals("a").unwrap();
    assert_eq!((t.accepted_total, t.suppressed_total), (5, 2));
    assert_eq!(l.totals("c").unwrap().accepted_total, 1);
}

#[test]
fn priority_is_level_and_the_must_keep_families() {
    assert!(is_priority(&native("client.lua.pcall", "warn")));
    assert!(is_priority(&native("client.lua.pcall", "ERROR")));
    assert!(is_priority(&native("client.telemetry.rollup", "info")));
    assert!(is_priority(&native("client.telemetry.health", "info")));
    assert!(is_priority(&native("client.dll.attached", "info")));
    assert!(is_priority(&native("client.ability.recv", "info")));
    assert!(is_priority(&native("client.hooks.fingerprint", "info")));
    assert!(!is_priority(&native("client.lua.pcall", "debug")));
    assert!(is_priority(&native("client.entity.create", "info")));
    assert!(is_priority(&native("client.mercury.bundle", "debug")));
    assert!(is_priority(&native(
        "client.mercury.request_misparse",
        "info"
    )));
    assert!(is_priority(&native("client.mercury.unpack_fault", "info")));
    assert!(!is_priority(&native("client.cme.event", "debug")));
    assert!(is_priority(&TelemetryEvent::DebugLog(DebugLogEvent {
        ts_ms: 0,
        seq: 0,
        source_file: "f".into(),
        level: "Warning".into(),
        message: "m".into(),
    })));
    assert!(!is_priority(&TelemetryEvent::KeyDump(KeyDumpEvent {
        ts_ms: 0,
        seq: 0,
        source_file: "f".into(),
        key_b64: "k".into(),
    })));
}

/// The handler's path end to end, past HTTP: a runaway session's chunk is
/// replayed up to the budget, its warn still replays, and the totals say
/// how many were suppressed. Its own session id keeps it clear of the
/// process-wide ledger's other users.
#[test]
fn a_runaway_chunk_is_cut_at_the_budget_and_counted() {
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-runaway".into(),
        sid: "session-runaway-budget-test".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: None,
    };
    let extra = 250u64;
    let mut lines: Vec<String> = (0..EVENTS_PER_WINDOW + extra)
        .map(|i| {
            format!(
                r#"{{"type":"client_native","ts_ms":1,"seq":{i},"target":"client.lua.pcall","level":"debug"}}"#
            )
        })
        .collect();
    lines.push(
        r#"{"type":"client_native","ts_ms":1,"seq":0,"target":"client.lua.error","level":"warn"}"#
            .to_string(),
    );
    let (counts, totals) = replay_budgeted(&claims, &lines.join("\n"), 1_000).unwrap();
    assert_eq!(counts.parsed, EVENTS_PER_WINDOW + extra + 1);
    assert_eq!(
        counts.accepted,
        EVENTS_PER_WINDOW + 1,
        "the budget plus the warn"
    );
    assert_eq!(counts.suppressed, extra);
    assert_eq!(totals.suppressed_total, extra);
    assert_eq!(totals.accepted_total, EVENTS_PER_WINDOW + 1);
}
