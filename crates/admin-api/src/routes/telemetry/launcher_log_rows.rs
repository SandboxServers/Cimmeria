//! Launcher log rows: the filter and scrub for log uploads under a `logs` token.
//!
//! A log row is one line of the launcher's own log or of the game's log. Its
//! message is kept whole and scrubbed word by word (README D-LF9), so a chat
//! line survives where a whole-value scrub would blank it. The row shape is
//! closed: the fields of C-LF5 and no other key. A row that breaks a rule is
//! dropped and counted, never repaired.
//!
//! The module is pure: no I/O, no tracing, and no regex crate.

use serde_json::{Map, Value};

use super::dto::ClientNativeEvent;
use super::launcher_rows::scrub_value;

/// The exact targets a `logs` token may post (C-LF5). The same list as
/// `crates/launcher-core/src/telemetry/launcher_log_targets.txt`, in the same order.
pub(super) const LAUNCHER_LOG_TARGETS: [&str; 2] =
    ["client.launcher.log", "client.launcher.game_log"];
/// Longest kept message, in bytes, before the cut marker is appended (C-LF5).
const MAX_MESSAGE_BYTES: usize = 2_048;
/// Appended to a message that was cut.
const TRUNCATED: &str = "...[truncated]";

/// Drop every row that is not a valid log row and clean the kept ones: the target
/// must be one of [`LAUNCHER_LOG_TARGETS`], the level becomes `info` (the line's own
/// level travels in `log_level`), and the fields must be the closed set of C-LF5.
/// Returns the kept rows and how many rows were dropped.
pub(super) fn admit_log_rows(rows: Vec<ClientNativeEvent>) -> (Vec<ClientNativeEvent>, u64) {
    let total = rows.len();
    let kept: Vec<ClientNativeEvent> = rows.into_iter().filter_map(clean_row).collect();
    let dropped = (total - kept.len()) as u64;
    (kept, dropped)
}

/// The replay body of a launcher log row: when `target` is one of
/// [`LAUNCHER_LOG_TARGETS`] and `fields` holds a string `message`, that message,
/// removed from `fields` so it is not repeated in the attributes. `None` for any
/// other row, and for a log row with no string message (nothing is removed).
pub(super) fn take_log_body(target: &str, fields: &mut Map<String, Value>) -> Option<String> {
    if !LAUNCHER_LOG_TARGETS.contains(&target) || !fields.get("message")?.is_string() {
        return None;
    }
    match fields.remove("message") {
        Some(Value::String(message)) => Some(message),
        _ => None,
    }
}

/// Scrub each whitespace-separated word of a message that [`scrub_value`] flags
/// with [`REMOVED`], join the words with one space and cut the result to
/// [`MAX_MESSAGE_BYTES`] on a char boundary (D-LF9).
pub(super) fn scrub_words(message: &str) -> String {
    let words: Vec<String> = message.split_whitespace().map(scrub_value).collect();
    cut_message(words.join(" "))
}

/// The row when it is a valid log row, cleaned; `None` when it must be dropped.
fn clean_row(mut row: ClientNativeEvent) -> Option<ClientNativeEvent> {
    if !LAUNCHER_LOG_TARGETS.contains(&row.target.as_str()) {
        return None;
    }
    row.fields = clean_log_fields(&row.fields)?;
    row.level = "info".to_string();
    Some(row)
}

/// The cleaned fields, or `None` when a key is unknown, a value is bad, or the
/// required `message` and `upload_id` are missing.
fn clean_log_fields(fields: &Map<String, Value>) -> Option<Map<String, Value>> {
    if !fields.contains_key("message") || !fields.contains_key("upload_id") {
        return None;
    }
    let mut cleaned = Map::new();
    for (key, value) in fields {
        cleaned.insert(key.clone(), log_field(key, value)?);
    }
    Some(cleaned)
}

/// One cleaned field of a log row, or `None` when the key is not in C-LF5 or the
/// value breaks its rule.
fn log_field(key: &str, value: &Value) -> Option<Value> {
    match key {
        "message" => value.as_str().map(|text| Value::String(scrub_words(text))),
        "log_level" => keep(
            matches!(
                value.as_str(),
                Some("TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR")
            ),
            value,
        ),
        "log_target" => bounded(value, 64, |b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'.' | b'-')
        }),
        "file" => bounded(value, 64, |b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')
        }),
        "line_no" => keep(value.as_u64().is_some_and(|n| n >= 1), value),
        "upload_id" => keep(value.as_str().is_some_and(is_upload_id), value),
        "truncated" => keep(value.is_boolean(), value),
        "launcher_version" => bounded(value, 32, |b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'+' | b'-')
        }),
        "os" => bounded(value, 16, |b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
        }),
        _ => None,
    }
}

/// The value itself when `ok`.
fn keep(ok: bool, value: &Value) -> Option<Value> {
    ok.then(|| value.clone())
}

/// The value itself when it is a string of at most `max` bytes, every byte of which
/// passes `allowed`.
fn bounded(value: &Value, max: usize, allowed: fn(u8) -> bool) -> Option<Value> {
    let text = value.as_str()?;
    keep(text.len() <= max && text.bytes().all(allowed), value)
}

/// Exactly 16 lowercase hex digits (C-LF5).
fn is_upload_id(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Cut `text` to [`MAX_MESSAGE_BYTES`] on a char boundary and append [`TRUNCATED`]
/// when anything was cut.
fn cut_message(text: String) -> String {
    if text.len() <= MAX_MESSAGE_BYTES {
        return text;
    }
    let mut end = MAX_MESSAGE_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATED}", &text[..end])
}
