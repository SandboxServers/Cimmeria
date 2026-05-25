//! Handler for `BaseToCellMsg::RequestEntityUpdate` — re-emits CREATE_ENTITY
//! + cascade to a player for entities they claim to be missing.
//!
//! Wire-level context: the client sends `requestEntityUpdate` (msg `0x07`)
//! when it suspects it is missing or has stale state for one or more entities.
//! This is the canonical recovery path when a `createEntity` (`0x09`) for an
//! NPC was dropped on the wire past the 20-retry lifetime cap — without the
//! re-emit, the NPC stays permanently invisible on that client. See issue
//! #289 and audit finding N3.
//!
//! Security: only entities currently in the witness's AoI are re-emitted.
//! Out-of-AoI requests are dropped silently — the client must not be able to
//! probe arbitrary entity ids.

use cimmeria_common::EntityId;
use tokio::sync::mpsc;

use super::super::super::messages::{CellToBaseMsg, NpcAoIData};
use super::super::super::space_manager::SpaceManager;

/// Re-emit `EnteredAoI` for each `entity_id` that's currently in
/// `witness_id`'s witness set. Unknown / out-of-AoI ids are skipped.
pub(super) async fn handle(
    witness_id: u32,
    entity_ids: Vec<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let requested = entity_ids.len();

    // The witness must exist in some space and be a player — otherwise no
    // AoI bookkeeping exists to authorise against.
    let Some(witness) = space_mgr.get_entity(witness_id) else {
        tracing::warn!(
            witness_id,
            requested,
            reason = "witness_not_in_space",
            "RequestEntityUpdate: witness entity not found — dropping"
        );
        return;
    };
    let Some(space_id) = space_mgr.get_entity_space_id(witness_id) else {
        tracing::warn!(
            witness_id,
            requested,
            reason = "witness_no_space_id",
            "RequestEntityUpdate: witness has no space_id — dropping"
        );
        return;
    };
    // Snapshot the witness set up front so the per-id loop only does
    // cheap set lookups (and isn't surprised by mid-iteration AoI ticks).
    let witnesses: std::collections::HashSet<EntityId> =
        witness.witnesses.iter().copied().collect();

    let mut emitted = 0usize;
    let mut skipped_unknown = 0usize;
    let mut skipped_not_in_aoi = 0usize;

    for entity_id in entity_ids {
        let target_eid = EntityId(entity_id as i32);
        if !witnesses.contains(&target_eid) {
            // Either the client raced an AoI-leave or it's probing an id it
            // shouldn't see. Either way: drop.
            skipped_not_in_aoi += 1;
            continue;
        }
        let Some(other) = space_mgr.get_entity(entity_id) else {
            skipped_unknown += 1;
            continue;
        };
        let npc_data = if !other.is_player {
            Some(NpcAoIData {
                name_id: other.name_id,
                faction: other.faction,
                alignment: other.alignment,
                entity_flags: other.entity_flags,
                interaction_type: other.interaction_type_flags,
                speaker_id: other.speaker_id,
                event_set_id: other.event_set_id,
                static_mesh: other.static_mesh.clone(),
                body_set: other.body_set.clone(),
                components: other.components.clone(),
            })
        } else {
            None
        };
        let msg = CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id,
            space_id,
            class_id: other.class_id,
            position: [other.position.x, other.position.y, other.position.z],
            direction: [other.direction.x, other.direction.y, other.direction.z],
            level: other.level,
            npc_data,
        };
        if let Err(e) = tx.send(msg).await {
            tracing::warn!(
                witness_id,
                entity_id,
                "RequestEntityUpdate: EnteredAoI re-emit cell\u{2192}base send failed: {e}"
            );
            // Bail — the channel is closed, no point processing further ids.
            return;
        }
        emitted += 1;
    }

    tracing::info!(
        witness_id,
        requested,
        emitted,
        skipped_not_in_aoi,
        skipped_unknown,
        "RequestEntityUpdate handled"
    );
}
