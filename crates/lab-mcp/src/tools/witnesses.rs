//! `server_witnesses` tool logic (issue #688).
//!
//! Reports the bidirectional witness relationship for one entity — who
//! currently has it in their AoI, and (for a player) whom it sees. This is the
//! direct answer to the invisible-corpse class of AoI bug this phase targets:
//! "does the server think entity X is witnessed by player Y?"
//!
//! Every entity id carries its names (Rule 6, NT-41): the list entries are
//! `{ entity_id, entity_name?, template_id?, template_name? }`.

use serde_json::{json, Value};

use cimmeria_names::NameBook;
use cimmeria_services::cell::messages::{LabQuery, LabQueryReply};

use super::entities::unexpected_reply;
use super::names::name_witness_report;
use crate::state::LabState;

/// Bidirectional witness report for `entity_id`.
pub async fn witnesses(state: &LabState, entity_id: u32) -> Result<Value, String> {
    let reply = state.lab_query(LabQuery::Witnesses { entity_id }).await?;
    shape(reply, &cimmeria_names::book())
}

/// The tool's JSON for a cell reply, named from `book`.
pub(super) fn shape(reply: LabQueryReply, book: &NameBook) -> Result<Value, String> {
    match reply {
        LabQueryReply::Witnesses { mut report } => {
            name_witness_report(&mut report, book);
            let mut out = json!({
                "entity_id": report.entity_id,
                "space_id": report.space_id,
                "witnessed_by_count": report.witnessed_by.len(),
                "witnesses_count": report.witnesses.len(),
                "witnessed_by": report.witnessed_by,
                "witnesses": report.witnesses,
            });
            // The report's own names and world, each left out when unresolved.
            if let (Value::Object(out), Ok(Value::Object(names))) =
                (&mut out, serde_json::to_value(&report.names))
            {
                out.extend(names);
                if let Some(world) = report.world {
                    out.insert("world".to_string(), json!(world));
                }
            }
            Ok(out)
        }
        other => Err(unexpected_reply(&other)),
    }
}

#[cfg(test)]
#[path = "witnesses_tests.rs"]
mod tests;
