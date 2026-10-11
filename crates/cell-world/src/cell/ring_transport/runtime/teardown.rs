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
            region_name = space_mgr.ring_transporters.region_name(region_id),
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
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
                let who = space_mgr.player_identity(entity_id);
                tracing::info!(
                    event = "ring.passenger_released",
                    entity_id,
                    entity_name = space_mgr.entity_label(entity_id),
                    account_id = who.account_id,
                    account_name = who.account_name,
                    player_id = who.player_id,
                    player_name = who.player_name,
                    world = space_mgr
                        .get_entity_space_id(entity_id)
                        .and_then(|sid| space_mgr.world_name_for_space(sid)),
                    "ring: passenger released from transport"
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::make_space_manager_with_player;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// One release row per ring passenger: the `ShowPlayer` arm logs the
    /// passenger's identity; the `UnlockMovement` arm adds nothing.
    #[tokio::test]
    async fn release_logs_one_row_per_passenger_with_account_id() {
        let mut mgr = make_space_manager_with_player(7);
        mgr.get_entity_mut(7).unwrap().account_id = Some(42);
        let (tx, _rx) = mpsc::channel(8);
        let capture = LogCapture::install();

        dispatch_release_effects(
            vec![
                Effect::ShowPlayer { entity_id: 7 },
                Effect::UnlockMovement { entity_id: 7 },
            ],
            &tx,
            &mut mgr,
        )
        .await;

        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "ring.passenger_released"))
            .collect();
        assert_eq!(rows.len(), 1, "one release row per passenger: {rows:#?}");
        assert_eq!(rows[0].level, Level::INFO);
        assert!(
            rows[0].has_field("account_id", "42"),
            "release row must carry account_id: {rows:#?}"
        );
        assert!(
            rows[0].has_field("world", "Agnos"),
            "release row must carry world: {rows:#?}"
        );
    }
}
