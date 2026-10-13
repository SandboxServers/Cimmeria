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

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::PathStatus;
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
        LabQuery::AbilityState { entity_id } => Ok(LabQueryReply::AbilityState {
            state: space_mgr.ability_state(entity_id),
        }),
        LabQuery::NavPath { space_id, from, to } => {
            if !from.into_iter().chain(to).all(f32::is_finite) {
                Err("nav path coordinates must be finite".into())
            } else {
                let space = space_mgr
                    .spaces
                    .get(&space_id)
                    .ok_or_else(|| format!("space {space_id} is not loaded"));
                space.and_then(|space| {
                    let nav = space
                        .navmesh
                        .as_ref()
                        .ok_or_else(|| format!("space {space_id} has no navmesh"))?;
                    let from = Vector3::new(from[0], from[1], from[2]);
                    let to = Vector3::new(to[0], to[1], to[2]);
                    let outcome = nav.find_path(&from, &to);
                    let corners = if outcome.status == PathStatus::Ok {
                        outcome.waypoints.iter().map(|p| [p.x, p.y, p.z]).collect()
                    } else {
                        Vec::new()
                    };
                    Ok(LabQueryReply::NavPath {
                        status: outcome.status.label().into(),
                        waypoints: corners,
                        start_snap: outcome.start_snap.map(|p| [p.x, p.y, p.z]),
                        end_snap: outcome.end_snap.map(|p| [p.x, p.y, p.z]),
                    })
                })
            }
        }
    };

    // The receiver is dropped only if the lab endpoint gave up waiting (client
    // disconnected / timed out). A read-only query has no side effects to
    // unwind, so a dropped reply is a non-event — swallow it.
    let _ = reply_tx.send(result);
}
