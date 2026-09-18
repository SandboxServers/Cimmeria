//! Cell-tick drain for content-engine actions deferred by
//! `content_actions.delay_ms > 0` (C08a).
//!
//! `execute_actions` queues a delayed action via
//! `SpaceManager::schedule_content_action` instead of running it inline;
//! `deferred_content_action_tick` is called once per cell tick (100ms,
//! same cadence as `cell::service::ticks::pending_holster`'s
//! `pending_attack_tick`/`pending_reload_tick` — the established
//! precedent in this codebase for "resume queued work later without
//! blocking the tick") and fires whatever has elapsed through the same
//! `execute_one` dispatch the immediate path uses.
//!
//! Storing the queue on `SpaceManager` rather than a spawned
//! `tokio::time::sleep` task was the deliberate call here: a spawned task
//! has no route to `&mut SpaceManager`, which is owned exclusively by the
//! single-threaded cell message loop, so it would need new inter-task
//! plumbing (a channel + a new message variant) just to re-enter the
//! executor. The tick-drained queue needs none of that, and reuses
//! `SpaceManager::destroy_entity`'s existing role as the single
//! disconnect/leave-space cleanup choke point — see
//! `space_manager::deferred_content_actions` for the scheduling API and
//! that cleanup.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::execute_one;

/// Fire every deferred content action whose `delay_ms` has elapsed.
///
/// Called once per cell tick from `cell::service::message_loop`. A queue
/// entry can only exist for an entity that's still in its space (an
/// entity leaving — cross-world teleport, disconnect, GM despawn, death —
/// always routes through `SpaceManager::destroy_entity` /
/// `disconnect_entity`, both of which scrub the entity's queue), so this
/// never needs to defensively check "does the entity still exist" before
/// calling `execute_one` — the per-action executor arms already no-op
/// safely on a missing entity regardless, the same as the immediate path.
pub(crate) async fn deferred_content_action_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    let now = std::time::Instant::now();
    let ready = space_mgr.take_ready_content_actions(now);
    if ready.is_empty() {
        return;
    }

    for (entity_id, pending) in ready {
        tracing::info!(
            entity_id,
            chain_id = pending.chain_id,
            action = ?pending.action,
            "Content: firing deferred action"
        );
        execute_one(
            pending.chain_id,
            pending.action,
            entity_id,
            pending.player_id,
            &pending.params,
            tx,
            space_mgr,
            engine,
        )
        .await;
    }
}
