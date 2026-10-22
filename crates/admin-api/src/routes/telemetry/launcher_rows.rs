//! Launcher-session rows: the filter and scrub for uploads under a launcher token.
//!
//! A launcher session is minted without a game and without a credential, so its
//! rows are held to an exact event list and a scalar field allow-list, scrubbed
//! of IPs, accounts and paths, and kept out of lifted attributes and entity
//! naming. See README D-AU20 and the security review R9-R14.
//!
//! The module is pure: no I/O, no tracing, and no regex crate.

use std::net::{IpAddr, SocketAddr};

use serde_json::{Map, Value};

use super::dto::TelemetryEvent;

/// The exact launcher event names a launcher token may post (R10). The same list as
/// `crates/launcher-core/src/telemetry/launcher_targets.txt`, in the same order (D-LT5).
pub(super) const LAUNCHER_TARGETS: [&str; 12] = [
    "client.launcher.adoption",
    "client.launcher.startup",
    "client.launcher.crash",
    "client.launcher.update",
    "client.launcher.game_update",
    "client.launcher.install",
    "client.launcher.repair",
    "client.launcher.launch",
    "client.launcher.settings",
    "client.launcher.telemetry",
    "client.launcher.ui",
    "client.launcher.login",
];
/// Fields kept per row (R12).
pub(super) const MAX_FIELDS: usize = 16;
/// Longest field key, in bytes (R12).
pub(super) const MAX_KEY_BYTES: usize = 32;
/// Longest kept string value, in bytes (R12).
pub(super) const MAX_VALUE_BYTES: usize = 256;
/// Keys `replay_native::LiftedFields` lifts into attributes (R13) and identity keys (defence in depth).
pub(super) const DENIED_KEYS: [&str; 29] = [
    "method_index",
    "msg_id",
    "type_id",
    "entity_id",
    "target_id",
    "source_id",
    "pet_id",
    "ability_id",
    "item_type_id",
    "address",
    "level_name",
    "dll_version",
    "usable",
    "rollup_target",
    "count",
    "ip",
    "addr",
    "peer",
    "host",
    "hostname",
    "account",
    "account_name",
    "user",
    "username",
    "email",
    "machine_id",
    "install_id",
    "session_id",
    "client_pid",
];
/// What a scrubbed value becomes (R14).
pub(super) const REMOVED: &str = "<removed>";

/// Drop every row that is not an allowed `ClientNative` row, keep at most `max_rows`,
/// clean the kept rows in place. Returns how many rows were dropped.
pub(super) fn admit_launcher_rows(events: &mut Vec<TelemetryEvent>, max_rows: usize) -> u64 {
    let total = events.len();
    let mut kept = Vec::new();
    for event in events.drain(..) {
        if kept.len() >= max_rows {
            break;
        }
        if let TelemetryEvent::ClientNative(mut row) = event {
            if !allowed_target(&row.target) {
                continue;
            }
            row.level = clean_level(&row.level);
            clean_fields(&mut row.fields);
            kept.push(TelemetryEvent::ClientNative(row));
        }
    }
    *events = kept;
    (total - events.len()) as u64
}

/// Whether `target` is exactly one of [`LAUNCHER_TARGETS`].
pub(super) fn allowed_target(target: &str) -> bool {
    LAUNCHER_TARGETS.contains(&target)
}

/// The row's level, lower-cased; anything but `info`, `warn` or `error` is `info` (R11).
fn clean_level(level: &str) -> String {
    let lower = level.to_ascii_lowercase();
    let known = matches!(lower.as_str(), "info" | "warn" | "error");
    if known {
        lower
    } else {
        "info".to_string()
    }
}

/// Keep the allowed scalar fields of a row, in place, up to [`MAX_FIELDS`] (R12, R13).
pub(super) fn clean_fields(fields: &mut Map<String, Value>) {
    let mut kept = Map::new();
    for (key, value) in std::mem::take(fields) {
        if kept.len() >= MAX_FIELDS {
            break;
        }
        if !key_allowed(&key) {
            continue;
        }
        let cleaned = match value {
            Value::String(s) => Value::String(clean_string(&s)),
            Value::Bool(_) | Value::Number(_) => value,
            _ => continue,
        };
        kept.insert(key, cleaned);
    }
    *fields = kept;
}

/// A key is `^[a-z][a-z0-9_]{0,31}$` and not in [`DENIED_KEYS`].
fn key_allowed(key: &str) -> bool {
    let bytes = key.as_bytes();
    let shape = (1..=MAX_KEY_BYTES).contains(&bytes.len())
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_');
    shape && !DENIED_KEYS.contains(&key)
}

/// Control characters become spaces, other non-printable characters `?`, then the
/// value is scrubbed and cut to [`MAX_VALUE_BYTES`]. The mapped text is ASCII, so the cut is on a char boundary.
fn clean_string(value: &str) -> String {
    let mapped: String = value
        .chars()
        .map(|c| {
            if c.is_control() {
                ' '
            } else if (' '..='~').contains(&c) {
                c
            } else {
                '?'
            }
        })
        .collect();
    if scrub_value(&mapped) == REMOVED {
        return REMOVED.to_string();
    }
    mapped.chars().take(MAX_VALUE_BYTES).collect()
}

/// [`REMOVED`] if the value, compared without ASCII case, looks like an IP, a
/// drive or UNC path, a URL, an e-mail address or a `%VAR%`; otherwise the value (R14).
pub(super) fn scrub_value(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    // Each piece between `:`, `;`, `,`, `=` and whitespace is checked too, so
    // `ip:1.2.3.4` and `id=<guid>` are caught as well (S3).
    if scrubbed(&lower) || lower.split(is_value_split).any(scrubbed) {
        REMOVED.to_string()
    } else {
        value.to_string()
    }
}

fn scrubbed(lower: &str) -> bool {
    lower.split(is_separator).any(is_ip_token)
        || contains_ip(lower)
        || drive_path(lower)
        || lower.split_whitespace().any(rooted_token)
        || [
            "://",
            "file:",
            "@",
            "\\users\\",
            "/users/",
            "/home/",
            "$home",
        ]
        .iter()
        .any(|p| lower.contains(*p))
        || percent_var(lower)
        || lower.split(is_separator).any(machine_id_like)
}

/// The characters that split a value into the pieces each is scrubbed as.
fn is_value_split(c: char) -> bool {
    c.is_whitespace() || matches!(c, ':' | ';' | ',' | '=')
}

/// A GUID (`8-4-4-4-12` hex) or at least 24 hex digits: a machine or install
/// identifier, with surrounding punctuation ignored (S3).
fn machine_id_like(token: &str) -> bool {
    let token = token.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    let hex = |s: &str| s.bytes().all(|b| b.is_ascii_hexdigit());
    let parts: Vec<&str> = token.split('-').collect();
    let guid = parts.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&parts)
            .all(|(n, p)| p.len() == *n && hex(p));
    guid || (token.len() >= 24 && hex(token))
}

/// Whitespace and the characters that split an address from its text.
fn is_separator(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            ',' | ';' | '(' | ')' | '[' | ']' | '"' | '\'' | '=' | '%' | '/'
        )
}

/// Whether an address appears anywhere in the value, next to any punctuation
/// (A4): a run of hex digits, `.` and `:` that parses as an IP or socket
/// address, or four dot-separated decimal groups of one to three digits, each
/// at most 255 (a leading zero included).
fn contains_ip(lower: &str) -> bool {
    let address = |run: &str| {
        let run = run.trim_matches(|c: char| c == ':' || c == '.');
        run.parse::<IpAddr>().is_ok() || run.parse::<SocketAddr>().is_ok()
    };
    lower
        .split(|c: char| !(c.is_ascii_hexdigit() || c == '.' || c == ':'))
        .any(address)
        || lower
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .any(dotted_quad)
}

/// Whether the run holds four consecutive dot-separated decimal groups, each
/// of one to three digits and at most 255.
fn dotted_quad(run: &str) -> bool {
    let parts: Vec<&str> = run.split('.').collect();
    parts.windows(4).any(|quad| {
        quad.iter().all(|p| {
            (1..=3).contains(&p.len())
                && p.bytes().all(|b| b.is_ascii_digit())
                && p.parse::<u16>().is_ok_and(|n| n <= 255)
        })
    })
}

/// Whether the token is an address, its trailing sentence punctuation ignored (S3).
fn is_ip_token(token: &str) -> bool {
    let token = token.trim_end_matches(['.', '!', '?', '}']);
    token.parse::<IpAddr>().is_ok() || token.parse::<SocketAddr>().is_ok()
}

/// An ASCII letter followed by `:\` or `:/`.
pub(super) fn drive_path(lower: &str) -> bool {
    lower
        .as_bytes()
        .windows(3)
        .any(|w| w[0].is_ascii_alphabetic() && w[1] == b':' && (w[2] == b'\\' || w[2] == b'/'))
}

/// A token that, after one leading quote, starts with `\` or `/`.
fn rooted_token(token: &str) -> bool {
    let token = token.strip_prefix(['"', '\'']).unwrap_or(token);
    token.starts_with(['\\', '/'])
}

/// Two `%` characters with only `[A-Za-z0-9_]` between them.
fn percent_var(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    let mut open = None;
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'%' {
            continue;
        }
        if let Some(start) = open {
            if bytes[start + 1..i]
                .iter()
                .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
            {
                return true;
            }
        }
        open = Some(i);
    }
    false
}
