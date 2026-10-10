//! Length caps on the strings an uploaded row carries into a log event.
//!
//! Every field of an uploaded row is the client's own text, and an accepted
//! row copies it into a `tracing` event that every sink stores. The caps
//! keep one row from becoming a multi-megabyte log line. A value over its
//! cap is cut at a character boundary and ends in [`marker`], which says
//! how long the original was, so a reader can tell a cut value from a short
//! one.
//!
//! | Field | Cap |
//! |---|---|
//! | `message` (client and debug log lines, bundle lines) | [`MAX_MESSAGE_BYTES`] |
//! | `source_file`, `level`, `category`, `target`, `kind` | [`MAX_LABEL_BYTES`] |
//! | `key_b64` | [`MAX_KEY_BYTES`] |
//! | a `fields` bag | [`MAX_FIELD_KEYS`] keys, each key [`MAX_LABEL_BYTES`], each value [`MAX_FIELD_VALUE_BYTES`]; a nested array or object becomes its JSON text |

use serde_json::{Map, Value};

use super::dto::TelemetryEvent;

/// A log line. Client log lines are rarely over a few hundred bytes; a
/// Lua stack trace fits in 4 KiB.
pub(super) const MAX_MESSAGE_BYTES: usize = 4 * 1024;

/// A file name, level, category or event name.
pub(super) const MAX_LABEL_BYTES: usize = 256;

/// A key dump: a base64 session key is well under 100 bytes.
pub(super) const MAX_KEY_BYTES: usize = 1024;

/// Keys kept from a `fields` bag. The DLL's richest rows carry about 20.
pub(super) const MAX_FIELD_KEYS: usize = 64;

/// One value in a `fields` bag: a string, or an object or array, which is
/// always replaced by its JSON text and capped as a string.
pub(super) const MAX_FIELD_VALUE_BYTES: usize = 2 * 1024;

/// The key that records how many keys a `fields` bag lost to
/// [`MAX_FIELD_KEYS`].
pub(super) const DROPPED_KEYS_FIELD: &str = "_truncated_keys";

/// The suffix a cut value ends in.
pub(super) fn marker(original_len: usize) -> String {
    format!("...[truncated, {original_len} bytes]")
}

/// `s` cut to at most `cap` bytes of its own text, plus the marker, at a
/// character boundary. Returns whether it was cut.
pub(super) fn cap_string(s: &mut String, cap: usize) -> bool {
    if s.len() <= cap {
        return false;
    }
    let original = s.len();
    let mut end = cap;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    s.push_str(&marker(original));
    true
}

/// [`cap_string`] for a borrowed line: the line itself when it fits.
pub(super) fn capped(s: &str, cap: usize) -> std::borrow::Cow<'_, str> {
    if s.len() <= cap {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut owned = s.to_string();
    cap_string(&mut owned, cap);
    std::borrow::Cow::Owned(owned)
}

/// Cap a `fields` bag in place (see the module docs).
pub(super) fn cap_fields(fields: &mut Map<String, Value>) {
    if fields.len() > MAX_FIELD_KEYS {
        let dropped = fields.len() - MAX_FIELD_KEYS;
        *fields = std::mem::take(fields)
            .into_iter()
            .take(MAX_FIELD_KEYS)
            .collect();
        fields.insert(DROPPED_KEYS_FIELD.into(), Value::from(dropped as u64));
    }
    let long_keys: Vec<String> = fields
        .keys()
        .filter(|k| k.len() > MAX_LABEL_BYTES)
        .cloned()
        .collect();
    for key in long_keys {
        if let Some(v) = fields.remove(&key) {
            let mut short = key;
            cap_string(&mut short, MAX_LABEL_BYTES);
            fields.insert(short, v);
        }
    }
    for value in fields.values_mut() {
        match value {
            Value::String(s) => {
                cap_string(s, MAX_FIELD_VALUE_BYTES);
            }
            // Always flattened to its JSON text, whatever its size: a parsed
            // array or object costs many times its text in memory (a dense
            // array of small numbers about 16x), and nothing reads nested
            // values (the replay renders `fields` as JSON text and lifts
            // only top-level scalars).
            Value::Array(_) | Value::Object(_) => {
                let json = value.to_string();
                *value = Value::String(capped(&json, MAX_FIELD_VALUE_BYTES).into_owned());
            }
            _ => {}
        }
    }
}

/// Cap every client-supplied string of one row, before it is admitted,
/// named or replayed.
pub(super) fn cap_event(ev: &mut TelemetryEvent) {
    match ev {
        TelemetryEvent::ClientLog(e) => {
            cap_string(&mut e.source_file, MAX_LABEL_BYTES);
            cap_string(&mut e.level, MAX_LABEL_BYTES);
            cap_string(&mut e.category, MAX_LABEL_BYTES);
            cap_string(&mut e.message, MAX_MESSAGE_BYTES);
        }
        TelemetryEvent::DebugLog(e) => {
            cap_string(&mut e.source_file, MAX_LABEL_BYTES);
            cap_string(&mut e.level, MAX_LABEL_BYTES);
            cap_string(&mut e.message, MAX_MESSAGE_BYTES);
        }
        TelemetryEvent::KeyDump(e) => {
            cap_string(&mut e.source_file, MAX_LABEL_BYTES);
            cap_string(&mut e.key_b64, MAX_KEY_BYTES);
        }
        TelemetryEvent::SessionMeta(e) => {
            cap_string(&mut e.kind, MAX_LABEL_BYTES);
            cap_fields(&mut e.fields);
        }
        TelemetryEvent::ClientNative(e) => {
            cap_string(&mut e.target, MAX_LABEL_BYTES);
            cap_string(&mut e.level, MAX_LABEL_BYTES);
            cap_fields(&mut e.fields);
        }
    }
}
