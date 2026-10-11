//! `BaseToCellMsg::GmSpawnNpcReady` handler — base resolved a `gmSpawnByCmd`
//! template into a `SpawnRecord`; this allocates an NPC id, drops it into the
//! target space, and sends the GM the completion feedback. Extracted from
//! `base_messages/mod.rs` as a pure code move.
//!
//! A batched copy (`.spawnrandom`, CH-13) reports to its batch instead
//! (`console::spawn_batch`), and so does `BaseToCellMsg::GmSpawnNpcFailed`,
//! the base's "this copy failed" answer; the batch sends one summary line.

use tokio::sync::mpsc;

use crate::cell::console::spawn_batch::{self, Recorded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;

/// Handle `BaseToCellMsg::GmSpawnNpcReady`.
pub(super) async fn handle_gm_spawn_npc_ready(
    record: Box<spawner::SpawnRecord>,
    space_id: u32,
    requester_entity_id: u32,
    batch: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Base resolved the gmSpawnByCmd template into a SpawnRecord —
    // allocate an NPC id and drop it into the target space. AoI fanout
    // handles client visibility on the next tick, so there's no extra
    // send here (same as DB-seeded NPC spawns).
    let id = space_mgr.allocate_npc_id();
    match space_mgr.spawn_npc_from_record_in_space(id, &record, space_id) {
        // `record` is a Box<SpawnRecord>; `&record` coerces to
        // `&SpawnRecord` via auto-deref at the call site.
        Ok(placed_space) => {
            tracing::info!(
                npc_entity_id = id,
                npc_entity_name = space_mgr.entity_label(id),
                template_id = record.template_id,
                template_name = %record.template_name,
                space_id = placed_space,
                world = space_mgr.world_name_for_space(placed_space),
                x = record.x,
                y = record.y,
                z = record.z,
                batch,
                "GmSpawnNpcReady: NPC spawned"
            );
            if let Some(batch) = batch {
                match spawn_batch::batches(space_mgr).spawned(batch, id) {
                    Recorded::Pending => return,
                    Recorded::Completed(done) => {
                        spawn_batch::deliver(done, tx, space_mgr).await;
                        return;
                    }
                    // The batch already reported (timed out): this copy
                    // still spawned, so say so on its own line.
                    Recorded::UnknownBatch => {
                        crate::cell::console::gm::feedback::send_gm_feedback(
                            requester_entity_id,
                            &format!(
                                "spawned npc {id} (template {}) after its .spawnrandom report",
                                record.template_id
                            ),
                            tx,
                        )
                        .await;
                        return;
                    }
                }
            }
            // Definitive success line: the spawn actually took, so the
            // GM gets confirmation with the real new NPC id. This is
            // the cell-side completion of the spawn round-trip (the
            // cell-side "requested" optimism was removed).
            //
            // Command-neutral wording: the same round-trip serves the native
            // `gmSpawnByCmd` and the dot-console `.spawn`, so naming one of
            // them would misreport which command the GM actually typed.
            crate::cell::console::gm::feedback::send_gm_feedback(
                requester_entity_id,
                &format!("spawned npc {id} (template {})", record.template_id),
                tx,
            )
            .await;
        }
        Err(e) => {
            tracing::warn!(
                npc_entity_id = id, // nt:id-only the spawn failed, so no entity holds this id
                template_id = record.template_id,
                template_name = %record.template_name,
                space_id,
                world = space_mgr.world_name_for_space(space_id),
                batch,
                "GmSpawnNpcReady: spawn failed: {e}"
            );
            if let Some(batch) = batch {
                handle_gm_spawn_npc_failed(
                    requester_entity_id,
                    batch,
                    record.template_id,
                    "could not place it in the space".to_string(),
                    tx,
                    space_mgr,
                )
                .await;
                return;
            }
            crate::cell::console::gm::feedback::send_gm_feedback(
                requester_entity_id,
                &format!("spawn failed for template {}", record.template_id),
                tx,
            )
            .await;
        }
    }
}

/// Handle `BaseToCellMsg::GmSpawnNpcFailed`: one copy of a `.spawnrandom`
/// batch failed. Counted against its batch; a batch that already reported
/// gets the failure as its own line.
pub(super) async fn handle_gm_spawn_npc_failed(
    requester_entity_id: u32,
    batch: u32,
    template_id: i32,
    reason: String,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match spawn_batch::batches(space_mgr).failed(batch, &reason) {
        Recorded::Pending => {}
        Recorded::Completed(done) => spawn_batch::deliver(done, tx, space_mgr).await,
        Recorded::UnknownBatch => {
            crate::cell::console::gm::feedback::send_gm_feedback(
                requester_entity_id,
                &format!("spawn failed for template {template_id}: {reason}"),
                tx,
            )
            .await;
        }
    }
}
