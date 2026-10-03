//! The one exit for a `BSF_AUTO_CYCLING` transition: tell the player's client
//! and save the bit for the next login.
//!
//! Every site that arms or clears the auto-cycle loop calls
//! [`send_auto_cycle_state`] with the new `state_field`: the `setAutoCycle`
//! toggle, the first-commit arm, a manual override, `AF_DEACTIVATE_AUTO_CYCLE`,
//! an interrupted cast, a bandolier swap, the target's death or surrender, the
//! player's own death and the tick's stop reasons. The saved value is therefore
//! always the value the button last showed.
//!
//! Saving only the explicit toggle (the #412 shape) let the two drift: a loop
//! the server stopped (target died, friendly NPC selected) still read as on in
//! `sgw_player.state_field`, so the next login lit the button and armed the
//! loop the player had watched switch off. Seen on colo 2026-10-03.

use tokio::sync::mpsc;

use super::super::combat::PERSISTED_STATE_FIELD_MASK;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::messaging::send_entity_method;

/// Broadcast `onStateFieldUpdate` after a `BSF_AUTO_CYCLING` transition and
/// save the persisted bits.
///
/// Self-only routing (like `BSF_InCombat` changes), so arm and clear sites
/// cannot drift apart on the wire rule. The save masks with
/// [`PERSISTED_STATE_FIELD_MASK`], so the transient combat bits riding the
/// same value (`BSF_Dead`, `BSF_InCombat`, `BSF_MovementLock`) never reach the
/// database: a relog stays a clean combat slate. NPCs and fixtures without a
/// `player_id` are broadcast to but not saved.
pub async fn send_auto_cycle_state(
    entity_id: u32,
    new_state: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        new_state.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;
    let Some(player_id) = space_mgr.get_entity(entity_id).and_then(|e| e.player_id) else {
        return;
    };
    let state_field = new_state & PERSISTED_STATE_FIELD_MASK;
    if let Err(e) = tx
        .send(CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field,
        })
        .await
    {
        // Channel closed: the bit still applies for this session but the
        // next login restores the previous value.
        tracing::warn!(
            entity_id,
            player_id,
            state_field,
            error = %e,
            "StateFieldUpdate send to base failed -- auto-cycle state not persisted"
        );
    }
}
