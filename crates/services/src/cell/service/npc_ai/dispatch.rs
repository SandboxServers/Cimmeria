//! AI tick entry + fast-retry sweep — the state-machine dispatchers
//! that route each NPC to its per-state handler.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::fight::{npc_ai_fight, npc_ai_idle_auto_aggro};
use super::follow::npc_ai_follow;
use super::investigate::npc_ai_investigate;
use super::leash::npc_ai_leash;
use super::lifecycle::{npc_ai_despawn, npc_ai_error, npc_ai_submit};
use super::patrol::npc_ai_patrol;
use super::wander::npc_ai_wander;

/// NPC AI tick — drives Fighting, Leashing, and Idle-with-aggression
/// NPCs. The `Idle` filter on `aggression > 0` is what makes the
/// `set_aggression` content action actually trigger combat — without it
/// the action would be a behavior bit nothing read. See
/// [`crate::cell::content::executor::world::set_aggression`].
pub(in crate::cell::service) async fn npc_ai_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    use cimmeria_entity::cell_entity::AiState;

    // Snapshot NPC IDs and their AI state so we don't hold a borrow on space_mgr
    // while calling handle_use_ability (which needs &mut SpaceManager).
    let npc_snapshot: Vec<(u32, AiState, i32, bool, bool)> = space_mgr
        .all_npc_entity_ids()
        .iter()
        .filter_map(|&eid| {
            space_mgr.get_entity(eid).map(|e| {
                (
                    eid,
                    e.ai_state,
                    e.aggression,
                    !e.patrol_path.is_empty(),
                    e.wander_radius > 0.0,
                )
            })
        })
        .filter(|(_, state, aggression, has_patrol, has_wander)| {
            // Admit any state that has a per-tick handler. Idle is
            // admitted when the NPC has a patrol path, a wander
            // radius, or positive aggression so the tick can promote
            // it into the matching behavior state.
            *state == AiState::Fighting
                || *state == AiState::Leashing
                || *state == AiState::Patrol
                || *state == AiState::Wander
                || *state == AiState::Investigating
                || *state == AiState::Follow
                || *state == AiState::Despawning
                || *state == AiState::Submit
                || *state == AiState::Error
                || (*state == AiState::Idle && (*aggression > 0 || *has_patrol || *has_wander))
        })
        .collect();

    use tracing::Instrument;

    for (npc_id, ai_state, _, has_patrol, has_wander) in npc_snapshot {
        // `.instrument()` (not `.entered()`) — the handler bodies await,
        // so a thread-local guard would silently fall off across runtime
        // thread switches.
        let space_id = space_mgr.get_entity(npc_id).map(|e| e.space_id.0);
        let ai_span = tracing::debug_span!(
            "npc_ai.decision",
            npc_id,
            ai_state = ?ai_state,
            space_id = space_id.unwrap_or(0),
            // Filled by each handler via Span::current().record(...).
            // The vocab is enumerated in docs/architecture/observability.md
            // §npc_ai.decision_outcome enum — adding a new outcome
            // requires updating that table so SigNoz queries stay
            // stable.
            decision_outcome = tracing::field::Empty,
        );
        async {
            match ai_state {
                AiState::Fighting => npc_ai_fight(npc_id, tx, space_mgr, engine).await,
                AiState::Leashing => npc_ai_leash(npc_id, tx, space_mgr).await,
                AiState::Patrol => npc_ai_patrol(npc_id, tx, space_mgr).await,
                AiState::Wander => npc_ai_wander(npc_id, tx, space_mgr).await,
                AiState::Investigating => npc_ai_investigate(npc_id, tx, space_mgr).await,
                AiState::Follow => npc_ai_follow(npc_id, tx, space_mgr).await,
                AiState::Despawning => npc_ai_despawn(npc_id, tx, space_mgr).await,
                AiState::Submit => npc_ai_submit(npc_id, tx, space_mgr).await,
                AiState::Error => npc_ai_error(npc_id, tx, space_mgr).await,
                AiState::Idle => {
                    // Priority order: aggression > patrol > wander.
                    // Aggro-driven idle has priority because an
                    // aggressive guard standing on a waypoint should
                    // still seed threat on a passing player rather
                    // than stride past them. Patrol beats wander
                    // because explicit waypoint authoring is more
                    // intentional than a wander radius.
                    let aggression = space_mgr
                        .get_entity(npc_id)
                        .map(|e| e.aggression)
                        .unwrap_or(0);
                    if aggression > 0 {
                        npc_ai_idle_auto_aggro(npc_id, tx, space_mgr).await;
                    } else if has_patrol {
                        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                            npc.ai_state = AiState::Patrol;
                        }
                        npc_ai_patrol(npc_id, tx, space_mgr).await;
                    } else if has_wander {
                        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                            npc.ai_state = AiState::Wander;
                        }
                        npc_ai_wander(npc_id, tx, space_mgr).await;
                    }
                }
                AiState::Dead | AiState::Spawning => {
                    // Excluded by the snapshot filter above. Listing
                    // them explicitly keeps the match exhaustive so a
                    // new `AiState` variant lands as a compile error
                    // here rather than a silent admit / no-op.
                }
            }
        }
        .instrument(ai_span)
        .await;
    }
}

/// Retry sweep — runs every AoI tick (100ms) from
/// `cell/service/message_loop.rs`. Iterates NPCs whose `ai_retry_at`
/// deadline has passed and runs `npc_ai_fight` on each, clearing the
/// retry slot afterward. Lets a launch-failure-driven re-attempt land
/// in 500-600ms instead of waiting for the 2-second natural-cadence
/// tick — see `AI_LAUNCH_FAILURE_RETRY_DELAY`.
///
/// The natural-cadence tick (`npc_ai_tick`, every 20th AoI tick)
/// continues to drive Idle-auto-aggro, Leashing, and the baseline
/// Fighting pass for NPCs without a pending retry — this sweep ONLY
/// services the retry path. Keeping the two functions separate avoids
/// changing the per-AoI-tick cost for healthy NPCs.
///
/// # Iteration cost
///
/// Iterates `space_mgr.pending_ai_retries` (a `HashSet<u32>` of NPCs
/// with a scheduled retry) rather than scanning every NPC in every
/// space. The set is maintained by `npc_ai_fight` on schedule and
/// cleared here on consume, so the per-AoI-tick cost is
/// `O(pending)` — typically 0 for a healthy server, bounded by the
/// number of NPCs that just lost a target mid-launch. A prior
/// implementation walked `all_npc_entity_ids()` every tick and was
/// `O(total NPCs)`; that's the cost this set avoids.
///
/// # Filter discipline
///
/// The set is a "candidates for fast-retry" pointer set, not the
/// source of truth — entries can become stale if an NPC is destroyed
/// or transitions out of `Fighting` while a retry is pending. The
/// double-check filter below combines `ai_retry_at.is_some_and(|t|
/// t <= now)` with `ai_state == Fighting` and handles stale cases by
/// skipping them and removing the entry, so the set self-heals.
pub(in crate::cell::service) async fn npc_ai_retry_sweep(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    use cimmeria_entity::cell_entity::AiState;

    if space_mgr.pending_ai_retries.is_empty() {
        return;
    }

    let now = std::time::Instant::now();
    // Snapshot the candidate set so the entity-mut + npc_ai_fight calls
    // below can borrow `space_mgr` mutably without aliasing the set.
    let candidates: Vec<u32> = space_mgr.pending_ai_retries.iter().copied().collect();

    let mut to_remove: Vec<u32> = Vec::new();
    let mut to_run: Vec<u32> = Vec::new();
    for npc_id in candidates {
        let Some(e) = space_mgr.get_entity(npc_id) else {
            // NPC was destroyed mid-flight — drop the stale set entry.
            to_remove.push(npc_id);
            continue;
        };
        let deadline_due = e.ai_retry_at.is_some_and(|t| t <= now);
        let fighting = e.ai_state == AiState::Fighting;
        if !fighting {
            // State-transitioned out of Fighting (Idle / Leashing /
            // Dead) — the natural-cadence tick handles those states
            // and the retry slot is meaningless. Drop the entry.
            to_remove.push(npc_id);
            continue;
        }
        if !deadline_due {
            // Set member but deadline still in the future — leave
            // alone, the next sweep tick will pick it up.
            continue;
        }
        to_run.push(npc_id);
    }

    for npc_id in to_remove {
        space_mgr.pending_ai_retries.remove(&npc_id);
    }

    for npc_id in to_run {
        // Clear the retry slot BEFORE running the fight pass, so a
        // failure inside the pass can set a fresh deadline without
        // racing this sweep's iteration. Same idempotence rationale
        // for the set — `npc_ai_fight`'s failure path will re-insert
        // if it schedules a new retry.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_retry_at = None;
        }
        space_mgr.pending_ai_retries.remove(&npc_id);
        npc_ai_fight(npc_id, tx, space_mgr, engine).await;
    }
}
