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
}
