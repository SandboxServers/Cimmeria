//! `server_witnesses` tool logic (issue #688).
//!
//! Reports the bidirectional witness relationship for one entity — who
//! currently has it in their AoI, and (for a player) whom it sees. This is the
//! direct answer to the invisible-corpse class of AoI bug this phase targets:
//! "does the server think entity X is witnessed by player Y?"

use serde_json::{json, Value};

use cimmeria_services::cell::messages::{LabQuery, LabQueryReply};

use crate::state::LabState;

/// Bidirectional witness report for `entity_id`.
pub async fn witnesses(state: &LabState, entity_id: u32) -> Result<Value, String> {
    match state.lab_query(LabQuery::Witnesses { entity_id }).await? {
        LabQueryReply::Witnesses { report } => Ok(json!({
            "entity_id": report.entity_id,
            "space_id": report.space_id,
            "witnessed_by": report.witnessed_by,
            "witnessed_by_count": report.witnessed_by.len(),
            "witnesses": report.witnesses,
            "witnesses_count": report.witnesses.len(),
        })),
        other => Err(format!("unexpected LabQuery reply variant: {other:?}")),
    }
}
