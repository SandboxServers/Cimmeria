//! `BaseToCellMsg::GmSpawnNpcReady` handler — base resolved a `gmSpawnByCmd`
//! template into a `SpawnRecord`; this allocates an NPC id, drops it into the
//! target space, and sends the GM the completion feedback. Extracted from
//! `base_messages/mod.rs` as a pure code move.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;

/// Handle `BaseToCellMsg::GmSpawnNpcReady`.
pub(super) async fn handle_gm_spawn_npc_ready(
    record: Box<spawner::SpawnRecord>,
    space_id: u32,
    requester_entity_id: u32,
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
                template_id = record.template_id,
                template_name = %record.template_name,
                space_id = placed_space,
                x = record.x,
                y = record.y,
                z = record.z,
                "GmSpawnNpcReady: NPC spawned"
            );
            // Definitive success line: the spawn actually took, so the
            // GM gets confirmation with the real new NPC id. This is
            // the cell-side completion of the gmSpawnByCmd round-trip
            // (the cell-side "requested" optimism was removed).
            crate::cell::cell_methods::gm::feedback::send_gm_feedback(
                requester_entity_id,
                &format!(
                    "gmSpawnByCmd: spawned npc {id} (template {})",
                    record.template_id
                ),
                tx,
            )
            .await;
        }
        Err(e) => {
            tracing::warn!(
                npc_entity_id = id,
                template_id = record.template_id,
                space_id,
                "GmSpawnNpcReady: spawn failed: {e}"
            );
            crate::cell::cell_methods::gm::feedback::send_gm_feedback(
                requester_entity_id,
                &format!(
                    "gmSpawnByCmd: spawn failed for template {}",
                    record.template_id
                ),
                tx,
            )
            .await;
        }
    }
}
