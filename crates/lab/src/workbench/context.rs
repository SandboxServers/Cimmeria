//! Identity and provenance carried with every workbench observation.

use serde::{Deserialize, Serialize};

/// One missing or incomplete layer of capture evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gap {
    pub layer: String,
    pub code: String,
    pub detail: String,
}

/// The identity, timing and known gaps for one captured event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureContext {
    pub schema_version: u32,
    pub instance: String,
    pub client_pid: u32,
    pub client_fingerprint: String,
    pub player_entity_id: Option<u32>,
    pub selected_entity_id: Option<u32>,
    pub server_session_id: Option<String>,
    pub host_ms: i64,
    pub monotonic_ms: u64,
    pub server_offset_ms: Option<i64>,
    pub source: Vec<String>,
    pub gaps: Vec<Gap>,
}

/// A sourced value; absence remains explicit rather than becoming an empty value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation<T> {
    pub source: String,
    pub at_ms: i64,
    pub value: Option<T>,
    pub gap: Option<Gap>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capture_context_matches_literal_json_contract() {
        let fixture = json!({
            "schema_version": 1,
            "instance": "default",
            "client_pid": 42,
            "client_fingerprint": "build-A",
            "player_entity_id": 100,
            "selected_entity_id": null,
            "server_session_id": "session-A",
            "host_ms": 1_000,
            "monotonic_ms": 500,
            "server_offset_ms": null,
            "source": ["client"],
            "gaps": [{"layer": "server", "code": "unavailable", "detail": "no session tap"}]
        });
        let parsed: CaptureContext = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), fixture);

        let unknown: Observation<String> = serde_json::from_value(json!({
            "source": "server", "at_ms": 1_000, "value": null,
            "gap": {"layer": "server", "code": "unavailable", "detail": "no session tap"}
        }))
        .unwrap();
        assert!(unknown.value.is_none());
        assert!(unknown.gap.is_some());
    }
}
