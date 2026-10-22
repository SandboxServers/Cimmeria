//! Tests for the launcher log-row filter and per-word scrub.

use serde_json::{json, Value};

use super::dto::ClientNativeEvent;
use super::launcher_log_rows::*;
use super::launcher_rows::LAUNCHER_TARGETS;

const UPLOAD_ID: &str = "0123456789abcdef";

fn row(target: &str, fields: Value) -> ClientNativeEvent {
    serde_json::from_value(json!({
        "ts_ms": 1, "seq": 1, "target": target, "level": "warn", "fields": fields
    }))
    .unwrap()
}

fn good_fields(message: &str) -> Value {
    json!({ "message": message, "upload_id": UPLOAD_ID, "log_level": "INFO" })
}

fn message_of(row: &ClientNativeEvent) -> &str {
    row.fields["message"].as_str().unwrap()
}

#[test]
fn a_flagged_word_is_removed_and_the_rest_is_kept() {
    // Per word, not the whole value: the trailing word survives.
    assert_eq!(
        scrub_words(r"C:\Users\someone\x.txt failed"),
        "<removed> failed"
    );
}

#[test]
fn an_address_word_is_removed_and_its_neighbours_are_kept() {
    assert_eq!(
        scrub_words("connect 10.0.0.1:443 refused"),
        "connect <removed> refused"
    );
}

#[test]
fn a_long_message_is_cut_to_2048_bytes_with_the_marker() {
    let message = "lorem ".repeat(500);
    let scrubbed = scrub_words(&message);
    assert_eq!(scrubbed.len(), 2_048 + "...[truncated]".len());
    assert!(scrubbed.starts_with("lorem lorem"));
    assert!(scrubbed.ends_with("...[truncated]"));
}

#[test]
fn a_cut_never_splits_a_multibyte_char() {
    // Byte 2048 falls inside the two-byte `é` that starts at byte 2047.
    let message = format!("{}é{}", "x".repeat(2_047), "y".repeat(10));
    let scrubbed = scrub_words(&message);
    assert_eq!(scrubbed.len(), 2_047 + "...[truncated]".len());
    assert!(scrubbed.ends_with("...[truncated]"));
}

#[test]
fn a_game_chat_line_is_kept_unchanged() {
    let (kept, dropped) = admit_log_rows(vec![row(
        "client.launcher.game_log",
        good_fields("[Say] Bob: hello"),
    )]);
    assert_eq!(dropped, 0);
    assert_eq!(kept.len(), 1);
    assert_eq!(message_of(&kept[0]), "[Say] Bob: hello");
}

#[test]
fn a_row_without_upload_id_is_dropped() {
    let fields = json!({ "message": "hello", "log_level": "INFO" });
    let (kept, dropped) = admit_log_rows(vec![row("client.launcher.log", fields)]);
    assert!(kept.is_empty());
    assert_eq!(dropped, 1);
}

#[test]
fn an_upper_case_upload_id_is_dropped() {
    let fields = json!({ "message": "hello", "upload_id": "0123456789ABCDEF" });
    let (kept, dropped) = admit_log_rows(vec![row("client.launcher.log", fields)]);
    assert!(kept.is_empty());
    assert_eq!(dropped, 1);
}

#[test]
fn an_event_target_is_dropped() {
    let mut rows = vec![row("client.launcher.adoption", good_fields("hello"))];
    for target in LAUNCHER_TARGETS {
        rows.push(row(target, good_fields("hello")));
    }
    let (kept, dropped) = admit_log_rows(rows);
    assert!(kept.is_empty());
    assert_eq!(dropped, LAUNCHER_TARGETS.len() as u64 + 1);
}

#[test]
fn the_row_level_is_forced_to_info_and_log_level_is_kept() {
    let mut fields = good_fields("boom");
    fields["log_level"] = json!("ERROR");
    let (kept, _) = admit_log_rows(vec![row("client.launcher.log", fields)]);
    assert_eq!(kept[0].level, "info");
    assert_eq!(kept[0].fields["log_level"], "ERROR");
}

#[test]
fn a_file_with_a_slash_is_dropped() {
    let mut fields = good_fields("hello");
    fields["file"] = json!("logs/launcher.log");
    let (kept, dropped) = admit_log_rows(vec![row("client.launcher.log", fields)]);
    assert!(kept.is_empty());
    assert_eq!(dropped, 1);
}

#[test]
fn an_unknown_key_is_dropped() {
    let mut fields = good_fields("hello");
    fields["account"] = json!("someone");
    let (kept, dropped) = admit_log_rows(vec![row("client.launcher.log", fields)]);
    assert!(kept.is_empty());
    assert_eq!(dropped, 1);
}

/// The server list is the canonical file, line for line (C-LF5).
#[test]
fn log_targets_match_the_canonical_file() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../launcher-core/src/telemetry/launcher_log_targets.txt"
    ))
    .unwrap();
    let targets: Vec<&str> = text
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(targets, LAUNCHER_LOG_TARGETS);
}

#[test]
fn a_flagged_path_takes_the_name_words_after_its_space() {
    assert_eq!(
        scrub_words(r"C:\Users\Jane Doe\AppData\x.log failed"),
        "<removed> <removed> failed"
    );
    assert_eq!(scrub_words("[Say] Bob: hello"), "[Say] Bob: hello");
}

#[test]
fn control_and_invisible_characters_do_not_reach_the_body() {
    assert_eq!(
        scrub_words("ok\u{1b}[0m\u{0}end\u{202e}x\u{200b}y"),
        "ok [0m end x y"
    );
    assert_eq!(scrub_words("Привет 😀"), "Привет 😀");
}

#[test]
fn an_address_next_to_punctuation_is_removed() {
    assert_eq!(scrub_words("peer <1.2.3.4:5000> ok"), "peer <removed> ok");
}
