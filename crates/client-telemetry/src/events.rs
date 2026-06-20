//! Wire-format events emitted by the injected DLL.
//!
//! `ClientNativeEvent` is a byte-shape mirror of the launcher's
//! `cimmeria_launcher::telemetry::events::ClientNativeEvent` —
//! pinned by the server-side test
//! `tests::client_native_event_matches_launcher_shape` in
//! `crates/admin-api/src/routes/telemetry.rs`. Keep the field set
//! and `#[serde(tag = "type")]` discriminant identical or the
//! server-side replay will fail to parse our chunks.
//!
//! Naming convention for `target`: `client.<subsystem>.<event>` so
//! SigNoz can filter the whole DLL stream with `target LIKE 'client.%'`.
//! Examples: `client.frame_tick`, `client.mercury.dispatch`,
//! `client.streaming.update`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One streamed telemetry event. Serializes as
/// `{"type": "client_native", ...}`.
///
/// Wrapped in this `TelemetryEvent` enum (rather than serialized
/// bare) so the launcher's chunk uploader and ours produce
/// interchangeable wire bodies — the server's `enum TelemetryEvent`
/// at `routes/telemetry.rs:84` dispatches by the `type` tag.
///
/// Today we only ever emit `ClientNative`; the launcher emits the
/// other variants. The enum exists because the wire is single-
/// channel and the variants travel together.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    ClientNative(ClientNativeEvent),
}

/// Native event from the injected DLL.
///
/// `fields` uses `BTreeMap` rather than `serde_json::Map` so the
/// serialized byte order is deterministic — that pins the
/// integration test against the wire body without requiring a JSON
/// equality matcher. The server-side struct uses `serde_json::Map`
/// and reads any key order, so this asymmetry is upload-side only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientNativeEvent {
    /// Milliseconds since the Unix epoch. Produced by
    /// `SystemTime::now().duration_since(UNIX_EPOCH)`.
    pub ts_ms: i64,
    /// Per-process monotonic sequence number. The uploader
    /// allocates these from a single `AtomicU64` so producers across
    /// threads always see strictly increasing values.
    pub seq: u64,
    /// Tracing target the server replays under.
    pub target: String,
    /// Tracing level: `"trace"` / `"debug"` / `"info"` / `"warn"` /
    /// `"error"`. Unknown values fall through to `"info"` server-side.
    pub level: String,
    /// Arbitrary key-value bag. Server-side fans these out as
    /// structured-log fields so SigNoz indexes them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl ClientNativeEvent {
    /// Convenience builder for "I just want a target + level + a
    /// few fields" — the common shape for hook handlers.
    ///
    /// `ts_ms` and `seq` are filled in by the queue's allocator at
    /// enqueue time, NOT here, because (a) the producer shouldn't
    /// have to read the clock and (b) `seq` is shared mutable state
    /// owned by the queue. Use `EventBuilder` to construct, then
    /// hand off to the queue.
    pub fn builder(target: &str, level: &str) -> EventBuilder {
        EventBuilder {
            target: target.to_string(),
            level: level.to_string(),
            fields: BTreeMap::new(),
        }
    }
}

/// Producer-facing builder. Constructed by [`ClientNativeEvent::builder`],
/// converted to a `ClientNativeEvent` by the queue (which stamps
/// `ts_ms` + `seq`).
#[derive(Debug, Clone)]
pub struct EventBuilder {
    pub(crate) target: String,
    pub(crate) level: String,
    pub(crate) fields: BTreeMap<String, serde_json::Value>,
}

impl EventBuilder {
    /// Attach a structured field. Common shapes (numbers, strings,
    /// bools) work directly via `serde_json::json!`. Nested objects
    /// are allowed but the server fans only top-level keys; deeper
    /// structure is preserved as a stringified value in SigNoz.
    pub fn field(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.fields.insert(key.to_string(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wire shape pin: must serialize to the same JSON the launcher's
    /// `ClientNativeEvent` does. Any field rename / removal here
    /// breaks the server-side `routes/telemetry.rs` dispatcher.
    #[test]
    fn client_native_event_wire_shape() {
        let ev = TelemetryEvent::ClientNative(ClientNativeEvent {
            ts_ms: 1_700_000_000_000,
            seq: 42,
            target: "client.frame_tick".into(),
            level: "debug".into(),
            fields: {
                let mut m = BTreeMap::new();
                m.insert("delta_ms".into(), serde_json::json!(16.7));
                m
            },
        });
        let json = serde_json::to_string(&ev).unwrap();
        // Field order is deterministic via BTreeMap + serde struct
        // order. `type` lands first via the tag attribute.
        assert_eq!(
            json,
            r#"{"type":"client_native","ts_ms":1700000000000,"seq":42,"target":"client.frame_tick","level":"debug","fields":{"delta_ms":16.7}}"#
        );
    }

    /// Empty `fields` map omits the key entirely (saves bytes on
    /// hot-path events that only carry target + level). Server
    /// `#[serde(default)]` makes the absence safe.
    #[test]
    fn empty_fields_map_omits_key() {
        let ev = TelemetryEvent::ClientNative(ClientNativeEvent {
            ts_ms: 1,
            seq: 2,
            target: "t".into(),
            level: "info".into(),
            fields: BTreeMap::new(),
        });
        let json = serde_json::to_string(&ev).unwrap();
        assert!(!json.contains("\"fields\""));
    }

    /// Round-trip through serde: a serialized event deserializes to
    /// an equal value. Pins the round-trip contract — the server
    /// must be able to parse exactly what we send.
    #[test]
    fn roundtrip_serialize_deserialize() {
        let ev = ClientNativeEvent {
            ts_ms: 1_700_000_000_000,
            seq: 100,
            target: "client.mercury.dispatch".into(),
            level: "info".into(),
            fields: {
                let mut m = BTreeMap::new();
                m.insert("msg_id".into(), serde_json::json!(0xC0u32));
                m.insert("len".into(), serde_json::json!(48u64));
                m
            },
        };
        let wrapped = TelemetryEvent::ClientNative(ev.clone());
        let json = serde_json::to_string(&wrapped).unwrap();
        let back: TelemetryEvent = serde_json::from_str(&json).unwrap();
        match back {
            TelemetryEvent::ClientNative(got) => assert_eq!(got, ev),
        }
    }

    /// EventBuilder ergonomics: target/level + chained `.field(...)`
    /// calls produce a builder that the queue can consume.
    #[test]
    fn builder_chains_fields() {
        let b = ClientNativeEvent::builder("client.streaming.update", "info")
            .field("level_name", serde_json::Value::String("Castle".into()))
            .field("transitioning", serde_json::Value::Bool(true))
            .field("count", serde_json::json!(7));
        assert_eq!(b.target, "client.streaming.update");
        assert_eq!(b.level, "info");
        assert_eq!(b.fields.len(), 3);
        assert_eq!(b.fields["count"], serde_json::json!(7));
    }
}
