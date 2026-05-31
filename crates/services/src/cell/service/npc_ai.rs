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
        );
        async {
            match ai_state {
                AiState::Fighting => npc_ai_fight(npc_id, tx, space_mgr).await,
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

/// NPC patrol behavior: walk a waypoint loop with arrival-based
/// dwell pauses.
///
/// State machine within Patrol (in evaluation order):
/// 1. **`patrol_path` empty** → drop to `Idle` and clear the
///    movement-type cache. Reachable only if a content action wipes
///    the path mid-tick.
/// 2. **`nav_path` non-empty** → `npc_movement_tick` is walking the
///    NPC toward the current target waypoint; no work this tick.
/// 3. **`nav_path` empty + close to target + dwell `None`** → just
///    arrived (or first entry). Stamp `patrol_dwell_until = now +
///    delay_secs`.
/// 4. **`nav_path` empty + close to target + dwell in the future**
///    → still pausing at the waypoint; no-op.
/// 5. **`nav_path` empty + close to target + dwell elapsed** →
///    advance `patrol_next_index` modulo path length and clear the
///    dwell. The next tick observes `not close` against the new
///    target index and queues movement.
/// 6. **`nav_path` empty + NOT close to target** → pathfind to the
///    current target waypoint and push the result into `nav_path`.
///    Also clears `patrol_dwell_until` — leaving a `Some(past)` here
///    would cause the re-arrival from a knockback to skip the
///    dwell.
///
/// The "close" threshold is `< 1.0` world units. `npc_movement_tick`
/// snaps to a waypoint when `distance <= move_speed` (default 0.6),
/// so post-arrival position is exactly on the waypoint; the 1.0
/// slack absorbs floating-point round-trips and keeps the
/// comparison well under any meaningful patrol distance.
///
/// Threat preemption is handled outside: when `generate_threat`
/// flips the state from Patrol → Fighting, the next AI tick routes
/// through `npc_ai_fight`. On Fighting → Leashing → Idle, the
/// tick's Idle branch transitions back to Patrol and the saved
/// `patrol_next_index` resumes the route from where it left off
/// (no progress lost).
async fn npc_ai_patrol(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::MobMovementType;

    let now = std::time::Instant::now();

    // Read patrol state without holding a borrow across the
    // pathfind / nav_path write below.
    let (path_empty, nav_empty, dwell, target_index, delay_secs, target_waypoint, npc_pos) = {
        let npc = match space_mgr.get_entity(npc_id) {
            Some(e) => e,
            None => return,
        };
        if npc.patrol_path.is_empty() {
            (
                true,
                true,
                None,
                0,
                0.0,
                None,
                cimmeria_common::Vector3::zero(),
            )
        } else {
            let next_idx = npc.patrol_next_index % npc.patrol_path.len();
            (
                false,
                npc.nav_path.is_empty(),
                npc.patrol_dwell_until,
                next_idx,
                npc.patrol_point_delay_secs,
                Some(npc.patrol_path[next_idx]),
                npc.position,
            )
        }
    };

    // Empty-path drop fires BEFORE the Patrol broadcast so the wire
    // doesn't see a Patrol byte for an NPC that's about to leave
    // the state. The drop also broadcasts None to clear the cache.
    if path_empty {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    // Broadcast Patrol movement-type. Dedup'd against
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

    if !nav_empty {
        // Movement in flight — npc_movement_tick is walking the NPC
        // toward the current waypoint. Nothing to do this tick.
        return;
    }

    // nav_path is empty. Are we at the target waypoint (arrived) or
    // never started (still need to queue movement)?
    //
    // The "close" threshold is 1.0 world units — `npc_movement_tick`
    // snaps to a waypoint when `dist <= move_speed` (default 0.6),
    // so the position will be exactly on the waypoint after arrival.
    // 1.0 gives a small slack for floating-point round-trips while
    // staying well under the smallest meaningful patrol distance.
    let Some(waypoint) = target_waypoint else {
        return;
    };
    let close = npc_pos.distance_to(&waypoint) < 1.0;

    if close {
        // At the waypoint. Dwell logic.
        match dwell {
            None => {
                // Just arrived — stamp the dwell deadline. Subsequent
                // ticks observe `Some(deadline)` and either keep
                // waiting or advance.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    let secs = delay_secs.max(0.5);
                    npc.patrol_dwell_until = Some(now + std::time::Duration::from_secs_f32(secs));
                }
                tracing::debug!(
                    target: "npc_ai",
                    event = "patrol_arrived",
                    npc_id,
                    target_index,
                    delay_secs,
                    "NPC AI: patrol → arrived, dwelling"
                );
            }
            Some(deadline) if now < deadline => {
                // Still dwelling — no-op.
            }
            Some(_) => {
                // Dwell elapsed — advance to next waypoint. Clear the
                // dwell deadline; the next tick will observe
                // `close = false` (because the target is now a
                // different waypoint) and queue movement.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    let len = npc.patrol_path.len();
                    npc.patrol_next_index = (npc.patrol_next_index + 1) % len;
                    npc.patrol_dwell_until = None;
                }
            }
        }
    } else {
        // Not at the target — pathfind and queue movement. Clearing
        // `patrol_dwell_until` here matters for the knockback case:
        // if the NPC dwelled at the waypoint, got pushed off, and is
        // now walking back, leaving `Some(past)` on the entity would
        // make the next arrival fall into the "elapsed → advance"
        // branch and skip the remainder of the dwell. Clearing means
        // the re-arrival re-stamps from scratch, which is the
        // expected "pause for delay_secs after arriving" semantic.
        let path = space_mgr
            .find_path(npc_id, &npc_pos, &waypoint)
            .unwrap_or_default();
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.patrol_dwell_until = None;
            npc.nav_path.clear();
            if path.len() > 1 {
                // Skip the first entry (start position). Detour returns
                // a straight-path that includes both endpoints.
                for wp in path.into_iter().skip(1) {
                    npc.nav_path.push_back(wp);
                }
            } else {
                // Pathfind failed or returned a single point — direct push.
                npc.nav_path.push_back(waypoint);
            }
        }
        tracing::debug!(
            target: "npc_ai",
            event = "patrol_waypoint_set",
            npc_id,
            target_index,
            wp_x = waypoint.x,
            wp_y = waypoint.y,
            wp_z = waypoint.z,
            "NPC AI: patrol → next waypoint queued"
        );
    }
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

    // Zero-radius drop fires BEFORE the Wander broadcast so the wire
    // doesn't see a Wander byte for an NPC that's about to leave
    // Wander this same tick.
    if radius <= 0.0 {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Wander),
        tx,
        space_mgr,
    )
    .await;

    if !nav_empty {
        // Movement tick is walking — nothing to do.
        return;
    }

    // Arrival-based dwell semantics (matches Patrol + Investigating
    // and the column comment on `wander_min/max_dwell_secs`):
    //
    // - `wander_next_at` None + nav_empty → just arrived (or first
    //   entry). Sample a dwell duration and stamp the deadline.
    // - `wander_next_at` Some(future) + nav_empty → dwelling at the
    //   destination, no-op.
    // - `wander_next_at` Some(elapsed) + nav_empty → dwell complete,
    //   pick a fresh destination, route, clear the deadline so the
    //   next arrival re-stamps.
    //
    // The "first entry" case stamps dwell at spawn_position: the NPC
    // pauses at spawn for [min, max] seconds before its first hop.
    // Acceptable initial behavior; alternative would be to sample
    // immediately and skip the dwell-at-spawn beat.
    match wander_next {
        Some(deadline) if now < deadline => {
            // Dwelling at the current destination.
            return;
        }
        None => {
            // Just arrived (or first call). Stamp dwell.
            //
            // Seed the per-call RNG from a wall-clock source for real
            // entropy (`Instant::elapsed()` from a same-function-frame
            // `now` would give only nanosecond jitter). The
            // multiplicative golden-ratio constant per `npc_id` ensures
            // two NPCs sampling on the same nanosecond get different
            // dwell durations.
            let wall_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let seed = u64::from(npc_id).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ wall_nanos;
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let dwell_secs = rng
                .random_range(min_dwell..=max_dwell.max(min_dwell))
                .max(0.5);
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.wander_next_at = Some(now + std::time::Duration::from_secs_f32(dwell_secs));
            }
            tracing::debug!(
                target: "npc_ai",
                event = "wander_arrived",
                npc_id,
                dwell_secs,
                "NPC AI: wander → arrived, dwelling"
            );
            return;
        }
        Some(_) => {
            // Dwell elapsed → fall through to "pick a fresh destination".
        }
    }

    let Some(spawn) = spawn_pos else {
        // No spawn anchor — wander can't pick a destination. Drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        return;
    };

    let wall_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seed = u64::from(npc_id).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ wall_nanos;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let angle = rng.random_range(0.0..(std::f32::consts::TAU));
    let distance = rng.random_range(0.0..radius);
    let _ = (min_dwell, max_dwell); // sampled in the arrival branch above
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
        // Clear the dwell deadline now that we're starting the next
        // hop. The next arrival (`nav_empty` again) will see
        // `wander_next_at = None` and re-stamp from the arrival
        // branch above.
        npc.wander_next_at = None;
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(target);
        }
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

    let (poi, npc_pos, nav_empty, dwell) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.poi,
            e.position,
            e.nav_path.is_empty(),
            e.investigate_until,
        ),
        None => return,
    };

    // No-POI drop fires BEFORE the CombatAdvance broadcast so the
    // wire doesn't see a movement-type for an NPC that's about to
    // leave Investigating this tick.
    let Some(poi_pos) = poi else {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = AiState::Idle;
            npc.investigate_until = None;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

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

    if !nav_empty {
        return; // Movement in flight.
    }

    // nav_path empty. Either we've arrived at the POI or we haven't
    // started yet. Use position-vs-POI distance to distinguish.
    let close = npc_pos.distance_to(&poi_pos) < 1.0;
    let now = std::time::Instant::now();

    if close {
        // At the POI. Dwell logic mirrors patrol.
        match dwell {
            None => {
                // Just arrived — stamp dwell.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.investigate_until =
                        Some(now + std::time::Duration::from_secs_f32(INVESTIGATE_DWELL_SECS));
                }
                tracing::debug!(
                    target: "npc_ai",
                    event = "investigate_arrived",
                    npc_id,
                    "NPC AI: investigate → arrived at POI, dwelling"
                );
            }
            Some(deadline) if now < deadline => {
                // Still dwelling.
            }
            Some(_) => {
                // Dwell elapsed → return to Idle.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.ai_state = AiState::Idle;
                    npc.poi = None;
                    npc.investigate_until = None;
                }
                super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
            }
        }
    } else {
        // Not at POI — pathfind and queue movement. Clearing
        // `investigate_until` here handles the knockback case: if
        // the NPC was dwelling at the POI and got pushed off, the
        // re-arrival should re-stamp from scratch rather than
        // observe `Some(past)` and immediately return to Idle.
        let path = space_mgr
            .find_path(npc_id, &npc_pos, &poi_pos)
            .unwrap_or_default();
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.investigate_until = None;
            npc.nav_path.clear();
            if path.len() > 1 {
                for wp in path.into_iter().skip(1) {
                    npc.nav_path.push_back(wp);
                }
            } else {
                npc.nav_path.push_back(poi_pos);
            }
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

/// NPC follow behavior: maintain a distance band to a target entity.
///
/// State machine within Follow:
/// - **No follow_target_id** → drop to Idle.
/// - **Target gone (entity removed)** → clear follow_target_id,
///   drop to Idle.
/// - **Target in band** (`min <= dist <= max`) → no work; stay put.
/// - **Target above max** → pathfind to a point one `min_distance`
///   short of the target so the NPC settles inside the band rather
///   than running all the way up to the target.
/// - **Target below min** → no work (NPCs don't back away).
async fn npc_ai_follow(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    let (target_id, npc_pos, min_d, max_d, nav_empty) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.follow_target_id,
            e.position,
            e.follow_min_distance,
            e.follow_max_distance,
            e.nav_path.is_empty(),
        ),
        None => return,
    };

    // No-target / gone-target drops fire BEFORE the Follow broadcast
    // so the wire doesn't see a Follow byte for an NPC that's about
    // to leave Follow this same tick.
    let Some(target_id) = target_id else {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    let Some(target_pos) = space_mgr.get_entity(target_id).map(|e| e.position) else {
        // Target despawned/disconnected. Clear and drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.follow_target_id = None;
            npc.ai_state = AiState::Idle;
        }
        super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    super::super::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Follow),
        tx,
        space_mgr,
    )
    .await;

    let dist = npc_pos.distance_to(&target_pos);
    if dist < min_d {
        // Too close — hold position.
        return;
    }
    if dist <= max_d {
        // In band — hold position.
        return;
    }

    if !nav_empty {
        // Movement in flight toward the target.
        return;
    }

    // Out of band — pathfind to a point one min_distance short of
    // the target along the line between the NPC and the target.
    let dx = target_pos.x - npc_pos.x;
    let dy = target_pos.y - npc_pos.y;
    let dz = target_pos.z - npc_pos.z;
    let mag = (dx * dx + dy * dy + dz * dz).sqrt();
    let stop_distance = min_d.max(0.1);
    let scale = ((mag - stop_distance) / mag).max(0.0);
    let dest = cimmeria_common::Vector3::new(
        npc_pos.x + dx * scale,
        npc_pos.y + dy * scale,
        npc_pos.z + dz * scale,
    );
    let path = space_mgr
        .find_path(npc_id, &npc_pos, &dest)
        .unwrap_or_default();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(dest);
        }
    }
    tracing::debug!(
        target: "npc_ai",
        event = "follow_routed",
        npc_id,
        target_id,
        dist,
        max_d,
        "NPC AI: follow → pathfinding toward target"
    );
}

/// NPC despawn behavior: remove the entity from the space. Used by
/// scripted cleanup (e.g., "the boss died, his bodyguards retreat
/// off-screen"). The destroy fires AoI-left events to all witnesses.
///
/// One-shot: the entity is gone by the time this returns, so any
/// subsequent tick filters skip it naturally.
async fn npc_ai_despawn(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Clear the movement-type cache first so the wire state is clean
    // before the destroy. The broadcast itself is dedup'd on None and
    // emits nothing — this is purely a state-clean step.
    super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(npc_id, "NPC AI: despawn → removing entity from space");
    space_mgr.destroy_entity(npc_id);
}

/// NPC submit behavior: the NPC surrenders. Clears combat state and
/// holds position. The AI tick will keep admitting Submit on every
/// pass (since the snapshot filter permits it), so the handler stays
/// cheap — broadcast None once, no further work. Content authors
/// destroy or transition the NPC when they're done with it.
async fn npc_ai_submit(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use super::super::combat;
    // Cache check: only do the heavy work on first entry. After that,
    // last_movement_type is None and we early-out.
    let needs_init = space_mgr
        .get_entity(npc_id)
        .is_some_and(|e| e.last_movement_type.is_some() || !e.threat_list.is_empty());
    if !needs_init {
        return;
    }
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.threat_list.clear();
        npc.nav_path.clear();
        npc.velocity = [0.0; 3];
        npc.state_field &= !combat::BSF_IN_COMBAT;
    }
    super::super::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
    tracing::info!(npc_id, "NPC AI: submit → combat state cleared, holding");
}

/// NPC error behavior: diagnostic fallback. Halts AI work (no
/// pathfind, no broadcast cadence). Logged once per entry so a stuck
/// NPC doesn't fill the log stream. Used by the `enterErrorAIState`
/// slash command and by the AI tick when it catches an unrecoverable
/// inconsistency (future).
async fn npc_ai_error(
    npc_id: u32,
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) {
    // No-op per tick — Error is a quiescent diagnostic state. The
    // entry log is emitted by whatever transitioned the NPC into
    // Error (typically the content action or the slash command).
    tracing::debug!(npc_id, "NPC AI: error state — holding");
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
