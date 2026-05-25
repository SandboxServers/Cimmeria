//! NPC AI tick — fight (threat-target attacks + leashing), and leash recovery.
//!
//! Runs every 2 seconds (every 20th AoI tick).

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
    let npc_snapshot: Vec<(u32, AiState, i32)> = space_mgr
        .all_npc_entity_ids()
        .iter()
        .filter_map(|&eid| {
            space_mgr
                .get_entity(eid)
                .map(|e| (eid, e.ai_state, e.aggression))
        })
        .filter(|(_, state, aggression)| {
            *state == AiState::Fighting
                || *state == AiState::Leashing
                || (*state == AiState::Idle && *aggression > 0)
        })
        .collect();

    for (npc_id, ai_state, _) in npc_snapshot {
        match ai_state {
            AiState::Fighting => {
                npc_ai_fight(npc_id, tx, space_mgr).await;
            }
            AiState::Leashing => {
                npc_ai_leash(npc_id, tx, space_mgr).await;
            }
            AiState::Idle => {
                npc_ai_idle_auto_aggro(npc_id, space_mgr);
            }
            _ => {}
        }
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
fn npc_ai_idle_auto_aggro(npc_id: u32, space_mgr: &mut SpaceManager) {
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
        // Discard the optional new-state — the auto-aggro path doesn't
        // broadcast `onStateFieldUpdate` to the player here. The next
        // explicit hit (player fires back, NPC retaliates, etc.) will go
        // through the normal generate_threat → enter_player_combat path
        // which does the BSF_IN_COMBAT broadcast. Doing it here would
        // light up the combat HUD before the player has any reason to
        // know they've been seen — surfacing as a "ghost combat" UX bug.
        let _ = combat::generate_threat(space_mgr, player_id, npc_id, 1.0);
    }
}

/// NPC fighting behavior: attack top-threat target or leash if too far from spawn.
async fn npc_ai_fight(npc_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    use super::super::combat;
    use cimmeria_entity::cell_entity::AiState;

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
                    npc_id,
                    target = target_id,
                    distance = dist_to_spawn,
                    "NPC AI: target too far from spawn, leashing"
                );
            }
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
    // by issue #329: prior code used the flat constant and ignored
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
                        npc_id,
                        target = target_id,
                        in_range,
                        has_los,
                        "NPC AI: pathfinding toward target"
                    );
                }
            } else {
                tracing::debug!(
                    npc_id,
                    target = target_id,
                    in_range,
                    has_los,
                    "NPC AI: no path to target"
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
                npc_id,
                target = target_id,
                ability_id = chosen_ability,
                distance = dist_to_target,
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
                npc_id,
                target = target_id,
                "NPC AI: no usable ability (all cooling or needs-ammo), holding fire"
            );
            return;
        }
    };

    tracing::debug!(
        npc_id,
        target = target_id,
        ability_id = chosen_ability,
        distance = dist_to_target,
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
    }
}

/// Delay before re-running `npc_ai_fight` after a `handle_use_ability`
/// launch failure. Pinned at 500ms per spec evidence in issue #329
/// (`Atrea.addTimer(t + 0.5, doAiAction)`). The retry sweep tick is
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
/// tick — see `AI_LAUNCH_FAILURE_RETRY_DELAY` and issue #329.
///
/// The natural-cadence tick (`npc_ai_tick`, every 20th AoI tick)
/// continues to drive Idle-auto-aggro, Leashing, and the baseline
/// Fighting pass for NPCs without a pending retry — this sweep ONLY
/// services the retry path. Keeping the two functions separate avoids
/// changing the per-AoI-tick cost for healthy NPCs.
pub(super) async fn npc_ai_retry_sweep(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;

    let now = std::time::Instant::now();
    let due: Vec<u32> = space_mgr
        .all_npc_entity_ids()
        .iter()
        .filter_map(|&eid| {
            let e = space_mgr.get_entity(eid)?;
            let due = e.ai_retry_at.is_some_and(|t| t <= now);
            // Only run the retry pass on Fighting NPCs — Idle / Leashing
            // NPCs that somehow have `ai_retry_at` set are misuse; the
            // natural-cadence tick handles them on its own.
            (due && e.ai_state == AiState::Fighting).then_some(eid)
        })
        .collect();

    for npc_id in due {
        // Clear the retry slot BEFORE running the fight pass, so a
        // failure inside the pass can set a fresh deadline without
        // racing this sweep's iteration.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_retry_at = None;
        }
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

/// NPC leashing behavior: reset to Idle and restore health.
///
/// In a full implementation this would pathfind the NPC back to spawn.
/// For now we snap back instantly and restore health.
async fn npc_ai_leash(npc_id: u32, tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    use cimmeria_entity::cell_entity::AiState;

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
}
