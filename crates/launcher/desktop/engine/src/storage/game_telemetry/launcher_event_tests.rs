//! Launcher event shape and scrub rules. The scrub is the client half of the
//! launcher privacy rules; the server scrubs again.
use super::launcher_event::*;
use super::transport::TelemetryEvent;
use serde_json::{json, Map, Value};

fn record(level: &str, fields: Value) -> LauncherRecord {
    LauncherRecord {
        ts_ms: 1_000,
        level: level.to_string(),
        target: "client.launcher.adoption",
        fields: fields.as_object().cloned().unwrap_or_else(Map::new),
    }
}

fn alice() -> Redaction {
    Redaction {
        home: Some(r"C:\Users\alice".into()),
        user: Some("alice".into()),
    }
}

#[test]
fn events_carry_the_launcher_target_and_build_fields() {
    let events = launcher_events(
        &[record("INFO", json!({"step": "mode_chosen"}))],
        &Redaction::default(),
    );
    assert_eq!(events.len(), 1);
    let TelemetryEvent::ClientNative(native) = &events[0] else {
        panic!("expected a ClientNative event");
    };
    assert_eq!(native.target, "client.launcher.adoption");
    assert_eq!(native.level, "info");
    assert_eq!(native.fields["step"], "mode_chosen");
    assert_eq!(native.fields["os"], std::env::consts::OS);
    assert!(native.fields.contains_key("launcher_version"));
}

#[test]
fn two_records_get_two_seq_values() {
    let events = launcher_events(
        &[record("info", json!({})), record("warn", json!({}))],
        &Redaction::default(),
    );
    let seq = |event: &TelemetryEvent| match event {
        TelemetryEvent::ClientNative(native) => native.seq,
        _ => panic!("expected a ClientNative event"),
    };
    assert_ne!(seq(&events[0]), seq(&events[1]));
}

#[test]
fn string_fields_are_scrubbed() {
    let long = "a".repeat(300);
    let events = launcher_events(
        &[record(
            "info",
            json!({
                "path": "later.txt",
                "detail": r"C:\Users\alice\Games\SGW",
                "other": r"D:\SGW\Binaries",
                "unc": r"\\server\share\x",
                "seg": "Games/alice/SGW",
                "n": 2,
                "long": long,
            }),
        )],
        &alice(),
    );
    let TelemetryEvent::ClientNative(native) = &events[0] else {
        panic!("expected a ClientNative event");
    };
    let f = &native.fields;
    assert_eq!(f["path"], "later.txt");
    assert_eq!(f["detail"], r"~\Games\SGW");
    assert_eq!(f["other"], "<path removed>");
    assert_eq!(f["unc"], "<path removed>");
    assert_eq!(f["seg"], "Games/<user>/SGW");
    assert_eq!(f["n"], 2);
    assert_eq!(f["long"], format!("{}...", "a".repeat(240)));
}

#[test]
fn replace_home_and_scrub_cases() {
    let home = alice();
    assert_eq!(home.replace_home(r"x C:\Users\alice\a"), r"x ~\a");
    assert_eq!(home.replace_home("C:/Users/alice/a"), "~/a");
    assert_eq!(home.replace_home(r"C:\\Users\\alice\\a"), r"~\\a");

    let none = Redaction {
        home: None,
        user: None,
    };
    assert_eq!(none.replace_home(r"C:\Users\alice\a"), r"C:\Users\alice\a");
    let empty = Redaction {
        home: Some(String::new()),
        user: Some(String::new()),
    };
    assert_eq!(empty.replace_home(r"C:\Users\alice\a"), r"C:\Users\alice\a");

    let bob = Redaction {
        home: None,
        user: Some("bob".into()),
    };
    assert_eq!(bob.replace_home("Bob/x"), "<user>/x");
    assert_eq!(bob.replace_home("bobcat.upk"), "bobcat.upk");

    assert_eq!(
        bob.scrub("hard_link:later.txt; link:(location)"),
        "hard_link:later.txt; link:(location)"
    );
    assert_eq!(bob.scrub("/home/bob/x"), "<path removed>");
    assert_eq!(bob.scrub("\"/etc/x\""), "<path removed>");
    assert_eq!(bob.scrub("."), ".");
}

/// LF-19 A9: a long token is removed when punctuation sits next to it, and the
/// punctuation stays. Only the token run is replaced.
#[test]
fn a_long_token_next_to_punctuation_is_removed_and_the_punctuation_kept() {
    let none = Redaction::default();
    let key = "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVowMTIzNDU2Nzg5";
    assert_eq!(
        none.scrub_words(&format!("{{\"key\":\"{key}\"}} done")),
        "{\"key\":\"<removed>\"} done"
    );
    assert_eq!(none.scrub_words(&format!("(ticket={key})")), "(<removed>)");
    assert_eq!(none.scrub_words(&format!("'{key}',")), "'<removed>',");
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N";
    assert_eq!(none.scrub_words(&format!("token={jwt} ok")), "<removed> ok");
}

/// LF-19 A9: a 31-character mixed run and a 40-character all-letter run are kept.
#[test]
fn a_short_or_all_letter_run_is_kept() {
    let none = Redaction::default();
    let short = "abcdefghij0123456789klmnopqrstu";
    let letters = "abcdefghijabcdefghijabcdefghijabcdefghij";
    assert_eq!(none.scrub_words(short), short);
    assert_eq!(none.scrub_words(letters), letters);
}
