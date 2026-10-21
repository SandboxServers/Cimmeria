//! One launcher log line the adoption flow forwards, built as a `ClientNative`
//! event. `launcher_session` sends it at once under a `launcher` session; nothing
//! is queued on disk. The server scrubs it again, so this scrub is defence in
//! depth (README D-AU19, D-AU21).

use super::session::mint_launcher_version;
use cimmeria_launcher_core::telemetry::events::{ClientNativeEvent, TelemetryEvent};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Target of every forwarded launcher log line (SigNoz `client_target`).
pub const LAUNCHER_EVENT_TARGET: &str = "client.launcher.adoption";
/// A string field is cut to this many bytes (on a char boundary) and "..." is appended.
pub const LAUNCHER_FIELD_MAX: usize = 240;

static SEQ: AtomicU64 = AtomicU64::new(0);

/// One forwarded launcher log line, before it is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct LauncherRecord {
    pub ts_ms: i64,
    /// "info", "warn" or "error".
    pub level: String,
    pub fields: Map<String, Value>,
}

/// What to take out of every string before it leaves the process or reaches the log file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redaction {
    pub home: Option<String>,
    pub user: Option<String>,
}

impl Redaction {
    /// home: USERPROFILE, else HOME. user: USERNAME, else USER. Trimmed; empty or
    /// shorter than 2 characters is None.
    pub fn from_env() -> Self {
        Self {
            home: env_value(&["USERPROFILE", "HOME"]),
            user: env_value(&["USERNAME", "USER"]),
        }
    }

    /// Home folder -> `~`, then user-name path segments -> `<user>`.
    pub fn replace_home(&self, value: &str) -> String {
        let mut out = value.to_string();
        if let Some(home) = self.home.as_deref().filter(|h| !h.is_empty()) {
            // Longest form first: the Debug output doubles each backslash.
            for form in [
                home.replace('\\', "\\\\"),
                home.to_string(),
                home.replace('\\', "/"),
            ] {
                out = out.replace(&form, "~");
            }
        }
        match self.user.as_deref().filter(|u| !u.is_empty()) {
            Some(user) => mark_user_runs(&out, user),
            None => out,
        }
    }

    /// `replace_home`, then `"<path removed>"` if an absolute path is still left.
    pub fn scrub(&self, value: &str) -> String {
        let replaced = self.replace_home(value);
        if has_absolute_path(&replaced) {
            "<path removed>".to_string()
        } else {
            replaced
        }
    }
}

/// One `TelemetryEvent::ClientNative` per record, scrubbed. Pure: no I/O, no consent check.
pub fn launcher_events(records: &[LauncherRecord], redaction: &Redaction) -> Vec<TelemetryEvent> {
    records
        .iter()
        .map(|record| launcher_event(record, redaction))
        .collect()
}

fn launcher_event(record: &LauncherRecord, redaction: &Redaction) -> TelemetryEvent {
    let mut fields = Map::new();
    for (key, value) in &record.fields {
        let value = match value {
            Value::String(text) => Value::String(bound(&redaction.scrub(text))),
            other => other.clone(),
        };
        fields.insert(key.clone(), value);
    }
    fields.insert(
        "launcher_version".into(),
        Value::String(mint_launcher_version()),
    );
    fields.insert("os".into(), Value::String(std::env::consts::OS.into()));
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms: record.ts_ms,
        seq: SEQ.fetch_add(1, Ordering::Relaxed),
        target: LAUNCHER_EVENT_TARGET.into(),
        level: record.level.to_lowercase(),
        fields,
    })
}

/// Cuts `text` to `LAUNCHER_FIELD_MAX` bytes on a char boundary, then appends "...".
fn bound(text: &str) -> String {
    if text.len() <= LAUNCHER_FIELD_MAX {
        return text.to_string();
    }
    let mut end = LAUNCHER_FIELD_MAX;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &text[..end])
}

fn env_value(names: &[&str]) -> Option<String> {
    let value = names.iter().find_map(|name| std::env::var(name).ok())?;
    let value = value.trim();
    (value.chars().count() >= 2).then(|| value.to_string())
}

fn is_separator(c: char) -> bool {
    matches!(c, '/' | '\\' | '"' | '\'' | ':' | ';') || c.is_whitespace()
}

/// Every run between separators that equals `user` (ASCII case ignored) becomes `<user>`.
fn mark_user_runs(value: &str, user: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut run = String::new();
    for c in value.chars() {
        if is_separator(c) {
            out.push_str(&mark_run(&run, user));
            run.clear();
            out.push(c);
        } else {
            run.push(c);
        }
    }
    out.push_str(&mark_run(&run, user));
    out
}

fn mark_run(run: &str, user: &str) -> String {
    if run.eq_ignore_ascii_case(user) {
        "<user>".to_string()
    } else {
        run.to_string()
    }
}

/// An ASCII letter then `:\` or `:/`, or (after one leading `"`) a value that starts
/// with `\\` or `/`.
fn has_absolute_path(value: &str) -> bool {
    let drive = value
        .as_bytes()
        .windows(3)
        .any(|w| w[0].is_ascii_alphabetic() && w[1] == b':' && (w[2] == b'\\' || w[2] == b'/'));
    let rest = value.strip_prefix('"').unwrap_or(value);
    drive || rest.starts_with("\\\\") || rest.starts_with('/')
}
