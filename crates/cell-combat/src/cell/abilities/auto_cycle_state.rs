//! The one exit for a `BSF_AUTO_CYCLING` transition: tell the player's client.
//!
//! Every site that arms or clears the auto-cycle loop calls
//! [`send_auto_cycle_state`] with the new `state_field`: the `setAutoCycle`
//! toggle, the first-commit arm, a manual override, `AF_DEACTIVATE_AUTO_CYCLE`,
//! an interrupted cast, a bandolier swap, the target's death or surrender, the
//! player's own death and the tick's stop reasons.
//!
//! The bit is not saved: every login starts with the loop off (owner decision
//! 2026-10-03). #412 had saved the button press only, so a loop the server
//! later stopped still read as on in `sgw_player.state_field`, and the next
//! login lit the button and armed a loop the player had watched switch off.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::messaging::WireRoute;
use super::wire_ledger::{self, WireCtx};

/// Broadcast `onStateFieldUpdate` after a `BSF_AUTO_CYCLING` transition.
///
/// Self-only routing (like `BSF_InCombat` changes), kept in one place so arm
/// and clear sites cannot drift apart on the wire rule.
pub async fn send_auto_cycle_state(
    entity_id: u32,
    new_state: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    wire_ledger::send(
        entity_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        new_state.to_le_bytes().to_vec(),
        WireRoute::EntityDefault,
        WireCtx::new("auto_cycle").reason("auto_cycle"),
        tx,
        space_mgr,
    )
    .await;
}
