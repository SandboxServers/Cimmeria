//! Wire-format event schema for the telemetry chunk uploads.
//!
//! Each event serializes to one NDJSON line. The `type` tag drives
//! the server-side deserializer — adding a new variant is forward
//! compatible (older servers ignore unknown types) but renaming an
//! existing one is a breaking change.

use serde::{Deserialize, Serialize};

/// One streamed telemetry event. Serialized as `{"type": "...", ...}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    ClientLog(ClientLogEvent),
    DebugLog(DebugLogEvent),
    KeyDump(KeyDumpEvent),
    SessionMeta(SessionMetaEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientLogEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub level: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_no: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugLogEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyDumpEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub key_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionMetaEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub kind: SessionMetaKind,
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionMetaKind {
    Started,
    FileRotated,
    DroppedWindow,
}

/// Parsed fields of one client log line, returned by
/// [`parse_client_log_line`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedClientLog<'a> {
    pub level: &'a str,
    pub category: &'a str,
    pub packet_no: Option<u64>,
    pub message: &'a str,
}

/// Best-effort parse of an Atera client log line. The 2009 client
/// writes `[ts] [level] [category] message` with optional `pkt=N` /
/// `packet=N` / ` #N ` tokens. Unrecognized shapes fall through to
/// `level="info" category="raw"` so no data is silently dropped.
pub fn parse_client_log_line(line: &str) -> ParsedClientLog<'_> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let after_ts = strip_bracketed_prefix(trimmed);
    let (level, after_level) = match strip_bracketed_token(after_ts) {
        Some((tok, rest)) => (tok, rest),
        None => return raw_fallback(trimmed),
    };
    let (category, after_cat) = match strip_bracketed_token(after_level) {
        Some((tok, rest)) => (tok, rest),
        None => return raw_fallback(trimmed),
    };
    let message = after_cat.trim_start();
    ParsedClientLog {
        level,
        category,
        packet_no: extract_packet_no(message),
        message,
    }
}

fn strip_bracketed_prefix(s: &str) -> &str {
    let t = s.trim_start();
    if let Some(rest) = t.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return &rest[end + 1..];
        }
    }
    t
}

fn strip_bracketed_token(s: &str) -> Option<(&str, &str)> {
    let t = s.trim_start();
    let rest = t.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some((&rest[..end], &rest[end + 1..]))
}

fn extract_packet_no(message: &str) -> Option<u64> {
    for prefix in ["pkt=", "packet="] {
        if let Some(start) = message.find(prefix) {
            let after = &message[start + prefix.len()..];
            let n: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !n.is_empty() {
                if let Ok(v) = n.parse::<u64>() {
                    return Some(v);
                }
            }
        }
    }
    for (idx, _) in message.match_indices('#') {
        let before = idx == 0 || message[..idx].ends_with(|c: char| c.is_whitespace());
        if !before {
            continue;
        }
        let after = &message[idx + 1..];
        let n: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !n.is_empty() {
            if let Ok(v) = n.parse::<u64>() {
                return Some(v);
            }
        }
    }
    None
}

fn raw_fallback(line: &str) -> ParsedClientLog<'_> {
    ParsedClientLog {
        level: "info",
        category: "raw",
        packet_no: None,
        message: line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tagged serialization shape — the Functions-side verifier keys off
    // `type`. Any rename here is a breaking change.
    #[test]
    fn client_log_serializes_with_type_tag() {
        let ev = TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: 1_700_000_000_000,
            seq: 42,
            source_file: "2026-05-23.log".into(),
            level: "info".into(),
            category: "Mercury".into(),
            packet_no: Some(8821),
            message: "hello".into(),
        });
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["type"], "client_log");
        assert_eq!(json["seq"], 42);
        assert_eq!(json["packet_no"], 8821);
    }

    // Missing packet_no must not serialize as null — Cosmos partition
    // indexing is happier when optional fields are absent.
    #[test]
    fn client_log_omits_absent_packet_no() {
        let ev = TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: 0,
            seq: 0,
            source_file: "x".into(),
            level: "info".into(),
            category: "raw".into(),
            packet_no: None,
            message: "x".into(),
        });
        let json = serde_json::to_string(&ev).unwrap();
        assert!(
            !json.contains("packet_no"),
            "absent packet_no should not appear in serialized output: {json}"
        );
    }

    #[test]
    fn session_meta_kind_serializes_snake_case() {
        let ev = TelemetryEvent::SessionMeta(SessionMetaEvent {
            ts_ms: 0,
            seq: 0,
            kind: SessionMetaKind::FileRotated,
            fields: serde_json::Map::new(),
        });
        let json = serde_json::to_value(&ev).unwrap();
        assert_eq!(json["kind"], "file_rotated");
    }

    #[test]
    fn event_roundtrip_preserves_variant() {
        let cases = vec![
            TelemetryEvent::ClientLog(ClientLogEvent {
                ts_ms: 1,
                seq: 1,
                source_file: "a".into(),
                level: "info".into(),
                category: "c".into(),
                packet_no: None,
                message: "m".into(),
            }),
            TelemetryEvent::DebugLog(DebugLogEvent {
                ts_ms: 2,
                seq: 2,
                source_file: "b".into(),
                level: "warn".into(),
                message: "m".into(),
            }),
            TelemetryEvent::KeyDump(KeyDumpEvent {
                ts_ms: 3,
                seq: 3,
                source_file: "k".into(),
                key_b64: "aGVsbG8=".into(),
            }),
            TelemetryEvent::SessionMeta(SessionMetaEvent {
                ts_ms: 4,
                seq: 4,
                kind: SessionMetaKind::Started,
                fields: serde_json::Map::new(),
            }),
        ];
        for ev in cases {
            let text = serde_json::to_string(&ev).unwrap();
            let back: TelemetryEvent = serde_json::from_str(&text).unwrap();
            assert_eq!(back, ev);
        }
    }

    #[test]
    fn parse_client_log_line_standard_shape() {
        let p = parse_client_log_line("[2026-05-25 12:00:01] [info] [Mercury] sent pkt=8821 ok");
        assert_eq!(p.level, "info");
        assert_eq!(p.category, "Mercury");
        assert_eq!(p.packet_no, Some(8821));
        assert_eq!(p.message, "sent pkt=8821 ok");
    }

    #[test]
    fn parse_client_log_line_packet_alt_spellings() {
        let p = parse_client_log_line("[t] [w] [Net] frame packet=12 lost");
        assert_eq!(p.packet_no, Some(12));
        let p = parse_client_log_line("[t] [w] [Net] resend #7 after timeout");
        assert_eq!(p.packet_no, Some(7));
    }

    // `#define` should NOT register — only whitespace-prefixed `#N`
    // counts as a packet number.
    #[test]
    fn parse_client_log_line_ignores_non_packet_hash() {
        let p = parse_client_log_line("[t] [i] [Build] use #define WIN32");
        assert_eq!(p.packet_no, None);
    }

    // Missing brackets → raw fallback so no data is silently dropped.
    #[test]
    fn parse_client_log_line_falls_back_to_raw_on_unknown_shape() {
        let p = parse_client_log_line("plain text no brackets");
        assert_eq!(p.level, "info");
        assert_eq!(p.category, "raw");
        assert_eq!(p.message, "plain text no brackets");
        assert_eq!(p.packet_no, None);
    }

    #[test]
    fn parse_client_log_line_first_packet_match_wins() {
        let p = parse_client_log_line("[t] [i] [Net] pkt=5 retried as #99");
        assert_eq!(p.packet_no, Some(5));
    }
}
