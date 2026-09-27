//! Cell-tick drains for the stargate dial timer and the post-crossing
//! cinematic hold.
//!
//! Both durations (`GATE_DIAL_DURATION`, `CROSSING_CINEMATIC_HOLD`) are
//! drained on the existing 100ms cell tick rather than a spawned `tokio`
//! task, for the same reason C08a's `deferred_content_action_tick` does: a
//! spawned task cannot reach `&mut SpaceManager`, which the
//! single-threaded cell message loop owns exclusively. The 100ms
//! granularity is invisible against either duration, and the drain
//! inherits `destroy_entity` / `disconnect_entity` cancellation for free.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::sequences::{send_gate_sequence, set_crossing_movement_lock, EVENT_STARGATE_MAKE_GATE};

/// Open every gate whose dial timer has elapsed.
///
/// Fires `onSequence(Stargate_MakeGate)` to the dialer and their
/// witnesses, and marks the dial passable so the gate-region crossing
/// will travel. Exactly once per dial — `take_opened_gate_dials` flips
/// `passable` as it yields, mirroring the pre-NA35 `gateDialTimerExpired`
/// shape (clear the timer before sending).
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

/// Run the deferred world transition for every crossing whose
/// `CROSSING_CINEMATIC_HOLD` has elapsed (NA35).
///
/// `handle_stargate_region_entered` already sent `Stargate_CrossGate` /
/// `onStargatePassage` and locked the traveller's movement before arming
/// the hold; this is the other half. On a **successful** deferred travel
/// the entity is torn down by `perform_gate_travel` itself, which
/// implicitly clears the movement lock (there is no entity left to hold a
/// state flag). On a **failed** one — the destination vanished from the
/// stargate cache, the arrival is unrecoverably off-mesh, or the base
/// channel is closed — the traveller is left in place by design (see
/// `perform_gate_travel`'s arrival-refusal comment), so the lock MUST be
/// cleared here or the player would be stuck immobile forever with no
/// travel to blame it on.
pub async fn crossing_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let ready = space_mgr.take_ready_crossings(std::time::Instant::now());
    if ready.is_empty() {
        return;
    }

    for (entity_id, crossing) in ready {
        tracing::info!(
            entity_id,
            target_address_id = crossing.target_address_id,
            "Crossing hold elapsed — running the deferred world transition"
        );
        let travelled =
            super::perform_gate_travel(entity_id, crossing.target_address_id, tx, space_mgr).await;
        if !travelled {
            tracing::warn!(
                entity_id,
                target_address_id = crossing.target_address_id,
                reason = "deferred_travel_failed",
                "gate crossing: deferred travel failed after the cinematic \
                 hold — releasing the movement lock so the traveller is \
                 not stuck immobile with no completed travel"
            );
            set_crossing_movement_lock(entity_id, false, tx, space_mgr).await;
        }
    }
}
