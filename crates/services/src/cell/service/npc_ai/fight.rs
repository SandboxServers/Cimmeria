//! Fighting state: attack the top-threat target (with cover routing,
//! range/LOS gating, and min-range backup) or transition to Leashing.
//! Also the Idle-auto-aggro seed that promotes an aggressive idle NPC
//! into combat.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::ability_select::{ability_ranges, choose_npc_ability, compute_backup_waypoint};

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
pub(super) async fn npc_ai_idle_auto_aggro(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use crate::cell::combat;

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
            crate::cell::abilities::request_appearance_refresh(player_id, tx, space_mgr).await;
        }
    }
}

/// NPC fighting behavior: attack top-threat target or leash if too far from spawn.
pub(super) async fn npc_ai_fight(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    use crate::cell::combat;
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    // Movement-type broadcast on Fighting entry. Dedup'd against the
    // cached `last_movement_type` — subsequent Fighting ticks are
    // no-ops on the wire. See `broadcast_movement_type` doc for
    // rationale (animation hint, not gameplay-side state).
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::CombatAdvance),
        tx,
        space_mgr,
    )
    .await;

    // Read NPC state (immutable borrow)
    let (top_target, spawn_pos, npc_pos, is_stationary, use_cover) = {
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

        (
            top,
            npc.spawn_position,
            npc.position,
            npc.is_stationary,
            npc.use_cover,
        )
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
            // Release any cover slot the NPC was holding — combat ended.
            // `release_for_entity` is idempotent; the call is cheap when
            // the NPC wasn't in cover.
            space_mgr
                .cover
                .release_for_entity(cimmeria_common::EntityId(npc_id as i32));
            // Clear cached movement-type so the next Fighting entry
            // re-broadcasts. None means "no wire emission, just drop
            // the dedup cache" per `broadcast_movement_type` doc.
            crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
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
            // Release any cover slot held — leash is a combat-end transition.
            space_mgr
                .cover
                .release_for_entity(cimmeria_common::EntityId(npc_id as i32));
            // Broadcast Leash movement-type now rather than wait for
            // the next AI tick (which would land ~2s later). The leash
            // handler itself snaps the position instantly, so even
            // though there's no actual leash-walk yet, the wire
            // signal lets the client play any leash-specific VFX it
            // wants for one frame before the corpse snaps home.
            crate::cell::abilities::broadcast_movement_type(
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
    let mut nav_target_pos = target_pos;
    if use_cover && !is_stationary {
        use crate::cell::cover::{maintain_cover_for_npc, CoverDecision, CoverWeights};
        let decision = maintain_cover_for_npc(
            cimmeria_common::EntityId(npc_id as i32),
            npc_pos,
            target_pos,
            in_range,
            true,
            &space_mgr.cover,
            &CoverWeights::default(),
        );
        match decision {
            CoverDecision::StayInCover { pos, slot } => {
                nav_target_pos = pos;
                tracing::debug!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "stay_in_cover",
                    npc_id,
                    target_id,
                    chunk_id = slot.chunk_id,
                    node_id = slot.node_id,
                    "NPC AI: holding cover slot"
                );
            }
            CoverDecision::MoveToCover { pos, slot } => {
                nav_target_pos = pos;
                tracing::info!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "move_to_cover",
                    npc_id,
                    target_id,
                    chunk_id = slot.chunk_id,
                    node_id = slot.node_id,
                    "NPC AI: picked cover slot"
                );
            }
            CoverDecision::Released { prior_slot } => {
                tracing::info!(
                    target: "npc_ai",
                    event = "decision",
                    decision_outcome = "cover_released_flanked",
                    npc_id,
                    target_id,
                    chunk_id = prior_slot.chunk_id,
                    node_id = prior_slot.node_id,
                    "NPC AI: released flanked cover slot, re-evaluating next tick"
                );
                // Clear any stale nav_path pointing at the now-released
                // cover slot. Without this, the NPC would continue
                // walking toward the abandoned slot for one more
                // movement tick before the re-pick lands next AI tick.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.nav_path.clear();
                }
                // Fire the OnNpcFlanked content trigger so chain
                // authors can hook narrative reactions (the AI itself
                // already repositions; this is just the affordance).
                let npc_template = space_mgr
                    .get_entity(npc_id)
                    .and_then(|e| e.npc_name.clone())
                    .unwrap_or_default();
                crate::cell::content::fire_npc_flanked(
                    npc_id,
                    target_id,
                    &npc_template,
                    engine,
                    tx,
                    space_mgr,
                )
                .await;
            }
            CoverDecision::NoCover => {}
        }
    }

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
            if let Some(path) = space_mgr.find_path(npc_id, &npc_pos, &nav_target_pos) {
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
