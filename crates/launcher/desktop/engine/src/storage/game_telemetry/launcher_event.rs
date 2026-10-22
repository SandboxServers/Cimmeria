//! One launcher log line the adoption flow forwards, built as a `ClientNative`
//! event. `launcher_session` sends it at once under a `launcher` session; nothing
//! is queued on disk. The server scrubs it again, so this scrub is defence in
//! depth (README D-AU19, D-AU21).

use super::ip_scrub::scrub_ips;
use super::session::mint_launcher_version;
use cimmeria_launcher_core::telemetry::events::{ClientNativeEvent, TelemetryEvent};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Tracing target -> wire target. The only targets the launcher forwards (D-LT5).
pub const LAUNCHER_EVENT_TARGETS: [(&str, &str); 12] = [
    ("adoption", "client.launcher.adoption"),
    ("launcher.startup", "client.launcher.startup"),
    ("launcher.crash", "client.launcher.crash"),
    ("launcher.update", "client.launcher.update"),
    ("launcher.game_update", "client.launcher.game_update"),
    ("launcher.install", "client.launcher.install"),
    ("launcher.repair", "client.launcher.repair"),
    ("launcher.launch", "client.launcher.launch"),
    ("launcher.settings", "client.launcher.settings"),
    ("launcher.telemetry", "client.launcher.telemetry"),
    ("launcher.ui", "client.launcher.ui"),
    ("launcher.login", "client.launcher.login"),
];
/// A string field is cut to this many bytes (on a char boundary) and "..." is appended.
pub const LAUNCHER_FIELD_MAX: usize = 240;

static SEQ: AtomicU64 = AtomicU64::new(0);

/// The wire target for a tracing target, or None when the launcher does not forward it.
pub fn wire_target(tracing_target: &str) -> Option<&'static str> {
    LAUNCHER_EVENT_TARGETS
        .iter()
        .find(|(t, _)| *t == tracing_target)
        .map(|(_, w)| *w)
}

/// A closed, snake_case code for an I/O error kind (C6). Any kind not listed is `"other"`.
pub fn kind(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => "not_found",
        ErrorKind::PermissionDenied => "permission_denied",
        ErrorKind::AlreadyExists => "already_exists",
        ErrorKind::WouldBlock => "would_block",
        ErrorKind::InvalidInput => "invalid_input",
        ErrorKind::InvalidData => "invalid_data",
        ErrorKind::TimedOut => "timed_out",
        ErrorKind::WriteZero => "write_zero",
        ErrorKind::Interrupted => "interrupted",
        ErrorKind::UnexpectedEof => "unexpected_eof",
        ErrorKind::OutOfMemory => "out_of_memory",
        ErrorKind::StorageFull => "storage_full",
        ErrorKind::Other => "other",
        _ => "other",
    }
}

/// One forwarded launcher log line, before it is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct LauncherRecord {
    pub ts_ms: i64,
    /// "info", "warn" or "error".
    pub level: String,
    /// The wire target the forward layer resolved through `wire_target`.
    pub target: &'static str,
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

    /// The per-word scrub for game log text (LF-19 A5, A9): each whitespace-separated
    /// word that is a path becomes `<path removed>` and each long mixed token (a key
    /// or ticket) becomes `<removed>`; the other words and the spacing stay, then
    /// `replace_home`. A line with a path keeps its text, unlike `scrub`.
    pub fn scrub_words(&self, value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let mut word = String::new();
        for c in value.chars() {
            if c.is_whitespace() {
                out.push_str(&scrub_word(&word));
                word.clear();
                out.push(c);
            } else {
                word.push(c);
            }
        }
        out.push_str(&scrub_word(&word));
        self.replace_home(&out)
    }
}

/// One word of `scrub_words`: a path word, a long mixed token, or the word as is.
fn scrub_word(word: &str) -> String {
    if is_path_word(word) {
        "<path removed>".to_string()
    } else if is_long_token(word) {
        "<removed>".to_string()
    } else {
        word.to_string()
    }
}

/// `has_absolute_path`, or a word that contains `:\`, `:/` or `\\`, or starts with `/`.
fn is_path_word(word: &str) -> bool {
    has_absolute_path(word)
        || word.contains(":\\")
        || word.contains(":/")
        || word.contains("\\\\")
        || word.starts_with('/')
}

/// A word of 32 or more base64-style characters with at least one digit and one
/// letter: a session key, a ticket or a `key=` value (LF-19 A9).
fn is_long_token(word: &str) -> bool {
    word.len() >= 32
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '_' | '-'))
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
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
            Value::String(text) => Value::String(bound(&scrub_ips(&redaction.scrub(text)))),
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
        target: record.target.into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_target_maps_each_table_row() {
        for (tracing_target, wire) in LAUNCHER_EVENT_TARGETS {
            assert_eq!(wire_target(tracing_target), Some(wire));
        }
        assert_eq!(
            wire_target("launcher.install"),
            Some("client.launcher.install")
        );
        assert_eq!(wire_target("adoption"), Some("client.launcher.adoption"));
    }

    #[test]
    fn wire_target_refuses_unlisted_targets() {
        assert_eq!(wire_target("launcher.pipeline"), None);
        assert_eq!(wire_target("launcher"), None);
        assert_eq!(wire_target("cimmeria_launcher_engine"), None);
        assert_eq!(wire_target("client.launcher.install"), None);
    }

    #[test]
    fn wire_column_matches_the_shared_target_file() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../launcher-core/src/telemetry/launcher_targets.txt"
        );
        let text = std::fs::read_to_string(path).unwrap();
        assert!(!text.contains('\r'), "the target file must use LF endings");
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .collect();
        let wires: Vec<&str> = LAUNCHER_EVENT_TARGETS.iter().map(|(_, w)| *w).collect();
        assert_eq!(lines, wires);
    }

    #[test]
    fn launcher_events_write_the_record_target() {
        let record = LauncherRecord {
            ts_ms: 1,
            level: "INFO".into(),
            target: "client.launcher.install",
            fields: Map::new(),
        };
        let events = launcher_events(&[record], &Redaction::default());
        let TelemetryEvent::ClientNative(native) = &events[0] else {
            panic!("expected a ClientNative event");
        };
        assert_eq!(native.target, "client.launcher.install");
    }

    #[test]
    fn launcher_events_scrub_ip_literals_from_message_and_fields() {
        let mut fields = Map::new();
        fields.insert(
            "message".into(),
            Value::String("connect to 198.51.100.7:8081 failed".into()),
        );
        fields.insert("host_note".into(), Value::String("198.51.100.7".into()));
        let record = LauncherRecord {
            ts_ms: 1,
            level: "INFO".into(),
            target: "client.launcher.install",
            fields,
        };
        let events = launcher_events(&[record], &Redaction::default());
        let TelemetryEvent::ClientNative(native) = &events[0] else {
            panic!("expected a ClientNative event");
        };
        assert_eq!(
            native.fields["message"].as_str(),
            Some("connect to [ip]:8081 failed")
        );
        assert_eq!(native.fields["host_note"].as_str(), Some("[ip]"));
    }

    #[test]
    fn kind_names_the_listed_error_kinds() {
        use std::io::{Error, ErrorKind};
        assert_eq!(kind(&Error::from(ErrorKind::NotFound)), "not_found");
        assert_eq!(
            kind(&Error::from(ErrorKind::PermissionDenied)),
            "permission_denied"
        );
        assert_eq!(kind(&Error::from(ErrorKind::TimedOut)), "timed_out");
        assert_eq!(kind(&Error::from(ErrorKind::StorageFull)), "storage_full");
        assert_eq!(kind(&Error::from(ErrorKind::Other)), "other");
    }

    #[test]
    fn kind_maps_an_unlisted_error_kind_to_other() {
        let error = std::io::Error::from(std::io::ErrorKind::ConnectionReset);
        assert_eq!(kind(&error), "other");
    }
}
