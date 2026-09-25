//! Fighting state: attack the top-threat target (with cover routing,
//! range/LOS gating, and min-range backup) or transition to Leashing.
//! The Idle auto-aggro seed that promotes an aggressive idle NPC into
//! combat lives in [`super::idle_aggro`].

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::ability_select::{
    ability_ranges, choose_npc_ability_within_reach, compute_backup_waypoint,
};
use super::fight_target::{select_target, Engagement};
use super::leash::policy as leash_policy;

/// NPC fighting behavior: attack top-threat target or leash if too far from spawn.
pub(super) async fn npc_ai_fight(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    use crate::cell::combat;
    use cimmeria_entity::cell_entity::MobMovementType;

    // Record CombatAdvance in the movement-type cache. Nothing goes on the
    // wire: the client has no movement-type receiver (NA10, see
    // `broadcast_movement_type`).
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::CombatAdvance),
        tx,
        space_mgr,
    )
    .await;

    // Who to fight, or why to stop (target-selection seam, NA12). A dead,
    // vanished or lost target is pruned there, with its combat state drained;
    // an empty list starts the walk home.
    let Some(Engagement {
        target_id,
        target_pos,
        spawn_pos,
        npc_pos,
        is_stationary,
        use_cover,
        leash_distance,
    }) = select_target(npc_id, tx, space_mgr).await
    else {
        return;
    };

    // Hard leash: the NPC itself is beyond the hysteresis band around its
    // spawn, or too far above or below it. Measured on the NPC, never on the
    // target (audit S3): a player 49.9 u from spawn no longer leashes an NPC
    // standing at its spawn.
    if let Some(spawn) = spawn_pos {
        if let Some(trigger) =
            leash_policy::leash_trigger(&npc_pos, &spawn, &target_pos, leash_distance, false)
        {
            let at = super::leash::LeashOutAt {
                spawn,
                npc_pos,
                target_pos,
                leash_distance,
            };
            super::leash::leash_out(npc_id, target_id, trigger, at, tx, space_mgr).await;
            return;
        }
    }

    // Distance is needed before the pick, not after: since H09 an ability
    // set can hold both a ranged and a melee auto-attack, and which of the
    // two is usable depends on how far away the target is.
    let dist_to_target = npc_pos.distance_to(&target_pos);

    // Pick the ability up front so the range check can gate on the
    // ability's own `min_range` / `max_range` instead of a flat
    // server-wide constant. `choose_npc_ability_within_reach` returns:
    //   - `Some(id)` for the lowest-id off-cooldown ability that can be
    //     used at `dist_to_target` — a melee ability only inside
    //     `NPC_MELEE_RANGE`, so a staff or ribbon swing is never played at
    //     a target the NPC cannot touch.
    //   - `Some(id)` for the lowest-id off-cooldown ability regardless of
    //     reach when none is in reach, so the out-of-range arm below can
    //     walk the NPC in (or hold it, if stationary) rather than freeze.
    //   - `Some(NPC_DEFAULT_ABILITY)` when the NPC has no known abilities
    //     (misconfigured template — explicit fallback per the selector's
    //     "don't wedge silently" rule).
    //   - `None` when every known ability is on cooldown.
    //
    // In the `None` case we keep the range/LOS logic running against the
    // server-wide fallback so the NPC still walks toward / tracks the
    // target while waiting for an off-cooldown ability — same effective
    // behavior as the pre-issue-329 flat-30.0 code path.
    let chosen_ability = choose_npc_ability_within_reach(
        npc_id,
        space_mgr,
        dist_to_target,
        combat::NPC_ATTACK_RANGE,
    );
    let (max_range, min_range) =
        ability_ranges(chosen_ability, space_mgr, combat::NPC_ATTACK_RANGE);

    // Range check: don't attack until target is within the chosen
    // ability's `max_range` (or `NPC_ATTACK_RANGE` if the def is missing
    // or carries the `0` sentinel meaning "use server default", or
    // `NPC_MELEE_RANGE` if the ability is melee). Pinned
    // Previously: prior code used the flat constant and ignored
    // per-ability `max_range`, which produced "NPC walks into firing
    // distance but stands there" for any ability with `max_range < 30`
    // (e.g., a grenade at `max_range = 15`).
    let in_range = dist_to_target <= max_range;
    let has_los = space_mgr.has_line_of_sight(npc_id, target_id);

    // Cover-system integration. When `use_cover` is on (set by the
    // spawner for NPCs from `SGWMob.def`'s `useCover` flag) and the
    // threat is engaged, `maintain_cover_for_npc` decides whether to
    // stay in the current cover, move to a new cover slot, or fall
    // back to direct chase. The returned `nav_target_pos` is the
    // position the chase block below paths toward — it overrides the
    // threat's position with the chosen cover slot's position so the
    // NPC paths to cover instead of running at the player.
    //
    // Reservation state is owned by the cover module; the function is
    // a pure decision against an atomic snapshot. Release on death /
    // leash / idle is handled elsewhere in this file.
    let nav_target_pos = super::fight_cover::route_via_cover(
        super::fight_cover::CoverStep {
            npc_id,
            target_id,
            npc_pos,
            world_id: space_mgr.get_entity_world_id(npc_id),
            target_pos,
            in_range,
            use_cover,
            is_stationary,
        },
        tx,
        space_mgr,
        engine,
    )
    .await;

    // Out of range OR occluded — keep pathfinding so the NPC can reposition
    // to regain line of sight. Treating "in range but blocked" as a stop
    // condition would freeze the NPC behind walls/corners; making it a repath
    // condition lets the AI walk around the obstruction.
    //
    // Stationary NPCs (turrets, fixed defenders) skip pathfinding entirely:
    // they hold position and only fire when the target enters range + LOS.
    // A pinned NPC never leaves its spawn, so the NPC-distance leash never
    // fires for it; it disengages when its target is lost instead (dead,
    // gone, or out of its AoI for the grace period, see `fight_target`).
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
            //
            // Turn toward the target even while holding fire. A pinned
            // NPC never gets a nav path, so the movement tick never
            // writes its yaw; without this a sentry being shot from out
            // of range (or across a navmesh gap that reads as no LoS)
            // keeps its authored heading and stands with its back to the
            // attacker. Harset seeds thirteen stationary sentries.
            face_target(space_mgr, npc_id, npc_pos, target_pos);
            super::note_outcome("stationary_holds");
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
        // Soft leash (hysteresis band): about to chase, already past the
        // leash radius, and the target is further from home than the NPC.
        // Inside the band an NPC that can hit its target keeps fighting;
        // only a chase that would drag it further out gives up.
        if let Some(spawn) = spawn_pos {
            if let Some(trigger) =
                leash_policy::leash_trigger(&npc_pos, &spawn, &target_pos, leash_distance, true)
            {
                let at = super::leash::LeashOutAt {
                    spawn,
                    npc_pos,
                    target_pos,
                    leash_distance,
                };
                super::leash::leash_out(npc_id, target_id, trigger, at, tx, space_mgr).await;
                return;
            }
        }
        let needs_repath = {
            let npc = space_mgr.get_entity(npc_id);
            match npc {
                Some(e) if !e.nav_path.is_empty() => {
                    // Check if nav target moved far from the last waypoint.
                    // Uses `nav_target_pos` (may be a cover-slot override)
                    // so cover-routed paths don't repath every tick.
                    let last_wp = match e.nav_path.back() {
                        Some(wp) => *wp,
                        None => return,
                    };
                    last_wp.distance_to(&nav_target_pos) > 5.0
                }
                _ => true, // No path — need one
            }
        };

        if needs_repath {
            let routed = super::path_request::request_path(
                space_mgr,
                super::path_request::PathRequest {
                    npc_id,
                    state: "fight",
                    from: npc_pos,
                    to: nav_target_pos,
                    target_id: Some(target_id),
                    partial_outcome: "chase_partial",
                },
                std::time::Instant::now(),
            );
            if let Some(path) = routed.waypoints {
                if path.len() > 1 {
                    let waypoints: std::collections::VecDeque<_> =
                        path.into_iter().skip(1).collect();
                    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                        super::replace_nav_path_on(npc, waypoints);
                    }
                    super::note_outcome("chase");
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
                } else {
                    super::note_outcome("repath_degenerate");
                    // Shared emitter — `in_range` / `has_los` /
                    // `dist_to_target` are already on the per-tick
                    // `npc_ai.tick` row for this same NPC, so dropping
                    // them here loses nothing and buys one query shape
                    // across all five AI states.
                    super::path_failure::report_path_failure(
                        space_mgr,
                        super::path_failure::PathFailure {
                            npc_id,
                            state: "fight",
                            decision_outcome: "repath_degenerate",
                            from: npc_pos,
                            to: nav_target_pos,
                            reason: super::path_failure::PathFailReason::DegeneratePath,
                            // `nav_path` is deliberately untouched on this
                            // branch: a one-waypoint repath is not worth
                            // discarding a route that may still be usable.
                            fallback: super::path_failure::PathFallback::PathUnchanged,
                            target_id: Some(target_id),
                        },
                        std::time::Instant::now(),
                    );
                }
            } else {
                // No-path is the diagnostic signal for "navmesh missing in
                // this zone" — see issue #407. The parent `npc_ai.decision`
                // span already carries `space_id`, so SigNoz can group
                // `groupBy=decision_outcome` across the npc_ai target and
                // pivot per zone via the span's space_id attribute.
                super::note_outcome("no_path");
                let reason = super::path_failure::PathFailReason::for_missing_path(
                    space_mgr,
                    npc_id,
                    routed.status,
                );
                super::path_failure::report_path_failure(
                    space_mgr,
                    super::path_failure::PathFailure {
                        npc_id,
                        state: "fight",
                        decision_outcome: "no_path",
                        from: npc_pos,
                        to: nav_target_pos,
                        reason,
                        // Unlike every other state, `fight` does **not**
                        // enqueue the raw target as a direct waypoint
                        // here — so the chaser stands still, or keeps
                        // walking a stale route. The shared message used
                        // to claim a straight-line fallback for this
                        // branch, which sent operators looking for a
                        // wall-clipping NPC that was never moving.
                        fallback: super::path_failure::PathFallback::PathUnchanged,
                        target_id: Some(target_id),
                    },
                    std::time::Instant::now(),
                );
            }
        } else {
            super::note_outcome("hold_no_repath");
            tracing::debug!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "hold_no_repath",
                npc_id,
                target_id,
                in_range,
                has_los,
                dist_to_target,
                "NPC AI: out of range/LoS but existing path still ends near the target -- no new order this tick"
            );
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
                super::replace_nav_path_on(npc, [backup]);
            }
            space_mgr
                .npc_detectors
                .note_move_source(npc_id, super::detectors::MoveSource::Backup);
            super::note_outcome("min_range_backup");
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
    //
    // Stopping zeroes velocity as well as clearing the path. Clearing only
    // the path left velocity at the chase speed, and the AoI tick kept
    // telling every witness the NPC was moving, so it ran in place (NA10,
    // audit S1).
    //
    // Face the target too. `direction` was only ever written by the movement
    // tick, which skips path-less NPCs -- and this branch clears the path --
    // so an attacker's yaw froze the moment it stopped while the player
    // strafed around it. Done before the ability check so a mob waiting on a
    // cooldown still tracks its target. No extra wire traffic: the AoI tick
    // already sends direction with every position update.
    super::stop_npc_movement(space_mgr, npc_id, super::StopReason::AttackInPlace);
    face_target(space_mgr, npc_id, npc_pos, target_pos);

    // `chosen_ability` may still be `None` here when every known ability
    // is on cooldown — hold fire and let the next tick re-evaluate.
    let chosen_ability = match chosen_ability {
        Some(id) => id,
        None => {
            super::note_outcome("no_ability");
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

    super::note_outcome("attack_in_place");
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
    let fired = crate::cell::abilities::handle_use_ability(
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

/// Point `npc_id`'s yaw at `target_pos`. `direction` is `[pitch, yaw, roll]`
/// in radians and yaw is `atan2(dx, dz)` (0 = +Z), the same convention the
/// movement tick writes. A target directly above or below (coincident in
/// XZ) has no bearing, so the current yaw is kept rather than snapped to 0.
/// No wire traffic of its own: the AoI tick sends direction with every
/// position update.
///
/// `pub(super)` so the surrender path in [`super::lifecycle`] can reuse
/// it: an NPC that disengages must end up facing the player it gave up
/// to, and that is the same geometry with a different trigger.
pub(super) fn face_target(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
) {
    let (dx, dz) = (target_pos.x - npc_pos.x, target_pos.z - npc_pos.z);
    if dx * dx + dz * dz < f32::EPSILON {
        return;
    }
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.direction = cimmeria_common::Vector3::new(0.0, dx.atan2(dz), 0.0);
    }
}
