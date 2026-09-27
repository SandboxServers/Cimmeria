//! The departing-player hook.
//!
//! Splits the two shapes of "a participant went away": a real client
//! disconnect, which can and must release the survivors synchronously, and
//! everything else, which goes through the synchronous
//! `SpaceManager::destroy_entity` and is reconciled by the ring tick (`cimmeria_cell_content::cell::ring_transport::runtime`).

use tokio::sync::mpsc;

use super::super::transporter::{AbortReason, Effect};
use super::super::wire_helpers::{send_visible, update_state_flag, BSF_MOVEMENT_LOCK};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A participating player's cell entity is going away for real (client
/// disconnect). Release every ring that was holding them and dispatch the
/// survivors' release effects **synchronously**, before the caller's AoI
/// teardown runs.
///
/// Called from `SpaceManager::disconnect_entity`. The synchronous
/// `destroy_entity` path cannot do this (no `tx`) and defers to the tick via
/// [`super::super::transporter::RingTransporterManager::note_player_gone`]; that
/// path is deliberately source-side only, so this is the one entry point
/// that also reconciles the destination's expected-passenger set.
pub async fn forget_player(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let (emptied, rechecks) = space_mgr
        .ring_transporters
        .forget_participant_everywhere(entity_id);
    for region_id in emptied {
        let effects = space_mgr.ring_transporters.abort_pair(
            region_id,
            AbortReason::PlayerGone,
            Some(entity_id),
        );
        dispatch_release_effects(effects, tx, space_mgr).await;
    }
    for region_id in rechecks {
        // Co-travellers survive: the expectation shrank by one, so the
        // readiness equality may now hold. Queue the re-check for the tick
        // rather than running it here — `try_advance_after_load` can reach
        // `Effect::FireTeleportIn`, which needs the `ChainEngine`, and this
        // entry point is called from `SpaceManager::disconnect_entity`,
        // which has none. Unlike the release effects above, a ≤100ms delay
        // on "the remaining travellers may now be ready" is harmless: it
        // advances a healthy trip, it does not un-stick a stuck player.
        tracing::debug!(
            region_id,
            entity_id,
            reason = "participant_gone",
            "ring: passenger removed from destination expectation — queued a load-readiness \
             re-check for the remaining travellers"
        );
        space_mgr.ring_transporters.note_load_recheck(region_id);
    }
}

/// Dispatch the release effects an abort produces — `ShowPlayer` and
/// `UnlockMovement` only.
///
/// Exists because the abort paths reachable from
/// `SpaceManager::disconnect_entity` have no
/// [`ChainEngine`](cimmeria_content_engine::chain::ChainEngine) in hand, and
/// `dispatch::dispatch_effect` needs one for `Effect::FireTeleportIn`. Restricting
/// the accepted set is the point, not a limitation: an abort must never fire
/// arrival content for a trip that did not arrive. Anything else in the list
/// is an FSM bug and is logged rather than silently skipped.
pub async fn dispatch_release_effects(
    effects: Vec<Effect>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for effect in effects {
        match effect {
            Effect::ShowPlayer { entity_id } => {
                send_visible(entity_id, true, tx, space_mgr).await;
                // An abort after the owner was already moved (the remote
                // load wait timed out) still brings its pets; an abort
                // before the move leaves them (pets PT-02).
                crate::cell::pets::on_owner_reappeared(
                    entity_id,
                    crate::cell::pets::OwnerPath::Ring,
                    tx,
                    space_mgr,
                )
                .await;
            }
            Effect::UnlockMovement { entity_id } => {
                update_state_flag(entity_id, BSF_MOVEMENT_LOCK, false, tx, space_mgr).await;
            }
            other => {
                tracing::error!(
                    effect = ?other,
                    reason = "non_release_effect_in_abort",
                    "ring abort: FSM produced an effect that is not a player release — \
                     dropped, because the abort path has no ChainEngine and must not fire \
                     arrival content for a trip that never arrived"
                );
            }
        }
    }
}
