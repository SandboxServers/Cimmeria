//! NPC AI tick — fight (threat-target attacks + leashing), and leash recovery.
//!
//! # Cadence
//!
//! Two passes share the AI surface, both driven from
//! [`crate::cell::service::message_loop`]:
//!
//! - **[`npc_ai_tick`]** — natural cadence, every 20th AoI tick (~2s
//!   at the 100ms AoI rate). Drives Idle-auto-aggro, Leashing, and
//!   the baseline Fighting pass for every NPC.
//! - **[`npc_ai_retry_sweep`]** — fast retry, every AoI tick (~100ms).
//!   Picks up Fighting NPCs whose `ai_retry_at` deadline has passed
//!   after a `handle_use_ability` launch failure, so the AI can
//!   re-attempt within 500–600ms instead of waiting the full 2s
//!   natural cadence. Iterates `space_mgr.pending_ai_retries`
//!   (`O(pending)`), not the full NPC list.
//!
//! Healthy NPCs never appear in `pending_ai_retries`; the retry
//! sweep is effectively free in that case.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// NPC AI tick — drives Fighting, Leashing, and Idle-with-aggression
/// NPCs. The `Idle` filter on `aggression > 0` is what makes the
/// `set_aggression` content action actually trigger combat — without it
/// the action would be a behavior bit nothing read. See
/// [`super::super::content::executor::world::set_aggression`].
pub(super) async fn npc_ai_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
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
        );
        async {
            match ai_state {
                AiState::Fighting => npc_ai_fight(npc_id, tx, space_mgr).await,
                AiState::Leashing => npc_ai_leash(npc_id, tx, space_mgr).await,
                AiState::Patrol => npc_ai_patrol(npc_id, tx, space_mgr).await,
                AiState::Wander => npc_ai_wander(npc_id, tx, space_mgr).await,
                AiState::Investigating => npc_ai_investigate(npc_id, tx, space_mgr).await,
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
                _ => {
                    // Other states (Dead, Spawning, Investigating,
                    // Follow, Despawning, Submit, Error) either don't have
                    // a per-tick handler today or shouldn't have been
                    // admitted by the snapshot filter. The remaining
                    // handlers land in later commits of this PR.
                }
            }
        }
        .instrument(ai_span)
        .await;
    }
}

/// Auto-aggro tick for Idle NPCs with `aggression > 0`.
///
/// Scans witnesses for opposing-faction players, seeds a small threat on
/// the closest. The next AI tick transitions the NPC to Fighting.
///
/// Seed magnitude (`1.0`) is intentionally tiny so an explicit
/// `generate_threat` from a content chain (e.g., chain 1032's `1000`)
/// dominates and focuses the NPC on the triggering player rather than
/// whichever player happens to be closest. Caller (`npc_ai_tick`)
/// guarantees `aggression > 0`.
async fn npc_ai_idle_auto_aggro(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use super::super::combat;

    let (npc_pos, npc_faction) = match space_mgr.get_entity(npc_id) {
        Some(e) => (e.position, e.faction),
        None => return,
    };

    // Witnesses-of-NPC = players currently rendering this NPC, i.e. players
    // in the NPC's AoI. That's exactly the candidate set the Python `Atrea`
    // engine scans — restricted to players because NPCs don't aggro on
    // other NPCs from idle.
    let witnesses = space_mgr.get_witnesses_of(npc_id);
    let target = witnesses
        .into_iter()
        .filter_map(|pid| {
            let p = space_mgr.get_entity(pid)?;
            if !p.is_player || p.faction == npc_faction {
                return None;
            }
            // Skip dead players (BSF_DEAD in state_field — bit 0).
            if combat::is_dead_state(p.state_field) {
                return None;
            }
            let dist = npc_pos.distance_to(&p.position);
            Some((pid, dist))
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(pid, _)| pid);

    if let Some(player_id) = target {
        tracing::info!(
            npc_id,
            player_id,
            "NPC AI: aggression-driven auto-aggro on opposing-faction player"
        );
        // Invariant: when `enter_player_combat` flips `weapon_holstered`
        // to false on first-add, the client's cached `ComponentList`
        // must be refreshed — otherwise the fire path passes the
        // `needs_unholster_queue` gate (server thinks drawn) while
        // the client still renders the holstered mesh. The
        // `onStateFieldUpdate` half is intentionally suppressed here
        // (auto-aggro can fire before the player has any visible
        // reason to know — lighting up `BSF_IN_COMBAT` is the "ghost
        // combat HUD" carve-out); the damage path broadcasts it on
        // the next explicit hit.
        if combat::generate_threat(space_mgr, player_id, npc_id, 1.0).is_some() {
            super::super::abilities::request_appearance_refresh(player_id, tx, space_mgr).await;
        }
    }
}

/// NPC fighting behavior: attack top-threat target or leash if too far from spawn.
async fn npc_ai_fight(npc_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    use super::super::combat;
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    // Movement-type broadcast on Fighting entry. Dedup'd against the
    // cached `last_movement_type` — subsequent Fighting ticks are
    // no-ops on the wire. See `broadcast_movement_type` doc for
    // rationale (animation hint, not gameplay-side state).
    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::CombatAdvance),
        tx,
        space_mgr,
    )
    .await;

    // Read NPC state (immutable borrow)
    let (top_target, spawn_pos, npc_pos, is_stationary) = {
        let npc = match space_mgr.get_entity(npc_id) {
            Some(e) => e,
            None => return,
        };

        // Find highest-threat target
        let top = npc
            .threat_list
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&eid, _)| eid);

        (top, npc.spawn_position, npc.position, npc.is_stationary)
    };

    let target_id = match top_target {
        Some(tid) => tid,
        None => {
            // No threat targets left — reset to idle
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.ai_state = AiState::Idle;
                npc.threat_list.clear();
                tracing::debug!(npc_id, "NPC AI: no threat targets, resetting to Idle");
            }
            // Clear cached movement-type so the next Fighting entry
            // re-broadcasts. None means "no wire emission, just drop
            // the dedup cache" per `broadcast_movement_type` doc.
            super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
            return;
        }
    };

    // Check if target still exists and is alive
    let target_pos = match space_mgr.get_entity(target_id) {
        Some(t) => {
            // Don't attack dead targets
            let is_dead = t
                .stats
                .get(cimmeria_entity::stats::HEALTH)
                .is_none_or(|s| s.cur <= 0);
            if is_dead {
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.threat_list.remove(&target_id);
                    tracing::debug!(
                        npc_id,
                        target = target_id,
                        "NPC AI: target is dead, removing from threat"
                    );
                }
                return;
            }
            t.position
        }
        None => {
            // Target gone (disconnected), remove from threat and re-evaluate
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.threat_list.remove(&target_id);
            }
            return;
        }
    };

    // Leash check: if target is too far from NPC's spawn point, disengage
    if let Some(spawn) = spawn_pos {
        let dist_to_spawn = spawn.distance_to(&target_pos);
        if dist_to_spawn > combat::LEASH_DISTANCE {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.ai_state = AiState::Leashing;
                npc.threat_list.clear();
                tracing::info!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "leashed",
                    npc_id,
                    target_id,
                    dist_to_spawn,
                    "NPC AI: target too far from spawn, leashing"
                );
            }
            // Broadcast Leash movement-type now rather than wait for
            // the next AI tick (which would land ~2s later). The leash
            // handler itself snaps the position instantly, so even
            // though there's no actual leash-walk yet, the wire
            // signal lets the client play any leash-specific VFX it
            // wants for one frame before the corpse snaps home.
            super::super::abilities::broadcast_movement_type(
                npc_id,
                Some(MobMovementType::Leash),
                tx,
                space_mgr,
            )
            .await;
            return;
        }
    }

    // Pick the ability up front so the range check can gate on the
    // ability's own `min_range` / `max_range` instead of a flat
    // server-wide constant. `choose_npc_ability` returns:
    //   - `Some(NPC_DEFAULT_ABILITY)` when the NPC has no known abilities
    //     (misconfigured template — explicit fallback per the selector's
    //     "don't wedge silently" rule).
    //   - `Some(id)` for the first non-cooling known ability.
    //   - `None` when every known ability is on cooldown.
    //
    // In the `None` case we keep the range/LOS logic running against the
    // server-wide fallback so the NPC still walks toward / tracks the
    // target while waiting for an off-cooldown ability — same effective
    // behavior as the pre-issue-329 flat-30.0 code path.
    let chosen_ability = choose_npc_ability(npc_id, space_mgr);
    let (max_range, min_range) =
        ability_ranges(chosen_ability, space_mgr, combat::NPC_ATTACK_RANGE);

    // Range check: don't attack until target is within the chosen
    // ability's `max_range` (or `NPC_ATTACK_RANGE` if the def is missing
    // or carries the `0` sentinel meaning "use server default"). Pinned
    // Previously: prior code used the flat constant and ignored
    // per-ability `max_range`, which produced "NPC walks into firing
    // distance but stands there" for any ability with `max_range < 30`
    // (e.g., a grenade at `max_range = 15`).
    let dist_to_target = npc_pos.distance_to(&target_pos);
    let in_range = dist_to_target <= max_range;
    let has_los = space_mgr.has_line_of_sight(npc_id, target_id);

    // Out of range OR occluded — keep pathfinding so the NPC can reposition
    // to regain line of sight. Treating "in range but blocked" as a stop
    // condition would freeze the NPC behind walls/corners; making it a repath
    // condition lets the AI walk around the obstruction.
    //
    // Stationary NPCs (turrets, fixed defenders) skip pathfinding entirely:
    // they hold position and only fire when the target enters range + LOS.
    // The leash check above still resets them if the target wanders past
    // LEASH_DISTANCE.
    if !in_range || !has_los {
        if is_stationary {
            // Stationary NPC out of range OR with no LoS — silently
            // skipped pre-fix. Emit a structured info log so this
            // branch is observable in SigNoz without code spelunking.
            // The Ambernol drone (template 4, ability_set 2 / Energy
            // Shock) sat in this branch for 54 s of aggro on every
            // tick because the navmesh raycast fail-closed on
            // off-mesh flyer positions — and no log line surfaced
            // it. Same pattern that the existing `no_path` log
            // catches for non-stationary NPCs.
            tracing::info!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "stationary_holds",
                npc_id,
                target_id,
                in_range,
                has_los,
                dist_to_target,
                max_range,
                "NPC AI: stationary mob holding fire (out of range or no LoS) — \
                 verify position is on the navmesh and target is reachable"
            );
            return;
        }
        let needs_repath = {
            let npc = space_mgr.get_entity(npc_id);
            match npc {
                Some(e) if !e.nav_path.is_empty() => {
                    // Check if target moved far from the last waypoint
                    let last_wp = match e.nav_path.back() {
                        Some(wp) => *wp,
                        None => return,
                    };
                    last_wp.distance_to(&target_pos) > 5.0
                }
                _ => true, // No path — need one
            }
        };

        if needs_repath {
            if let Some(path) = space_mgr.find_path(npc_id, &npc_pos, &target_pos) {
                if path.len() > 1 {
                    let waypoints: std::collections::VecDeque<_> =
                        path.into_iter().skip(1).collect();
                    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                        npc.nav_path = waypoints;
                    }
                    tracing::debug!(
                        target: "npc_ai",
                        event = "decision",
                        decision_outcome = "chase",
                        npc_id,
                        target_id,
                        in_range,
                        has_los,
                        dist_to_target,
                        "NPC AI: pathfinding toward target"
                    );
                }
            } else {
                // No-path is the diagnostic signal for "navmesh missing in
                // this zone" — see issue #407. The parent `npc_ai.decision`
                // span already carries `space_id`, so SigNoz can group
                // `groupBy=decision_outcome` across the npc_ai target and
                // pivot per zone via the span's space_id attribute.
                tracing::info!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "no_path",
                    npc_id,
                    target_id,
                    in_range,
                    has_los,
                    dist_to_target,
                    "NPC AI: no path to target (zone may need navmesh)"
                );
            }
        }
        return;
    }

    // Min-range backup: target is inside the chosen ability's
    // `min_range`. The ability would refuse to fire (e.g., a sniper at
    // `min_range = 5`, target at distance 3). Step the NPC back along
    // the target→NPC vector to `min_range + 1.0` so the next tick lands
    // it just outside the dead zone and can fire.
    //
    // Stationary NPCs skip the backup — they're pinned in place by
    // design. A sniper turret with a min-range gap just won't fire on
    // a close target, same as today.
    if min_range > 0.0 && dist_to_target < min_range && !is_stationary {
        if let Some(backup) = compute_backup_waypoint(npc_pos, target_pos, min_range) {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.nav_path.clear();
                npc.nav_path.push_back(backup);
            }
            tracing::debug!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "min_range_backup",
                npc_id,
                target_id,
                ability_id = chosen_ability,
                dist_to_target,
                min_range,
                backup_x = backup.x,
                backup_z = backup.z,
                "NPC AI: target inside min_range — stepping back to fire"
            );
        }
        return;
    }

    // In range, LOS confirmed, and not too close — stop moving and attack.
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
    }

    // `chosen_ability` may still be `None` here when every known ability
    // is on cooldown — hold fire and let the next tick re-evaluate.
    let chosen_ability = match chosen_ability {
        Some(id) => id,
        None => {
            tracing::debug!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "no_ability",
                npc_id,
                target_id,
                dist_to_target,
                "NPC AI: no usable ability (all cooling or needs-ammo), holding fire"
            );
            return;
        }
    };

    tracing::debug!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "attack_in_place",
        npc_id,
        target_id,
        ability_id = chosen_ability,
        dist_to_target,
        max_range,
        min_range,
        "NPC AI: attacking top threat target"
    );
    let fired = super::super::abilities::handle_use_ability(
        npc_id,
        chosen_ability,
        target_id as i32,
        tx,
        space_mgr,
    )
    .await;
    if !fired {
        // handle_use_ability returns false when the
        // pre-consume guard rejected the call (entity missing/dead, no
        // ability, on cooldown, reload in flight, no ammo, or
        // out-of-range). For NPC AI ticks this is normally a cooldown
        // race against the pick logic; warn! so player-visible "mob
        // standing still" can be diagnosed without attaching a profiler.
        tracing::warn!(
            npc_id,
            target = target_id,
            ability_id = chosen_ability,
            distance = dist_to_target,
            reason = "handle_use_ability_returned_false",
            "NPC AI: attack tick produced no ability fire -- mob may appear stuck"
        );

        // Schedule a 500ms retry so the NPC doesn't sit visibly idle
        // until the natural 2-second AI tick — mirrors the
        // `Atrea.addTimer(t + 0.5, doAiAction)` pattern in
        // `python/cell/SGWMob.py`. The retry sweep
        // (`npc_ai_retry_sweep`) runs every AoI tick (100ms) and
        // consumes any `ai_retry_at <= now`, so the worst-case
        // observable retry latency is one AoI tick (~100ms) above the
        // 500ms deadline.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_retry_at = Some(std::time::Instant::now() + AI_LAUNCH_FAILURE_RETRY_DELAY);
        }
        // Mirror into the SpaceManager-level pending set so the sweep
        // can iterate `O(pending)` instead of `O(total NPCs)`. Borrow
        // sequencing matters: the entity-mut block above ends before
        // we re-borrow `space_mgr` for the set.
        space_mgr.pending_ai_retries.insert(npc_id);
    }
}

/// Delay before re-running `npc_ai_fight` after a `handle_use_ability`
/// launch failure. Pinned at 500ms per the Python fork's
/// `Atrea.addTimer(Atrea.getGameTime() + 0.5, lambda: self.doAiAction())`
/// call at [`deprecated/python/cell/SGWMob.py:287`]. The fork is the
/// closest behavioral reference we have for the original Stargate
/// Worlds AI cadence; treat the 0.5s as canon until a Ghidra trace of
/// the C++ AI tick says otherwise. The retry sweep tick is
/// 100ms granular, so the actual latency lands in `[500, 600)` ms.
const AI_LAUNCH_FAILURE_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// Resolve `(max_range, min_range)` for a chosen ability, falling back
/// to the server-default `NPC_ATTACK_RANGE` when the def is missing or
/// the field carries the `0` sentinel meaning "use server default."
///
/// `min_range` is `0.0` when the def carries `0` (no minimum). Distinct
/// from `max_range` which never zeroes legitimately — `0` always means
/// "default to `npc_attack_range`."
///
/// `chosen_ability == None` → all-cooling case; we still need a
/// max_range for the "should we walk toward the target?" gate, so the
/// fallback applies the same way as a missing def.
///
/// Returned as `(max, min)` because the call site reads `max` first in
/// the in-range check.
fn ability_ranges(
    chosen_ability: Option<i32>,
    space_mgr: &SpaceManager,
    npc_attack_range: f32,
) -> (f32, f32) {
    let def = chosen_ability.and_then(|id| space_mgr.ability_defs.get(&id));
    let max_range = def.map_or(npc_attack_range, |d| {
        if d.max_range > 0 {
            d.max_range as f32
        } else {
            npc_attack_range
        }
    });
    let min_range = def.map_or(0.0, |d| {
        if d.min_range > 0 {
            d.min_range as f32
        } else {
            0.0
        }
    });
    (max_range, min_range)
}

/// Step back along the target→NPC vector to a point at distance
/// `min_range + 1.0` from the target. Returns `None` if the NPC and
/// target are co-located (degenerate vector — can't normalize).
///
/// The +1.0 margin keeps the next tick's range check from oscillating
/// at exactly `min_range`; without it floating-point jitter would push
/// the NPC back inside the dead zone every other tick.
///
/// # Vertical-axis caveat
///
/// The returned waypoint preserves the NPC's Y-axis offset from the
/// target — if the NPC is uphill of the target, the backup point is
/// also uphill. This can yield a Y that the navmesh would reject (in
/// mid-air over a ledge, or under the floor). The waypoint is fed
/// into the same path-follower as `find_path` output, which clamps
/// invalid Y via the navmesh on consume. Callers that bypass that
/// path-follower must clamp themselves.
fn compute_backup_waypoint(
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    let dx = npc_pos.x - target_pos.x;
    let dy = npc_pos.y - target_pos.y;
    let dz = npc_pos.z - target_pos.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist < f32::EPSILON {
        return None;
    }
    let scale = (min_range + 1.0) / dist;
    Some(cimmeria_common::Vector3::new(
        target_pos.x + dx * scale,
        target_pos.y + dy * scale,
        target_pos.z + dz * scale,
    ))
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
pub(super) async fn npc_ai_retry_sweep(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
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
        npc_ai_fight(npc_id, tx, space_mgr).await;
    }
}

/// Pick an off-cooldown ability for the NPC's fight tick. `None` → all
/// cooling → caller holds fire. Empty bucket falls back to
/// `NPC_DEFAULT_ABILITY` so a misconfigured template doesn't wedge silently.
///
/// Why no ammo gate: NPCs have infinite ammo (the `required_ammo > 0` check
/// at the dispatch site is player-only). Gating here would permanently
/// disable abilities like Pistol Shot 592 (`required_ammo = 1`) that every
/// stock NPC carries.
///
/// Stable sort over `known_ability_ids` keeps selection deterministic
/// tick-to-tick; a future "prefer higher threat_level_id" refinement
/// changes the ordering without touching the partition.
pub(super) fn choose_npc_ability(npc_id: u32, space_mgr: &SpaceManager) -> Option<i32> {
    use super::super::combat;

    let npc = space_mgr.get_entity(npc_id)?;
    if npc.abilities.known_count() == 0 {
        return Some(combat::NPC_DEFAULT_ABILITY);
    }

    let mut ability_ids = npc.abilities.known_ability_ids();
    ability_ids.sort_unstable();

    ability_ids
        .into_iter()
        .find(|&id| !npc.abilities.is_on_cooldown(id))
}

/// NPC patrol behavior: walk a waypoint loop with dwell pauses.
///
/// State machine within Patrol:
/// 1. **Empty `nav_path` + no dwell deadline** → push the next
///    waypoint into `nav_path`, broadcast `CombatAdvance` is
///    intentionally NOT used (Patrol has its own
///    `MobMovementType::Patrol` byte). The path-follower
///    (`npc_movement_tick`) walks the NPC toward the waypoint at
///    `move_speed`.
/// 2. **Empty `nav_path` + dwell deadline in the future** → still
///    paused at the waypoint, no work to do.
/// 3. **Empty `nav_path` + dwell deadline elapsed** → advance
///    `patrol_next_index` (modulo path length) and clear the
///    deadline so the next pass falls into branch 1.
/// 4. **Non-empty `nav_path`** → movement is in flight, no work
///    to do here.
///
/// The dwell deadline is stamped here when we clear `nav_path` —
/// movement-tick consumes nav_path until it's empty, at which
/// point we know the NPC has arrived at the waypoint.
///
/// Threat preemption is handled outside: when `generate_threat`
/// flips the state from Patrol → Fighting, the next AI tick
/// routes through `npc_ai_fight`. On Fighting → Leashing →
/// Idle, the tick's Idle branch transitions back to Patrol and
/// the saved `patrol_next_index` resumes the route from where
/// it left off (no progress lost).
async fn npc_ai_patrol(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::MobMovementType;

    // Broadcast Patrol movement-type on entry. Dedup'd against
    // `last_movement_type` — subsequent Patrol ticks are no-ops on
    // the wire (the cache stays Some(Patrol) until a state
    // transition clears it).
    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Patrol),
        tx,
        space_mgr,
    )
    .await;

    let now = std::time::Instant::now();

    // Read patrol state without holding a borrow across the
    // pathfind / nav_path write below.
    let (path_empty_now, nav_empty, dwell, next_index, delay_secs, next_waypoint) = {
        let npc = match space_mgr.get_entity(npc_id) {
            Some(e) => e,
            None => return,
        };
        if npc.patrol_path.is_empty() {
            // Defensive: snapshot filter should have screened this
            // out, but a content action could have wiped the path
            // mid-tick. Drop back to Idle so the next AI tick can
            // re-route the NPC.
            (true, true, None, 0, 0.0, None)
        } else {
            let next_idx = npc.patrol_next_index % npc.patrol_path.len();
            (
                false,
                npc.nav_path.is_empty(),
                npc.patrol_dwell_until,
                next_idx,
                npc.patrol_point_delay_secs,
                Some(npc.patrol_path[next_idx]),
            )
        }
    };

    if path_empty_now {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    if !nav_empty {
        // Movement in flight — npc_movement_tick is walking the NPC
        // toward the current waypoint. Nothing to do this tick.
        return;
    }

    // Branch 2 or 3: nav_path is empty so we've arrived at a
    // waypoint (or never started). Decide between dwelling and
    // advancing.
    if let Some(deadline) = dwell {
        if now < deadline {
            // Still dwelling.
            return;
        }
        // Dwell elapsed → advance to next waypoint AND clear the
        // deadline. The next tick (branch 1) pushes the new
        // waypoint into nav_path.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            let len = npc.patrol_path.len();
            npc.patrol_next_index = (npc.patrol_next_index + 1) % len;
            npc.patrol_dwell_until = None;
        }
        // Fall through to push the new waypoint in the same tick
        // — saves one tick of latency at every waypoint.
    }

    // Branch 1: push the current waypoint into nav_path. Pathfind
    // so the NPC respects the navmesh rather than walking in a
    // straight line through walls. Falls back to a direct push if
    // pathfinding fails (no navmesh in the space, target off-mesh,
    // etc.) — better to walk through a wall than to wedge.
    let Some(waypoint) = next_waypoint else {
        return;
    };
    let npc_pos = match space_mgr.get_entity(npc_id) {
        Some(e) => e.position,
        None => return,
    };
    let path = space_mgr
        .find_path(npc_id, &npc_pos, &waypoint)
        .unwrap_or_default();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
        if path.len() > 1 {
            // Skip the first entry (start position). Detour returns a
            // straight-path that includes both endpoints.
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            // Pathfind failed or returned a single point — direct push.
            npc.nav_path.push_back(waypoint);
        }
        // Stamp the dwell deadline NOW (not on arrival) so it's
        // ready when the path-follower drains nav_path. The
        // movement tick pops waypoints one-by-one; when the last
        // pop yields an empty nav_path, branch 2/3 of the next
        // patrol tick consults `patrol_dwell_until` to decide
        // whether to advance. Setting the deadline relative to
        // "now + delay" assumes the movement tick takes well
        // under `delay` seconds to consume nav_path; for typical
        // 5–15 m hops at 0.6 unit/100ms and a 2-second delay
        // that holds.
        let secs = delay_secs.max(0.5); // floor: half a second
        npc.patrol_dwell_until = Some(now + std::time::Duration::from_secs_f32(secs));
    }
    tracing::debug!(
        target: "npc_ai",
        event = "patrol_waypoint_set",
        npc_id,
        next_index,
        wp_x = waypoint.x,
        wp_y = waypoint.y,
        wp_z = waypoint.z,
        "NPC AI: patrol → next waypoint queued"
    );
}

/// NPC wander behavior: pick a random point within `wander_radius` of
/// `spawn_position`, walk there via the navmesh, pause for a random
/// dwell drawn from `[wander_min_dwell_secs, wander_max_dwell_secs]`,
/// repeat.
///
/// State machine within Wander:
/// - **nav_path empty + no/elapsed dwell** → sample a fresh waypoint,
///   pathfind, queue into `nav_path`, stamp `wander_next_at`. RNG is
///   seeded from `(npc_id, current_dwell_deadline_nanos)` so the
///   sample is reproducible per-tick for tests but varies across
///   real-world ticks.
/// - **nav_path empty + future dwell deadline** → no-op (pausing).
/// - **nav_path non-empty** → movement-tick is walking, no work.
///
/// Off-mesh rejection: when `space_mgr.is_position_valid` fails for
/// the sampled point, the handler falls back to `spawn_position` so
/// the NPC heads home rather than walking through a wall. This is
/// deliberately simple — a future "re-sample N times before giving
/// up" would be a small refinement.
async fn npc_ai_wander(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::MobMovementType;
    use rand::{RngExt, SeedableRng};

    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Wander),
        tx,
        space_mgr,
    )
    .await;

    let now = std::time::Instant::now();

    // Read wander config + current position via an immutable borrow.
    let (spawn_pos, npc_pos, radius, min_dwell, max_dwell, nav_empty, wander_next) =
        match space_mgr.get_entity(npc_id) {
            Some(e) => (
                e.spawn_position,
                e.position,
                e.wander_radius,
                e.wander_min_dwell_secs,
                e.wander_max_dwell_secs,
                e.nav_path.is_empty(),
                e.wander_next_at,
            ),
            None => return,
        };

    // Defensive: a content action could have zeroed wander_radius
    // mid-tick. Drop back to Idle so the next tick re-routes.
    if radius <= 0.0 {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    if !nav_empty {
        // Movement tick is walking — nothing to do.
        return;
    }

    if let Some(deadline) = wander_next {
        if now < deadline {
            return; // Still pausing.
        }
    }

    let Some(spawn) = spawn_pos else {
        // No spawn anchor — wander can't pick a destination. Drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        return;
    };

    // Seed: (npc_id, current dwell-deadline nanos since UNIX epoch
    // would require std::time::SystemTime; Instant doesn't expose
    // that). Use elapsed time since the manager's `now` for variance
    // and npc_id for per-entity divergence. Tests can pre-set
    // `wander_next_at` to a known deadline to make the seed
    // deterministic; the runtime is non-deterministic which is the
    // desired property for "different NPCs wander to different
    // places".
    let seed = u64::from(npc_id) ^ (now.elapsed().as_nanos() as u64);
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let angle = rng.random_range(0.0..(std::f32::consts::TAU));
    let distance = rng.random_range(0.0..radius);
    let candidate = cimmeria_common::Vector3::new(
        spawn.x + angle.cos() * distance,
        spawn.y,
        spawn.z + angle.sin() * distance,
    );

    // Off-mesh rejection: fall back to spawn_position. A single
    // re-sample would be a nicer behavior but adds complexity for
    // marginal benefit at this stage.
    let target = if space_mgr.is_position_valid(npc_id, &candidate) {
        candidate
    } else {
        spawn
    };

    let path = space_mgr
        .find_path(npc_id, &npc_pos, &target)
        .unwrap_or_default();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(target);
        }
        // Sample the post-arrival dwell now so it's set when the
        // movement tick drains nav_path. Floor at 0.5s to guard
        // against a degenerate config.
        let dwell_secs = rng
            .random_range(min_dwell..=max_dwell.max(min_dwell))
            .max(0.5);
        npc.wander_next_at = Some(now + std::time::Duration::from_secs_f32(dwell_secs));
    }
    tracing::debug!(
        target: "npc_ai",
        event = "wander_waypoint_set",
        npc_id,
        target_x = target.x,
        target_z = target.z,
        radius,
        "NPC AI: wander → fresh waypoint queued"
    );
}

/// Dwell at the POI after pathfinding reaches it, in seconds. Hardcoded
/// because no template field carries an investigate-specific dwell yet;
/// future work can lift this to `entity_templates.investigate_dwell_secs`
/// if encounters need varying durations.
const INVESTIGATE_DWELL_SECS: f32 = 5.0;

/// NPC investigate behavior: walk to a content-set POI, dwell, return
/// to Idle.
///
/// State machine within Investigating:
/// - **No POI** → drop back to Idle (defensive — a content action
///   could have cleared the POI mid-tick).
/// - **POI + nav_path non-empty** → walking, no-op.
/// - **POI + nav_path empty + no dwell** → first entry; pathfind to
///   POI and queue.
/// - **POI + nav_path empty + future dwell** → at the POI, pausing.
/// - **POI + nav_path empty + elapsed dwell** → done, clear POI +
///   investigate_until, drop to Idle.
async fn npc_ai_investigate(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    // Use CombatAdvance as the closest movement-type — no dedicated
    // "investigating" byte exists in EMobMovementType, and the
    // animation it implies (alert advance) is the right hint.
    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::CombatAdvance),
        tx,
        space_mgr,
    )
    .await;

    let (poi, npc_pos, nav_empty, dwell) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.poi,
            e.position,
            e.nav_path.is_empty(),
            e.investigate_until,
        ),
        None => return,
    };

    let Some(poi_pos) = poi else {
        // Defensive: drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = AiState::Idle;
            npc.investigate_until = None;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    if !nav_empty {
        return; // Movement in flight.
    }

    let now = std::time::Instant::now();

    match dwell {
        Some(deadline) if now < deadline => {
            // At POI, dwelling.
        }
        Some(_) => {
            // Dwell elapsed → done, return to Idle.
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.ai_state = AiState::Idle;
                npc.poi = None;
                npc.investigate_until = None;
            }
            super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        }
        None => {
            // First entry — pathfind to POI, queue, stamp dwell.
            let path = space_mgr
                .find_path(npc_id, &npc_pos, &poi_pos)
                .unwrap_or_default();
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.nav_path.clear();
                if path.len() > 1 {
                    for wp in path.into_iter().skip(1) {
                        npc.nav_path.push_back(wp);
                    }
                } else {
                    npc.nav_path.push_back(poi_pos);
                }
                // Stamp dwell deadline now (relative to POI arrival
                // — same approximation patrol uses; the movement tick
                // takes well under INVESTIGATE_DWELL_SECS to consume
                // nav_path for typical POI distances).
                npc.investigate_until =
                    Some(now + std::time::Duration::from_secs_f32(INVESTIGATE_DWELL_SECS));
            }
            tracing::debug!(
                target: "npc_ai",
                event = "investigate_routed",
                npc_id,
                poi_x = poi_pos.x,
                poi_y = poi_pos.y,
                poi_z = poi_pos.z,
                "NPC AI: investigate → pathfinding to POI"
            );
        }
    }
}

/// NPC leashing behavior: reset to Idle and restore health.
///
/// In a full implementation this would pathfind the NPC back to spawn.
/// For now we snap back instantly and restore health.
async fn npc_ai_leash(npc_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    // The Fighting → Leashing transition site in `npc_ai_fight`
    // already broadcasts Leash, so this is a no-op in the normal
    // path — but for completeness (and for the future when leash
    // becomes a multi-tick walk-back rather than a snap) call it
    // here too. Dedup'd by `last_movement_type`.
    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Leash),
        tx,
        space_mgr,
    )
    .await;

    let (stat_update, state_field) = {
        let npc = match space_mgr.get_entity_mut(npc_id) {
            Some(e) => e,
            None => return,
        };

        // Snap back to spawn position
        if let Some(spawn_pos) = npc.spawn_position {
            npc.position = spawn_pos;
        }

        // Restore health to max
        if let Some(health) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            health.set_current(health.max);
        }

        npc.ai_state = AiState::Idle;
        npc.threat_list.clear();
        npc.abilities.clear_all_cooldowns();

        // No state-flag unsetting here: leash only fires when the NPC is
        // alive (the AI state machine routes dead NPCs to AiState::Dead, not
        // Leashing). BSF_DEAD/BSF_MOVEMENT_LOCK were never set in the first
        // place on a leashing NPC, so unsetting them would be defensive
        // paranoia against an unreachable code path.

        tracing::info!(
            npc_id,
            "NPC AI: leash complete, reset to Idle with full health"
        );

        // Collect data before dropping the mutable borrow
        let stat_update = npc.stats.serialize_dirty();
        npc.stats.clear_dirty();
        let state_field = npc.state_field;
        (stat_update, state_field)
    };

    super::super::abilities::send_entity_method(npc_id, 20, stat_update, tx, space_mgr).await;

    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    super::super::abilities::send_entity_method(npc_id, 19, state_args, tx, space_mgr).await;

    // Leash complete — clear the cached movement-type so the next
    // Fighting transition re-broadcasts CombatAdvance. None emits no
    // wire byte (client keeps its idle pose); only the dedup cache
    // resets. See `broadcast_movement_type` doc.
    super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
}

/// Test-only re-export of the private `compute_backup_waypoint` so
/// the sibling `tests/npc_ai.rs` module can exercise its degenerate
/// (co-located NPC + target) branch without making the helper `pub`.
///
/// The helper stays private to enforce the convention that only
/// `npc_ai_fight` calls it (the `+1.0` margin assumption is tied to
/// that caller); production callers must go through the fight pass.
#[cfg(test)]
pub(super) fn compute_backup_waypoint_for_test(
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    compute_backup_waypoint(npc_pos, target_pos, min_range)
}
