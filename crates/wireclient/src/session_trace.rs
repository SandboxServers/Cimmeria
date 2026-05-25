//! Captured-session trace format — the on-disk representation of a decrypted
//! Mercury pcap that wireclient consumes for replay tests.
//!
//! ## Generalised, not Castle-Cellblock-specific
//!
//! The trace format is **capture-agnostic**: it represents an arbitrary
//! Mercury session as a chronological stream of `(direction, msg_id, body)`
//! triples plus the AES key needed to decrypt the original wire bytes (kept
//! alongside so future tooling can re-derive the trace from raw `.pcap`).
//!
//! A single test corpus can hold many traces — one per recorded session, one
//! per zone, one per regression case. The Castle Cellblock e2e is the
//! flagship; arbitrary follow-up captures drop in beside it without code
//! changes.
//!
//! ## Producing a trace
//!
//! `tools/pcap_to_session.py <pcap> <keys.txt> > corpus/<slug>.jsonl`
//!
//! Each line is one [`TraceEvent`] in JSON. The exporter rides on top of
//! `tools/pcap_dissect.py`, which already decrypts AES-256-CBC and strips
//! Mercury footers; the only addition is JSONL serialisation. Keeping the
//! producer in Python (rather than rewriting the pcap reader in Rust) buys
//! us:
//!
//! - **One source of truth** for the wire-format decoder — the existing
//!   Python dissector is already cross-checked against `crates/mercury`'s
//!   `parse_incoming` (the loopback harness pins byte-exactness on both
//!   ends).
//! - **Hermeticity** — the Rust replay path never touches `libpcap`; it only
//!   reads the JSONL fixture. Tests are deterministic.
//!
//! ## Consuming a trace
//!
//! The replay engine (Phase 1+) walks the trace event-by-event:
//!
//! 1. For a `C2S` event: build the same Mercury packet and send it.
//! 2. For an `S2C` event: read the server's actual response, decrypt, parse,
//!    and diff against the recorded body using the per-msg-id
//!    [`ComparisonPolicy`].
//!
//! A drift that the policy classifies as "load-bearing" fails the test;
//! drifts in tolerable fields (timestamps, seq IDs, nonces) are logged but
//! pass.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

// ── On-disk shape ───────────────────────────────────────────────────────────

/// One captured packet in a Mercury session.
///
/// The format intentionally mirrors what `tools/pcap_dissect.py` already
/// produces — direction, sequence ID, flag bits, list of (msg_id, body)
/// pairs. Adding a JSON output mode to the dissector is a one-line change;
/// see `tools/pcap_to_session.py`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEvent {
    /// Wall-clock offset from the start of the capture, in seconds.
    /// Used for keepalive-cadence assertions and replay pacing, not for
    /// byte-exact diffing.
    pub t_seconds: f64,

    /// Wire-level packet index within the capture (1-based). Diff failures
    /// quote this so log lines line up with `pcap_dissect.py` output.
    pub packet_no: u32,

    /// Direction of travel.
    pub direction: Direction,

    /// Mercury sequence ID, if `FLAG_HAS_SEQUENCE` was set.
    pub seq: Option<u32>,

    /// Raw flag byte (`FLAG_HAS_REQUESTS | FLAG_ON_CHANNEL | …`).
    pub flags: u8,

    /// Cumulative ACKs piggybacked on this packet.
    #[serde(default)]
    pub acks: Vec<u32>,

    /// Messages packed into the packet body, in order.
    pub messages: Vec<TraceMessage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Client → server.
    C2s,
    /// Server → client.
    S2c,
}

/// One Mercury message extracted from a packet body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceMessage {
    /// Mercury message ID byte. `0x00`–`0x7F` are static base messages
    /// (`baseAppLogin`, `tickSync`, `enableEntities`, …); `0x80`–`0xFE` are
    /// dynamic entity methods registered against the player or a witnessed
    /// entity.
    pub msg_id: u8,

    /// Human-readable name where the dissector recognised the msg_id.
    /// Optional because dynamic entity methods don't have stable names
    /// across all entity classes.
    pub name: Option<String>,

    /// Raw message body bytes (lowercase hex). Encoded as a string rather
    /// than a `Vec<u8>` for JSON ergonomics — JSONL stays human-diffable.
    pub body_hex: String,
}

impl TraceMessage {
    /// Decode the hex body into bytes. Returns an error if the producer
    /// emitted malformed hex.
    pub fn body_bytes(&self) -> Result<Vec<u8>> {
        hex::decode(&self.body_hex).map_err(Error::from)
    }

    /// Byte length without paying the hex-decode cost.
    pub fn body_len(&self) -> usize {
        self.body_hex.len() / 2
    }
}

/// Top-of-file metadata describing the capture as a whole.
///
/// Stored as the *first line* of the JSONL file under a discriminator —
/// every subsequent line is a [`TraceEvent`]. The producer always writes the
/// header line first; the reader assumes that ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceHeader {
    /// Human-readable label for the capture (e.g. `"castle-cellblock-full-run"`).
    pub label: String,

    /// Source pcap filename, recorded for traceability.
    pub source_pcap: String,

    /// AES-256 session key as 64 hex chars — kept so the replay engine can
    /// verify the recorded `body_hex` decrypts cleanly from the original
    /// pcap if a test wants to re-derive a trace.
    pub session_key_hex: String,

    /// Client UDP endpoint observed in the capture (e.g. `"127.0.0.1:56927"`).
    pub client_addr: String,

    /// Server UDP endpoint observed in the capture (e.g. `"127.0.0.1:32832"`).
    pub server_addr: String,

    /// Total packet count, for fast sanity checks at load time.
    pub packet_count: u32,

    /// Schema version. Bumped on any breaking change to the JSONL shape.
    #[serde(default = "default_schema")]
    pub schema_version: u32,
}

/// Currently-supported on-disk schema version. Bump in lockstep with any
/// breaking change to the JSONL shape, and add the previous version to
/// `SUPPORTED_SCHEMA_VERSIONS` if backwards compatibility is required.
pub const TRACE_SCHEMA_VERSION: u32 = 1;

/// Schema versions the loader will accept. Today, exactly one — extend
/// once a v2 is shipped with documented migration semantics.
const SUPPORTED_SCHEMA_VERSIONS: &[u32] = &[TRACE_SCHEMA_VERSION];

fn default_schema() -> u32 {
    TRACE_SCHEMA_VERSION
}

/// Loaded trace — header + ordered events.
#[derive(Debug, Clone)]
pub struct Trace {
    pub header: TraceHeader,
    pub events: Vec<TraceEvent>,
}

impl Trace {
    /// Load a trace from a JSONL file.
    ///
    /// File shape: line 1 is a `TraceHeader` wrapped in
    /// `{ "header": ... }`. Lines 2+ are `TraceEvent`s wrapped in
    /// `{ "event": ... }`. Any other top-level key is rejected so a
    /// producer drift surfaces at load time, not at replay time.
    pub fn from_jsonl_path(path: impl AsRef<Path>) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(Error::TraceIo)?;
        Self::from_jsonl_str(&text)
    }

    /// Parse a trace from in-memory JSONL. Visible for unit tests that
    /// embed fixtures inline.
    pub fn from_jsonl_str(text: &str) -> Result<Self> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty());
        let header_line = lines
            .next()
            .ok_or_else(|| Error::TraceIo(std::io::Error::other("trace file is empty")))?;
        let header_envelope: HeaderEnvelope = serde_json::from_str(header_line)?;
        let header = header_envelope.header;

        // Reject unknown schema versions — a v2 trace silently loading as
        // v1 is a class of regression the schema field exists to prevent.
        if !SUPPORTED_SCHEMA_VERSIONS.contains(&header.schema_version) {
            return Err(Error::TraceIo(std::io::Error::other(format!(
                "unsupported schema_version: {} (supported: {:?})",
                header.schema_version, SUPPORTED_SCHEMA_VERSIONS,
            ))));
        }

        let mut events = Vec::with_capacity(header.packet_count as usize);
        for line in lines {
            let env: EventEnvelope = serde_json::from_str(line)?;
            events.push(env.event);
        }

        // Enforce the header's packet_count — a truncated JSONL that
        // claims more events than it carries would otherwise pass
        // silently and surface as a mid-replay assertion failure.
        if events.len() != header.packet_count as usize {
            return Err(Error::TraceIo(std::io::Error::other(format!(
                "packet_count mismatch: header={} events={}",
                header.packet_count,
                events.len(),
            ))));
        }

        Ok(Trace { header, events })
    }

    /// Filter to only client-originated events — useful when wiring up the
    /// driver, which only *sends* C2S events and *receives* S2C events.
    pub fn c2s(&self) -> impl Iterator<Item = &TraceEvent> {
        self.events.iter().filter(|e| e.direction == Direction::C2s)
    }

    /// Filter to only server-originated events.
    pub fn s2c(&self) -> impl Iterator<Item = &TraceEvent> {
        self.events.iter().filter(|e| e.direction == Direction::S2c)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderEnvelope {
    header: TraceHeader,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventEnvelope {
    event: TraceEvent,
}

// ── Diff & comparison policy ────────────────────────────────────────────────

/// Result of comparing one observed `TraceMessage` against the recorded
/// baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diff {
    /// Observed bytes equal the recorded bytes exactly.
    Exact,
    /// Bytes differ, but only in fields the policy classifies as drift
    /// (timestamps, seq IDs, server-assigned entity IDs, …). The string
    /// names the drifted field for log output.
    Drift(String),
    /// Bytes differ in a load-bearing field. Test failure.
    Regression(String),
}

/// Decides how strictly an observed message must match the recorded one.
///
/// The default policy is byte-exact for static msg_ids (`0x00`–`0x7F`) and
/// structure-only for entity-method msg_ids (`0x80`–`0xFE`), reflecting the
/// observation that base messages carry no server-assigned state but
/// entity-method bodies often embed runtime-allocated IDs.
///
/// A test can swap in a stricter or looser policy by impl'ing this trait;
/// the replay engine will call `compare` for every observed message.
pub trait ComparisonPolicy: Send + Sync {
    fn compare(&self, observed: &TraceMessage, recorded: &TraceMessage) -> Diff;
}

/// Default policy: byte-exact for static messages, length-and-msg_id for
/// dynamic entity-method messages.
pub struct DefaultPolicy;

impl ComparisonPolicy for DefaultPolicy {
    fn compare(&self, observed: &TraceMessage, recorded: &TraceMessage) -> Diff {
        if observed.msg_id != recorded.msg_id {
            return Diff::Regression(format!(
                "msg_id mismatch: observed 0x{:02x} ({:?}), recorded 0x{:02x} ({:?})",
                observed.msg_id, observed.name, recorded.msg_id, recorded.name
            ));
        }
        // Case-insensitive byte comparison — the Python producer emits
        // lowercase hex but a hand-edited or future-Rust-derived fixture
        // could use uppercase; both round-trip to the same bytes.
        if observed.body_hex.eq_ignore_ascii_case(&recorded.body_hex) {
            return Diff::Exact;
        }
        // Entity-method dynamic IDs (`0x80..=0xFE`) commonly embed
        // server-assigned IDs; treat a length match as structural drift,
        // length mismatch as regression. `0xFF` is the static
        // `BASEMSG_REPLY_MESSAGE` — exclude it from the drift band so a
        // wire-format change there fails loudly.
        if (0x80..=0xFE).contains(&observed.msg_id) {
            if observed.body_len() == recorded.body_len() {
                return Diff::Drift(format!(
                    "entity-method body bytes differ but length matches ({}B)",
                    observed.body_len()
                ));
            }
            return Diff::Regression(format!(
                "entity-method body length: observed {}B, recorded {}B",
                observed.body_len(),
                recorded.body_len()
            ));
        }
        Diff::Regression(format!(
            "static msg 0x{:02x} body differs: observed {} vs recorded {}",
            observed.msg_id, observed.body_hex, recorded.body_hex
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal_trace() {
        let header = TraceHeader {
            label: "fixture-min".into(),
            source_pcap: "fixture.pcap".into(),
            session_key_hex: "00".repeat(32),
            client_addr: "127.0.0.1:1000".into(),
            server_addr: "127.0.0.1:2000".into(),
            packet_count: 1,
            schema_version: 1,
        };
        let event = TraceEvent {
            t_seconds: 0.5,
            packet_no: 1,
            direction: Direction::C2s,
            seq: Some(0),
            flags: 0x58,
            acks: vec![],
            messages: vec![TraceMessage {
                msg_id: 0x08,
                name: Some("enableEntities".into()),
                body_hex: "0000004000004000".into(),
            }],
        };

        let mut jsonl = String::new();
        jsonl.push_str(&serde_json::to_string(&HeaderJ { header }).unwrap());
        jsonl.push('\n');
        jsonl.push_str(&serde_json::to_string(&EventJ { event }).unwrap());

        let trace = Trace::from_jsonl_str(&jsonl).unwrap();
        assert_eq!(trace.header.label, "fixture-min");
        assert_eq!(trace.events.len(), 1);
        assert_eq!(trace.events[0].direction, Direction::C2s);
        assert_eq!(trace.events[0].messages[0].msg_id, 0x08);
    }

    #[test]
    fn default_policy_exact_match_is_exact() {
        let pol = DefaultPolicy;
        let m = TraceMessage {
            msg_id: 0x05,
            name: Some("createBasePlayer".into()),
            body_hex: "0102030405".into(),
        };
        assert_eq!(pol.compare(&m, &m), Diff::Exact);
    }

    #[test]
    fn default_policy_static_byte_drift_is_regression() {
        let pol = DefaultPolicy;
        let observed = TraceMessage {
            msg_id: 0x05,
            name: Some("createBasePlayer".into()),
            body_hex: "0102030405".into(),
        };
        let recorded = TraceMessage {
            msg_id: 0x05,
            name: Some("createBasePlayer".into()),
            body_hex: "0102030406".into(),
        };
        assert!(matches!(
            pol.compare(&observed, &recorded),
            Diff::Regression(_)
        ));
    }

    #[test]
    fn default_policy_entity_method_same_length_is_drift_not_regression() {
        let pol = DefaultPolicy;
        // Same length, different bytes — server-assigned IDs likely differ.
        let observed = TraceMessage {
            msg_id: 0x82,
            name: None,
            body_hex: "0102030405".into(),
        };
        let recorded = TraceMessage {
            msg_id: 0x82,
            name: None,
            body_hex: "feeddeadbe".into(),
        };
        assert!(matches!(pol.compare(&observed, &recorded), Diff::Drift(_)));
    }

    #[test]
    fn default_policy_entity_method_length_change_is_regression() {
        let pol = DefaultPolicy;
        let observed = TraceMessage {
            msg_id: 0x82,
            name: None,
            body_hex: "0102".into(),
        };
        let recorded = TraceMessage {
            msg_id: 0x82,
            name: None,
            body_hex: "010203".into(),
        };
        assert!(matches!(
            pol.compare(&observed, &recorded),
            Diff::Regression(_)
        ));
    }

    /// Regression guard: `0xFF` is the static `BASEMSG_REPLY_MESSAGE`
    /// (not a dynamic entity-method slot), so a byte drift on it must
    /// fail. A loose `msg_id >= 0x80` check would misclassify this as
    /// drift and let wire-format regressions slip through.
    #[test]
    fn default_policy_msg_0xff_byte_drift_is_regression() {
        let pol = DefaultPolicy;
        let observed = TraceMessage {
            msg_id: 0xFF,
            name: Some("replyMessage".into()),
            body_hex: "01020304".into(),
        };
        let recorded = TraceMessage {
            msg_id: 0xFF,
            name: Some("replyMessage".into()),
            body_hex: "feedbeef".into(),
        };
        assert!(matches!(
            pol.compare(&observed, &recorded),
            Diff::Regression(_)
        ));
    }

    /// Regression guard: producer drift between lowercase and uppercase
    /// hex must not surface as a diff. Both decode to identical bytes.
    #[test]
    fn default_policy_case_insensitive_hex_match() {
        let pol = DefaultPolicy;
        let observed = TraceMessage {
            msg_id: 0x05,
            name: None,
            body_hex: "deadbeef".into(),
        };
        let recorded = TraceMessage {
            msg_id: 0x05,
            name: None,
            body_hex: "DEADBEEF".into(),
        };
        assert_eq!(pol.compare(&observed, &recorded), Diff::Exact);
    }

    /// Regression guard: header.packet_count must equal the actual event
    /// count. A truncated JSONL that claims more would slip past load.
    #[test]
    fn trace_load_rejects_packet_count_mismatch() {
        let header = TraceHeader {
            label: "mismatch".into(),
            source_pcap: "x.pcap".into(),
            session_key_hex: "00".repeat(32),
            client_addr: "127.0.0.1:1".into(),
            server_addr: "127.0.0.1:2".into(),
            packet_count: 99,
            schema_version: TRACE_SCHEMA_VERSION,
        };
        let event = TraceEvent {
            t_seconds: 0.0,
            packet_no: 1,
            direction: Direction::C2s,
            seq: None,
            flags: 0,
            acks: vec![],
            messages: vec![],
        };
        let mut jsonl = String::new();
        jsonl.push_str(&serde_json::to_string(&HeaderJ { header }).unwrap());
        jsonl.push('\n');
        jsonl.push_str(&serde_json::to_string(&EventJ { event }).unwrap());
        let err = Trace::from_jsonl_str(&jsonl).unwrap_err();
        assert!(
            matches!(&err, Error::TraceIo(e) if e.to_string().contains("packet_count mismatch")),
            "expected packet_count mismatch error, got: {err}"
        );
    }

    /// Regression guard: an unsupported `schema_version` must fail at
    /// load time, not silently load as if it were a known version.
    #[test]
    fn trace_load_rejects_unknown_schema_version() {
        let header = TraceHeader {
            label: "future".into(),
            source_pcap: "x.pcap".into(),
            session_key_hex: "00".repeat(32),
            client_addr: "127.0.0.1:1".into(),
            server_addr: "127.0.0.1:2".into(),
            packet_count: 0,
            schema_version: 999,
        };
        let jsonl = serde_json::to_string(&HeaderJ { header }).unwrap();
        let err = Trace::from_jsonl_str(&jsonl).unwrap_err();
        assert!(
            matches!(&err, Error::TraceIo(e) if e.to_string().contains("unsupported schema_version")),
            "expected schema_version error, got: {err}"
        );
    }

    /// Regression guard: an unknown top-level field on an event must
    /// fail at deserialisation. `deny_unknown_fields` makes producer
    /// drift surface here rather than at replay time.
    #[test]
    fn trace_load_rejects_unknown_event_field() {
        let header = TraceHeader {
            label: "drift".into(),
            source_pcap: "x.pcap".into(),
            session_key_hex: "00".repeat(32),
            client_addr: "127.0.0.1:1".into(),
            server_addr: "127.0.0.1:2".into(),
            packet_count: 1,
            schema_version: TRACE_SCHEMA_VERSION,
        };
        let mut jsonl = String::new();
        jsonl.push_str(&serde_json::to_string(&HeaderJ { header }).unwrap());
        jsonl.push('\n');
        // Synthetic event with an unknown field `extra_future_field`.
        jsonl.push_str(r#"{"event":{"t_seconds":0,"packet_no":1,"direction":"c2s","seq":null,"flags":0,"acks":[],"messages":[],"extra_future_field":42}}"#);
        let err = Trace::from_jsonl_str(&jsonl).unwrap_err();
        assert!(
            matches!(&err, Error::TraceJson(_)),
            "expected TraceJson rejection for unknown field, got: {err}"
        );
    }

    #[derive(Serialize)]
    struct HeaderJ {
        header: TraceHeader,
    }
    #[derive(Serialize)]
    struct EventJ {
        event: TraceEvent,
    }
}
