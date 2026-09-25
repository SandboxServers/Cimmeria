//! Client-entity resync — the per-entity caches the client keeps on its
//! player entity and rebuilds only from the server.
//!
//! Two callers share these helpers:
//!
//! - [`super::handle_init_player_state`] on initial login / gate arrival,
//!   where the client entity has just been created for the first time.
//! - `combat::respawn::handle_respawn` after the same-world reanchor. The
//!   reanchor burst re-issues `CREATE_BASE_PLAYER` to drop ragdoll, and the
//!   client answers by destroying and re-creating its player entity, which
//!   empties every cache the login burst had populated. The inventory
//!   snapshot was the first casualty found (empty bag until relog), the
//!   region-hint list the second (no region triggers for the rest of the
//!   session). Both are already replayed after the reanchor; this module
//!   covers the rest of the same wipe: the hotbar ability list, the active
//!   bandolier slot, the mission journal and the `state_field` preference
//!   bits.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Re-send `onActiveSlotUpdate` for the bandolier.
///
/// On login this is a defensive resync against a client-side initialization
/// race: the login burst already carries the packet, but the client's NetIn
/// handler silently no-ops when the bag-list map is not yet initialized,
/// leaving the cached active slot at 0 and the `ActivateBandolierSlotN` Lua
/// gate misfiring ("F2 doesn't swap to the P90"). Sent after `onClientReady`
/// the bag list is guaranteed to exist. After a pawn recreate it is the only
/// copy the new entity ever gets. Ghidra detail in
/// docs/reverse-engineering/findings/client-wire-emit-suppression.md.
///
/// Wire: bag_id (i32 LE) + (slot_id + 1) (i32 LE, 1-indexed) = 8 bytes.
pub(crate) async fn send_active_slot_resend(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    const CONTAINER_BANDOLIER: i32 = 3;
    let active_slot = space_mgr
        .get_entity(entity_id)
        .map(|e| e.active_bandolier_slot)
        .unwrap_or(0);
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&CONTAINER_BANDOLIER.to_le_bytes());
    args.extend_from_slice(&(active_slot + 1).to_le_bytes());
    crate::cell::abilities::send_entity_method(
        entity_id,
        crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE,
        args,
        tx,
        space_mgr,
    )
    .await;
    tracing::info!(
        target: "bandolier.resend",
        entity_id,
        active_slot,
        "Re-sent onActiveSlotUpdate post-onClientReady (defensive resync \
         against client bag-list init race — see \
         docs/reverse-engineering/findings/client-wire-emit-suppression.md)"
    );
}

/// Replay the remaining per-entity client caches after a
/// `CREATE_BASE_PLAYER` re-issue (same-world respawn reanchor).
///
/// Order:
/// 1. `onKnownAbilitiesUpdate` — hotbar.
/// 2. `onActiveSlotUpdate` — bandolier slot.
/// 3. Mission journal (`onMissionUpdate` / `onStepUpdate` /
///    `onObjectiveUpdate` for every active, visible mission).
/// 4. The full `state_field`, only when non-zero. The client initialises
///    its cached copy to 0 on entity creation and applies updates as an
///    XOR delta against it, so a preserved `BSF_AutoCycling` needs an
///    explicit re-broadcast or the button highlight stays off until the
///    next toggle. A zero field needs no packet.
///
/// Region hints and the inventory snapshot are not repeated here: the
/// respawn handler already queues both right behind the reanchor.
///
/// Every message targets the player's own entity and is queued on the
/// same reliable channel *after* the reanchor, so it lands once the
/// client's creation transaction has settled — the same reason the
/// reanchor's appearance replay travels as a separate bundle.
pub(crate) async fn resync_after_pawn_recreate(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(state_field) = space_mgr.get_entity(entity_id).map(|e| e.state_field) else {
        tracing::warn!(
            entity_id,
            reason = "entity_missing",
            "resync after pawn recreate skipped — entity not found"
        );
        return;
    };

    super::send_known_abilities_update(entity_id, tx, space_mgr).await;
    send_active_slot_resend(entity_id, tx, space_mgr).await;
    crate::cell::missions::resend_missions(entity_id, tx, space_mgr).await;

    if state_field != 0 {
        crate::cell::abilities::send_entity_method(
            entity_id,
            crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
            state_field.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }

    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        state_field,
        "Resynced client entity state after pawn recreate (hotbar, active slot, \
         journal, state_field)"
    );
}
