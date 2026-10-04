//! `server_ability_state` tool logic (ability-mechanics AB-L1).
//!
//! Sends [`LabQuery::AbilityState`] to the cell loop and returns AB-T5's
//! snapshot of one entity: the warmup, cooldowns, pulsing effects, ledger
//! entries and absorb pools, `state_field` refcounts and every stat. It is
//! the same builder the `abilities.snapshot` row and the GM `.effects`
//! readout use, so the three never disagree. Read-only.

use serde_json::{json, Value};

use cimmeria_services::cell::messages::{LabQuery, LabQueryReply};

use crate::state::LabState;

/// One entity's ability state. Returns `{ "state": <snapshot|null> }`.
pub async fn ability_state(state: &LabState, entity_id: u32) -> Result<Value, String> {
    shape(
        state
            .lab_query(LabQuery::AbilityState { entity_id })
            .await?,
    )
}

/// The tool's JSON for a cell reply. Split from [`ability_state`] so the
/// shaping is testable without a running cell loop.
fn shape(reply: LabQueryReply) -> Result<Value, String> {
    match reply {
        LabQueryReply::AbilityState { state } => Ok(json!({ "state": state })),
        other => Err(format!("unexpected LabQuery reply variant: {other:?}")),
    }
}

#[cfg(test)]
#[path = "abilities_tests.rs"]
mod tests;
