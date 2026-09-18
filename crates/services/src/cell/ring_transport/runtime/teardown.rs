//! The departing-player hook.
//!
//! Splits the two shapes of "a participant went away": a real client
//! disconnect, which can and must release the survivors synchronously, and
//! everything else, which goes through the synchronous
//! `SpaceManager::destroy_entity` and is reconciled by [`super::tick`].

use tokio::sync::mpsc;

use super::super::dispatch::dispatch_release_effects;
use super::super::transporter::AbortReason;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A participating player's cell entity is going away for real (client
/// disconnect). Release every ring that was holding them and dispatch the
/// survivors' release effects **synchronously**, before the caller's AoI
/// teardown runs.
///
/// Called from `SpaceManager::disconnect_entity`. The synchronous
/// `destroy_entity` path cannot do this (no `tx`) and defers to the tick via
/// [`super::transporter::RingTransporterManager::note_player_gone`]; that
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
