//! `BaseToCellMsg::LabQuery` handler — read-only live-state snapshots for the
//! live-research-lab MCP endpoint (issue #688, phase 5).
//!
//! This is the read side of the `CreateEntity`/`LabConsoleExec` request/reply
//! precedent: the lab endpoint (base-side) sends a [`LabQuery`] with a `oneshot`
//! reply channel; this handler answers it against the loop-owned
//! `SpaceManager`. It takes `&SpaceManager`, never `&mut` — no simulation state
//! is touched. Result sizes are bounded by the snapshot builders
//! (`SpaceManager::lab_query_entities` caps at `LAB_ENTITY_QUERY_CAP`), so a
//! query can never stall the tick.

use tokio::sync::oneshot;

use crate::cell::messages::{LabQuery, LabQueryReply, LabQueryResult};
use crate::cell::space_manager::SpaceManager;

/// Handle `BaseToCellMsg::LabQuery`. Synchronous and allocation-light: it reads
/// the space manager and answers on the reply channel. Not `async` — there is
/// nothing to await, and keeping it sync makes the read-only guarantee obvious.
pub(super) fn handle_lab_query(
    query: LabQuery,
    reply_tx: oneshot::Sender<LabQueryResult>,
    space_mgr: &SpaceManager,
) {
    let result: LabQueryResult = match query {
        LabQuery::EntityGet { entity_id } => Ok(LabQueryReply::Entity {
            entity: space_mgr.lab_entity_snapshot(entity_id),
        }),
        LabQuery::EntityQuery { filter } => space_mgr.lab_query_entities(&filter),
        LabQuery::Witnesses { entity_id } => match space_mgr.lab_witness_report(entity_id) {
            Some(report) => Ok(LabQueryReply::Witnesses { report }),
            None => Err(format!("entity {entity_id} does not exist")),
        },
    };

    // The receiver is dropped only if the lab endpoint gave up waiting (client
    // disconnected / timed out). A read-only query has no side effects to
    // unwind, so a dropped reply is a non-event — swallow it.
    let _ = reply_tx.send(result);
}
