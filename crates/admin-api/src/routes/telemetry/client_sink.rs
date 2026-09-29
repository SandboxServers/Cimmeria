//! Structured export of client-originated telemetry.
//!
//! Replaying an upload through `tracing` puts every client event under the
//! server's `service.name`, and the DLL's free-form `fields` map arrives in
//! SigNoz as one JSON string that cannot be filtered on. When the server's
//! OTLP export is on, it installs a [`ClientLogSink`] ([`install`]) backed
//! by its own `service.name = cimmeria-client` logger provider, and every
//! uploaded event is also handed to it as a [`ClientRecord`] whose fields
//! are real attributes. The server's other OTLP layers skip these targets
//! (`otel::is_client_target`), so nothing is indexed twice; the `tracing`
//! replay still feeds the on-disk logs and the admin WebSocket.
//!
//! This crate stays free of OpenTelemetry: the sink takes plain values and
//! the server converts them.
//!
//! Every record of a session also carries that session's identity once the
//! client has reported it (`client.session.identity`, see
//! [`IDENTITY_TARGET`]): the player's entity id and the like are the join key
//! to the server's own events for the same player.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::dto::{ClientNativeEvent, TelemetryEvent};
use crate::routes::dev_session::TokenClaims;

/// One attribute value.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    Str(String),
    I64(i64),
    F64(f64),
    Bool(bool),
}

/// Severity of a [`ClientRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl ClientSeverity {
    fn parse(level: &str) -> Self {
        match level.to_ascii_lowercase().as_str() {
            "trace" => Self::Trace,
            "debug" => Self::Debug,
            "warn" | "warning" => Self::Warn,
            "error" | "fatal" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// One client-originated log record.
#[derive(Debug, Clone, PartialEq)]
pub struct ClientRecord {
    /// Instrumentation scope: `client.native` for DLL events, the
    /// `launcher.*` target for launcher-read logs.
    pub scope: &'static str,
    pub severity: ClientSeverity,
    /// Client-side timestamp, epoch ms; 0 when the client sent none.
    pub ts_ms: i64,
    pub body: String,
    pub attributes: Vec<(String, AttrValue)>,
}

/// Where [`ClientRecord`]s go. Implemented by the server over its
/// `cimmeria-client` logger provider.
pub trait ClientLogSink: Send + Sync {
    fn emit(&self, record: ClientRecord);
}

static SINK: OnceLock<Box<dyn ClientLogSink>> = OnceLock::new();

/// Install the process's sink. The first call wins; later calls are
/// ignored and return `false`.
pub fn install(sink: Box<dyn ClientLogSink>) -> bool {
    SINK.set(sink).is_ok()
}

/// The DLL target that carries the player's identity.
pub const IDENTITY_TARGET: &str = "client.session.identity";

/// Identity fields copied from [`IDENTITY_TARGET`] onto every later record
/// of the same session. Anything else in that event stays on it alone.
const IDENTITY_KEYS: &[&str] = &[
    "player_entity_id",
    "account_id",
    "account_name",
    "character_name",
    "server_addr",
    "world",
];

/// Attribute names the record itself sets; a DLL field with one of these
/// names is kept under `field.<name>` instead of overwriting it.
const RESERVED: &[&str] = &["session_id", "install_id", "seq", "ts_ms", "client_target"];

/// Sessions whose identity is remembered at once; the oldest is forgotten
/// beyond this.
const MAX_SESSIONS: usize = 4096;

#[derive(Default)]
struct Identities {
    by_session: HashMap<String, Vec<(String, AttrValue)>>,
    order: Vec<String>,
}

impl Identities {
    fn get(&self, sid: &str) -> Vec<(String, AttrValue)> {
        self.by_session.get(sid).cloned().unwrap_or_default()
    }

    fn remember(&mut self, sid: &str, attrs: Vec<(String, AttrValue)>) {
        if attrs.is_empty() {
            return;
        }
        if !self.by_session.contains_key(sid) {
            self.order.push(sid.to_string());
            if self.order.len() > MAX_SESSIONS {
                let oldest = self.order.remove(0);
                self.by_session.remove(&oldest);
            }
        }
        let entry = self.by_session.entry(sid.to_string()).or_default();
        for (k, v) in attrs {
            entry.retain(|(ek, _)| *ek != k);
            entry.push((k, v));
        }
    }
}

fn identities() -> &'static Mutex<Identities> {
    static IDS: OnceLock<Mutex<Identities>> = OnceLock::new();
    IDS.get_or_init(Mutex::default)
}

/// Hand `ev` to the installed sink, if any.
pub(super) fn export(claims: &TokenClaims, ev: &TelemetryEvent) {
    let Some(sink) = SINK.get() else {
        return;
    };
    let mut ids = identities().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(record) = build_record(claims, ev, &mut ids) {
        drop(ids);
        sink.emit(record);
    }
}

/// Build the record for `ev`, learning the session's identity from it
/// first when it carries one. `None` for events that must not leave the
/// server's own logs (key dumps).
fn build_record(
    claims: &TokenClaims,
    ev: &TelemetryEvent,
    ids: &mut Identities,
) -> Option<ClientRecord> {
    let mut attributes = vec![
        ("session_id".to_string(), AttrValue::Str(claims.sid.clone())),
        ("install_id".to_string(), AttrValue::Str(claims.sub.clone())),
    ];
    let record = match ev {
        TelemetryEvent::ClientNative(e) => {
            if e.target == IDENTITY_TARGET {
                ids.remember(&claims.sid, identity_attrs(e));
            }
            attributes.push(("seq".into(), AttrValue::I64(e.seq as i64)));
            attributes.push(("client_target".into(), AttrValue::Str(e.target.clone())));
            attributes.extend(flatten(&e.fields));
            ClientRecord {
                scope: "client.native",
                severity: ClientSeverity::parse(&e.level),
                ts_ms: e.ts_ms,
                body: e.target.clone(),
                attributes,
            }
        }
        TelemetryEvent::DebugLog(e) => {
            attributes.push(("seq".into(), AttrValue::I64(e.seq as i64)));
            attributes.push(("source_file".into(), AttrValue::Str(e.source_file.clone())));
            ClientRecord {
                scope: "launcher.debug_log",
                severity: ClientSeverity::parse(&e.level),
                ts_ms: e.ts_ms,
                body: e.message.clone(),
                attributes,
            }
        }
        TelemetryEvent::ClientLog(e) => {
            attributes.push(("seq".into(), AttrValue::I64(e.seq as i64)));
            attributes.push(("source_file".into(), AttrValue::Str(e.source_file.clone())));
            attributes.push(("category".into(), AttrValue::Str(e.category.clone())));
            if let Some(n) = e.packet_no {
                attributes.push(("packet_no".into(), AttrValue::I64(n as i64)));
            }
            ClientRecord {
                scope: "launcher.client_log",
                severity: ClientSeverity::parse(&e.level),
                ts_ms: e.ts_ms,
                body: e.message.clone(),
                attributes,
            }
        }
        TelemetryEvent::SessionMeta(e) => {
            attributes.push(("seq".into(), AttrValue::I64(e.seq as i64)));
            attributes.push(("kind".into(), AttrValue::Str(e.kind.clone())));
            attributes.extend(flatten(&e.fields));
            ClientRecord {
                scope: "launcher.session_meta",
                severity: ClientSeverity::Info,
                ts_ms: e.ts_ms,
                body: format!("session {}", e.kind),
                attributes,
            }
        }
        // Key material stays in the server's DEBUG file logs only.
        TelemetryEvent::KeyDump(_) => return None,
    };
    let mut record = record;
    for (k, v) in ids.get(&claims.sid) {
        if !record.attributes.iter().any(|(ek, _)| *ek == k) {
            record.attributes.push((k, v));
        }
    }
    Some(record)
}

fn identity_attrs(e: &ClientNativeEvent) -> Vec<(String, AttrValue)> {
    IDENTITY_KEYS
        .iter()
        .filter_map(|k| {
            e.fields
                .get(*k)
                .and_then(scalar)
                .map(|v| (k.to_string(), v))
        })
        .collect()
}

/// One attribute per field: scalars as themselves, anything nested as its
/// JSON text. A field named like one the record sets is kept as
/// `field.<name>`.
fn flatten(fields: &serde_json::Map<String, serde_json::Value>) -> Vec<(String, AttrValue)> {
    fields
        .iter()
        .map(|(k, v)| {
            let key = if RESERVED.contains(&k.as_str()) {
                format!("field.{k}")
            } else {
                k.clone()
            };
            let value = scalar(v).unwrap_or_else(|| AttrValue::Str(v.to_string()));
            (key, value)
        })
        .collect()
}

fn scalar(v: &serde_json::Value) -> Option<AttrValue> {
    match v {
        serde_json::Value::String(s) => Some(AttrValue::Str(s.clone())),
        serde_json::Value::Bool(b) => Some(AttrValue::Bool(*b)),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(AttrValue::I64)
            .or_else(|| n.as_u64().map(|u| AttrValue::Str(u.to_string())))
            .or_else(|| n.as_f64().map(AttrValue::F64)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::dto::{DebugLogEvent, KeyDumpEvent};
    use super::*;

    fn claims(sid: &str) -> TokenClaims {
        TokenClaims {
            iss: "t".into(),
            sub: "install-1".into(),
            sid: sid.into(),
            iat: 0,
            exp: 0,
            scope: vec![],
        }
    }

    fn native(target: &str, fields: serde_json::Value) -> TelemetryEvent {
        TelemetryEvent::ClientNative(ClientNativeEvent {
            ts_ms: 5,
            seq: 7,
            target: target.into(),
            level: "warn".into(),
            fields: fields.as_object().unwrap().clone(),
        })
    }

    fn attr<'a>(r: &'a ClientRecord, k: &str) -> Option<&'a AttrValue> {
        r.attributes.iter().find(|(ek, _)| ek == k).map(|(_, v)| v)
    }

    /// The point of the sink: DLL fields become attributes SigNoz can
    /// filter on, not one JSON string.
    #[test]
    fn native_fields_become_typed_attributes() {
        let mut ids = Identities::default();
        let r = build_record(
            &claims("s1"),
            &native(
                "client.engine.frame_hitch",
                serde_json::json!({"ms": 250, "ratio": 1.5, "map": "castle", "ok": true,
                                   "nested": {"a": 1}, "seq": 99}),
            ),
            &mut ids,
        )
        .unwrap();
        assert_eq!(r.scope, "client.native");
        assert_eq!(r.severity, ClientSeverity::Warn);
        assert_eq!(r.body, "client.engine.frame_hitch");
        assert_eq!(attr(&r, "ms"), Some(&AttrValue::I64(250)));
        assert_eq!(attr(&r, "ratio"), Some(&AttrValue::F64(1.5)));
        assert_eq!(attr(&r, "map"), Some(&AttrValue::Str("castle".into())));
        assert_eq!(attr(&r, "ok"), Some(&AttrValue::Bool(true)));
        assert_eq!(
            attr(&r, "nested"),
            Some(&AttrValue::Str("{\"a\":1}".into()))
        );
        // A DLL field named like a record attribute cannot overwrite it.
        assert_eq!(attr(&r, "seq"), Some(&AttrValue::I64(7)));
        assert_eq!(attr(&r, "field.seq"), Some(&AttrValue::I64(99)));
        assert_eq!(attr(&r, "session_id"), Some(&AttrValue::Str("s1".into())));
    }

    /// Once the client reports who it is, every later record of that
    /// session carries it, launcher-read logs included; other sessions
    /// do not.
    #[test]
    fn identity_is_stamped_on_later_records_of_the_same_session_only() {
        let mut ids = Identities::default();
        build_record(
            &claims("s1"),
            &native(
                IDENTITY_TARGET,
                serde_json::json!({"player_entity_id": 4242, "account_name": "tester", "junk": 1}),
            ),
            &mut ids,
        );
        let later = build_record(
            &claims("s1"),
            &TelemetryEvent::DebugLog(DebugLogEvent {
                ts_ms: 1,
                seq: 2,
                source_file: "SGWDebugLog.log".into(),
                level: "info".into(),
                message: "hello".into(),
            }),
            &mut ids,
        )
        .unwrap();
        assert_eq!(
            attr(&later, "player_entity_id"),
            Some(&AttrValue::I64(4242))
        );
        assert_eq!(
            attr(&later, "account_name"),
            Some(&AttrValue::Str("tester".into()))
        );
        assert_eq!(
            attr(&later, "junk"),
            None,
            "only identity keys are carried over"
        );
        assert_eq!(later.body, "hello");

        let other = build_record(
            &claims("s2"),
            &native("client.x", serde_json::json!({})),
            &mut ids,
        )
        .unwrap();
        assert_eq!(attr(&other, "player_entity_id"), None);
    }

    #[test]
    fn key_dumps_are_never_exported() {
        let mut ids = Identities::default();
        let ev = TelemetryEvent::KeyDump(KeyDumpEvent {
            ts_ms: 0,
            seq: 0,
            source_file: "k".into(),
            key_b64: "secret".into(),
        });
        assert!(build_record(&claims("s1"), &ev, &mut ids).is_none());
    }

    #[test]
    fn identity_memory_is_bounded() {
        let mut ids = Identities::default();
        for i in 0..=MAX_SESSIONS {
            ids.remember(&format!("s{i}"), vec![("world".into(), AttrValue::I64(1))]);
        }
        assert_eq!(ids.by_session.len(), MAX_SESSIONS);
        assert!(ids.get("s0").is_empty(), "the oldest session is forgotten");
    }
}
