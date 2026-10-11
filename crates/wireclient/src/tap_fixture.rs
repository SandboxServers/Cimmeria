//! The lab packet tap for one new Praxis character, captured on the colo
//! server on 2026-10-10 (ADR § Praxis start). It is the ground truth the
//! client-call builders are pinned to, so a builder that drifts from what
//! the real client sent fails against these bytes rather than against
//! today's server.
//!
//! Inbound records (client to server) keep the entity-id prefix and the
//! `0xBD` sub-slot byte in `args_hex`; outbound records (server to client)
//! hold the arguments only (README F9). Builder tests therefore compare
//! against the whole inbound payload.

use crate::error::{Error, Result};

/// One tap capture: the server's ring buffer dumped to JSON.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TapCapture {
    pub capacity: u32,
    pub count: u32,
    pub dropped: u32,
    pub entity_id: u32,
    pub messages: Vec<TapRecord>,
}

/// Which way a record travelled, from the server's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TapDir {
    In,
    Out,
}

/// One captured message. `args_hex` is the raw payload as the tap saw it.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TapRecord {
    pub ts_ms: u64,
    pub dir: TapDir,
    pub msg_id: Option<u8>,
    pub msg_name: String,
    pub method_index: i32,
    pub target_entity_id: Option<u32>,
    pub args_hex: String,
    pub args_len: usize,
    #[serde(default)]
    pub decoded: Option<serde_json::Value>,
}

impl TapCapture {
    /// Parse a tap dump. A malformed dump is an [`Error::TraceJson`].
    pub fn parse(json: &str) -> Result<TapCapture> {
        serde_json::from_str(json).map_err(Error::TraceJson)
    }

    /// The checked-in Praxis start capture, parsed. Panics on a bad fixture,
    /// which is a broken checkout rather than a runtime condition.
    pub fn praxis_start() -> TapCapture {
        TapCapture::parse(include_str!("../tests/fixtures/praxis_start_tap.json"))
            .expect("praxis_start_tap.json is a valid tap capture")
    }

    /// Record `i`, asserting its `msg_name` is `name`. A fixture whose records
    /// have moved fails here instead of pinning a builder to the wrong bytes.
    pub fn expect(&self, i: usize, name: &str) -> &TapRecord {
        let rec = &self.messages[i];
        assert_eq!(rec.msg_name, name, "fixture record #{i} moved");
        rec
    }
}

impl TapRecord {
    /// The raw payload bytes, decoded from `args_hex`.
    pub fn args(&self) -> Vec<u8> {
        hex::decode(&self.args_hex).expect("args_hex is valid hex")
    }
}

#[cfg(test)]
mod tests {
    use super::TapCapture;

    /// The Praxis capture holds the 84 records the client-call tests index
    /// into, all for entity 8, with nothing dropped from the ring buffer.
    #[test]
    fn praxis_fixture_loads_84_records_for_entity_8() {
        let cap = TapCapture::praxis_start();
        assert_eq!(cap.count, 84);
        assert_eq!(cap.messages.len(), 84);
        assert_eq!(cap.entity_id, 8);
        assert_eq!(cap.dropped, 0);
    }
}
