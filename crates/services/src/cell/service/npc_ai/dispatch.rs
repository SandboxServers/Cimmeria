//! AI tick entry + fast-retry sweep — the state-machine dispatchers
//! that route each NPC to its per-state handler.

use std::time::{Duration, Instant};

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

/// Defence-in-depth admit filter: an NPC at or below zero HEALTH never
/// gets an AI turn, whatever its `ai_state` says.
///
/// `ai_state = Dead` (stamped by `combat::mark_npc_dead` inside
/// `abilities::death::resolve_death`) is the primary mechanism and this
/// filter should be redundant. It is not free redundancy: a 0-HP NPC that
/// slipped past the death path kept its `Fighting` state and kept
/// attacking — the playtest symptom where a guard bled to zero by an
/// effect script went on hitting the player for 86 damage while sitting
/// at 0 HP. Gating on the health stat rather than on `ai_state` means the
/// AI cannot act on a target the combat layer already considers finished,
/// even if a future kill path forgets to stamp the state.
///
/// A 0-HP NPC *without* `BSF_DEAD` is an invariant violation, so it warns
/// (a proper corpse carries the bit and is silently skipped). See
/// `docs/architecture/negative-logging-convention.md`.
///
/// The warning is throttled per NPC. Nothing clears the condition — the NPC
/// stays in `all_npc_entity_ids()` at 0 HEALTH — so the natural tick would
/// otherwise repeat it for as long as the zone is up. One row per NPC per
/// [`ZERO_HEALTH_WARN_MIN_INTERVAL`], carrying `suppressed = N` for the
/// skipped ticks in between, keeps the fact and the rate without the volume.
pub(in crate::cell::service) fn npc_is_incapacitated(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    now: Instant,
) -> bool {
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return false;
    };
    let zeroed = e
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .is_some_and(|s| s.cur <= 0);
    if !zeroed {
        return false;
    }
    if crate::cell::combat::is_dead_state(e.state_field) {
        return true;
    }
    let Some(suppressed) =
        space_mgr
            .zero_health_npc_log
            .admit(npc_id, now, ZERO_HEALTH_WARN_MIN_INTERVAL)
    else {
        return true;
    };
    // Re-borrowed: `admit` above needed `&mut`.
    if let Some(e) = space_mgr.get_entity(npc_id) {
        tracing::warn!(
            target: "npc_ai.tick",
            npc_id,
            npc_name = e.npc_name.as_deref().unwrap_or(""),
            tag = e.tag.as_deref().unwrap_or(""),
            ai_state = ?e.ai_state,
            state_field = e.state_field,
            suppressed,
            "npc_ai: skipping NPC at 0 HEALTH with no BSF_DEAD — a kill path \
             zeroed health without running abilities::death::resolve_death"
        );
    }
    true
}

/// Minimum gap between two zero-health invariant warnings for one NPC. The
/// AI tick reaches a stuck NPC about every two seconds; a minute is frequent
/// enough to show the condition persisting and rare enough to leave on.
pub(in crate::cell::service) const ZERO_HEALTH_WARN_MIN_INTERVAL: Duration =
    Duration::from_secs(60);

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
    let now = Instant::now();
    let mut npc_ids = space_mgr.all_npc_entity_ids();
    npc_ids.retain(|&eid| !npc_is_incapacitated(space_mgr, eid, now));
    let npc_snapshot: Vec<(u32, AiState, i32, bool, bool)> = npc_ids
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
            super::take_last_outcome();
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
            log_ai_tick(space_mgr, npc_id, ai_state, super::take_last_outcome());
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
        if npc_is_incapacitated(space_mgr, npc_id, now) {
            // At or below zero HEALTH — the retry slot is meaningless and
            // the corpse must not get a fast-retry swing in. Drop it.
            to_remove.push(npc_id);
            continue;
        }
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

/// One row per ticked NPC per AI tick, emitted AFTER its handler ran, with no
/// silent paths: where the NPC is, where it is going, which way it faces (and
/// the byte clients are sent), and what it is fighting or following. An empty
/// `decision_outcome` means the handler returned without declaring one --
/// itself worth seeing.
fn log_ai_tick(
    space_mgr: &crate::cell::space_manager::SpaceManager,
    npc_id: u32,
    state_before: cimmeria_entity::cell_entity::AiState,
    outcome: &'static str,
) {
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let (target_id, threat) = e
        .threat_list
        .iter()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map_or((e.follow_target_id.unwrap_or(0), 0.0), |(id, v)| (*id, *v));
    let target = space_mgr.get_entity(target_id).map(|t| t.position);
    let dest = e.nav_path.back().copied();
    let next = e.nav_path.front().copied();
    tracing::debug!(
        target: "npc_ai.tick",
        npc_id,
        npc_name = e.npc_name.as_deref().unwrap_or(""),
        tag = e.tag.as_deref().unwrap_or(""),
        state_before = ?state_before,
        ai_state = ?e.ai_state,
        decision_outcome = outcome,
        x = e.position.x,
        y = e.position.y,
        z = e.position.z,
        yaw_rad = e.direction.y,
        yaw_byte = crate::mercury::aoi::pack_angle(e.direction.y),
        last_movement_type = ?e.last_movement_type,
        nav_path_len = e.nav_path.len(),
        next_wp = ?next.map(|p| [p.x, p.y, p.z]),
        dest = ?dest.map(|p| [p.x, p.y, p.z]),
        dist_to_dest = ?dest.map(|p| p.distance_to(&e.position)),
        target_id,
        threat,
        threat_count = e.threat_list.len(),
        target_pos = ?target.map(|p| [p.x, p.y, p.z]),
        dist_to_target = ?target.map(|p| p.distance_to(&e.position)),
        has_los = target.is_some().then(|| space_mgr.has_line_of_sight(npc_id, target_id)),
        follow_target_id = e.follow_target_id.unwrap_or(0),
        dist_to_spawn = ?e.spawn_position.map(|p| p.distance_to(&e.position)),
        move_speed = e.move_speed,
        navmesh_loaded = space_mgr.space_has_navmesh(npc_id),
        "NPC AI tick"
    );
}
