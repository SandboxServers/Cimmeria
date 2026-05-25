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

    // Range check: don't attack until target is within weapon range
    let dist_to_target = npc_pos.distance_to(&target_pos);
    let in_range = dist_to_target <= combat::NPC_ATTACK_RANGE;
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

    // In range and LOS confirmed — stop moving and attack.
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
    }

    // Mirrors `python/cell/SGWMob.py:chooseAbility`. Range gating already
    // happened above; selector only sees cooldown state. `None` → hold fire.
    let chosen_ability = match choose_npc_ability(npc_id, space_mgr) {
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
