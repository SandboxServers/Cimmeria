//! Cell-tick drain for armed stargate dials — the Rust shape of
//! `SGWPlayer.gateDialTimerExpired` (`SGWPlayer.py:2105-2114`).
//!
//! The 2009 server armed `Atrea.addTimer(now + 4.0, ...)` per dial. We
//! drain a deadline map on the existing 100ms cell tick instead, for the
//! same reason C08a's `deferred_content_action_tick` does: a spawned
//! `tokio` task cannot reach `&mut SpaceManager`, which the
//! single-threaded cell message loop owns. The 100ms granularity is
//! invisible against a 4s timer, and the drain inherits the
//! `destroy_entity` / `disconnect_entity` cancellation for free.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::sequences::{send_gate_sequence, EVENT_STARGATE_MAKE_GATE};

/// Open every gate whose 4-second dial timer has elapsed.
///
/// Fires `onSequence(Stargate_MakeGate)` to the dialer and their
/// witnesses, and marks the dial passable so the gate-region crossing
/// will travel. Exactly once per dial — `take_opened_gate_dials` flips
/// `passable` as it yields, mirroring `gateDialTimerExpired` clearing
/// `gateDialTimer` before it sends.
pub async fn gate_dial_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let opened = space_mgr.take_opened_gate_dials(std::time::Instant::now());
    if opened.is_empty() {
        return;
    }

    for (entity_id, dial) in opened {
        tracing::info!(
            entity_id,
            target_address_id = dial.target_address_id,
            target_world = %dial.target_world_name,
            "Gate dial timer expired — opening gate"
        );
        send_gate_sequence(
            entity_id,
            dial.origin_event_set_id,
            EVENT_STARGATE_MAKE_GATE,
            tx,
            space_mgr,
        )
        .await;
    }
}
