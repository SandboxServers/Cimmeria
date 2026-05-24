//! NPC AI tick — fight (threat-target attacks + leashing), and leash recovery.
//!
//! Runs every 2 seconds (every 20th AoI tick).

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

/// NPC AI tick — drive the three Fighting/Leashing/Idle-with-aggression
/// paths. For NPCs in Fighting: find top threat target, check leash, attack
/// via the three-bucket selector. For NPCs in Leashing: reset to Idle when
/// close enough to spawn point, restore health. For NPCs in Idle whose
/// `aggression > 0`: scan AoI witnesses for opposing-faction players, seed
/// threat on the closest one (transitions to Fighting on the next tick).
///
/// The Idle-with-aggression path is what makes `set_aggression(level=1)`
/// content actions actually drive combat — see
/// `python/cell/missions/Castle_CellBlock/FindAmbernol.py:99` and
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
/// Scans the NPC's AoI witnesses (players who can see this NPC) and seeds
/// threat on the closest opposing-faction player. The next AI tick finds
/// the NPC in `Fighting` (the threat seed transitioned it via
/// `combat::generate_threat`) and runs the normal fight path.
///
/// "Opposing faction" is any faction value different from the NPC's own —
/// the simple rule the curated chains expect. Chain 1032's drone is
/// faction 10 (hostile-NID); the player joins at faction 0 (neutral).
/// Different = opposing for the purposes of auto-aggro.
///
/// The seed threat is intentionally small (`1.0`) so that an explicit
/// `generate_threat 1000` from the same chain (chain 1032 has both)
/// dominates the threat list and focuses the drone on the triggering
/// player rather than whichever player happens to be closest.
fn npc_ai_idle_auto_aggro(npc_id: u32, space_mgr: &mut SpaceManager) {
    use super::super::combat;

    // Defensive gate: the outer `npc_ai_tick` already filters on
    // `aggression > 0`, but a direct caller (test, future driver) could
    // skip that filter. Bail out here too so the function is safe to
    // call on any Idle NPC — passive ones remain passive.
    let (npc_pos, npc_faction) = match space_mgr.get_entity(npc_id) {
        Some(e) if e.aggression > 0 => (e.position, e.faction),
        Some(_) | None => return,
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

    // Three-bucket ability selection. Pick the ability with the fewest
    // gates blocking it — `usable` if available, otherwise hold fire and
    // wait for cooldowns. `None` means every ability is cooling or
    // needs ammo; the AI ticks again in 2s and tries again.
    //
    // Mirrors `python/cell/SGWMob.py:chooseAbility` — three buckets,
    // pick-from-usable, hold otherwise. Without per-ability range gating
    // (#329's territory) we use the same flat `NPC_ATTACK_RANGE` we
    // already pass before reaching this point; once #329 lands the
    // selector itself can read each ability's `max_range`/`min_range`
    // from `AbilityDef` and gate per-ability.
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
    super::super::abilities::handle_use_ability(
        npc_id,
        chosen_ability,
        target_id as i32,
        tx,
        space_mgr,
    )
    .await;
}

/// Three-bucket ability picker for an NPC's fight tick.
///
/// Partition the NPC's known abilities into:
///
/// - **usable** — off cooldown AND (`required_ammo == 0` OR ammo available)
/// - **cooling** — on cooldown (will become usable when the cooldown clears)
/// - **needs_ammo** — out of ammo (the ammo refill path will eventually unblock)
///
/// Returns the first usable ability, or `None` if every ability is gated.
/// Falls back to [`combat::NPC_DEFAULT_ABILITY`] when the NPC has no known
/// abilities at all (a misconfigured template) so the AI tick never wedges
/// silently — the dispatch will then reject the unknown-ability call with a
/// clear log line.
///
/// Range/LOS gating is deliberately NOT done here — those decisions live in
/// the AI tick's geometry check (and will move to per-ability evaluation
/// once #329 lands). The selector is purely about "what's ready to fire
/// right now," not "what's in range of which target."
///
/// Mirrors the python `chooseAbility` shape on `SGWMob.py` (multi-ability
/// NPCs scan their list, partition, pick the first usable).
fn choose_npc_ability(npc_id: u32, space_mgr: &SpaceManager) -> Option<i32> {
    use super::super::combat;

    let npc = space_mgr.get_entity(npc_id)?;
    if npc.abilities.known_count() == 0 {
        return Some(combat::NPC_DEFAULT_ABILITY);
    }

    // Stable iteration order — `known_ability_ids` returns HashSet contents
    // unsorted, so sort here to keep tick-to-tick selection deterministic.
    // A future "prefer higher threat_level_id" refinement just changes this
    // ordering without touching the bucket partition.
    let mut ability_ids = npc.abilities.known_ability_ids();
    ability_ids.sort_unstable();

    for ability_id in ability_ids {
        if npc.abilities.is_on_cooldown(ability_id) {
            // Cooling bucket — skip.
            continue;
        }
        // Ammo gate. NPCs in this codebase don't track ammo (they have
        // infinite ammo by convention — the `required_ammo > 0` guard at
        // `use_ability.rs:150,160,185` is player-only). For NPC selection
        // we treat `required_ammo > 0` as "needs ammo" and skip, so a
        // future ammo-bearing NPC archetype slots in without revisiting
        // the bucket logic. Today every NPC ability is `required_ammo = 0`
        // (Pistol Shot 592, Energy Shock 221), so this branch is dormant
        // until we ship one that isn't.
        let required_ammo = space_mgr
            .ability_defs
            .get(&ability_id)
            .map_or(0, |d| d.required_ammo);
        if required_ammo > 0 {
            // needs_ammo bucket — skip.
            continue;
        }
        // Usable bucket — first match wins.
        return Some(ability_id);
    }

    // All abilities gated (cooling or needs_ammo). Caller holds fire.
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::AbilityDef;
    use cimmeria_entity::cell_entity::AiState;

    /// Castle test space + one NPC at the origin and (optionally) one
    /// player. The NPC defaults to `class_id = 0x04` so the AI tick sees
    /// it; the caller flips faction / aggression / ai_state as needed.
    fn make_mgr_with_npc_and_player(
        npc_id: u32,
        npc_faction: u8,
        player_id: u32,
        player_pos: [f32; 3],
    ) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        // NPC at origin.
        mgr.spawn_npc(npc_id, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.faction = npc_faction;
            npc.ai_state = AiState::Idle;
            // Drop the default NPC_DEFAULT_ABILITY seeded by `spawn_npc` so
            // selector tests can control the bucket precisely.
            npc.abilities
                .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        }
        // Player as a co-located entity in the same space (witness of NPC).
        mgr.create_entity(player_id, "Castle", player_pos, [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(player_id) {
            p.is_player = true;
            p.player_id = Some(player_id as i32);
            p.faction = 0;
        }
        mgr.connect_entity(player_id);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    fn make_ability_def(id: i32, required_ammo: i32) -> AbilityDef {
        AbilityDef {
            ability_id: id,
            name: format!("test-{id}"),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo,
            event_set_id: None,
            velocity: 0.0,
        }
    }

    /// `aggression > 0` on an Idle NPC with an opposing-faction player in
    /// AoI seeds threat → NPC transitions to Fighting on the next tick.
    /// Bug shape: the previous `set_aggression` wrote a property nothing
    /// read, so the drone sat idle forever.
    #[test]
    fn idle_npc_with_aggression_aggros_opposing_player() {
        let mut mgr = make_mgr_with_npc_and_player(200_001, 10, 1, [5.0, 0.0, 0.0]);
        if let Some(npc) = mgr.get_entity_mut(200_001) {
            npc.aggression = 1;
        }

        npc_ai_idle_auto_aggro(200_001, &mut mgr);

        let npc = mgr.get_entity(200_001).unwrap();
        assert_eq!(
            npc.ai_state,
            AiState::Fighting,
            "aggression=1 with opposing-faction witness must seed threat → Fighting",
        );
        assert!(
            npc.threat_list.contains_key(&1),
            "player must be on NPC threat list after auto-aggro",
        );
    }

    /// `aggression == 0` (default) keeps the NPC Idle even with a hostile
    /// player in AoI. The pre-existing "passive until attacked" baseline.
    #[test]
    fn idle_npc_without_aggression_stays_idle() {
        let mut mgr = make_mgr_with_npc_and_player(200_002, 10, 1, [5.0, 0.0, 0.0]);
        // aggression defaults to 0 — explicit assert just for clarity.
        assert_eq!(mgr.get_entity(200_002).unwrap().aggression, 0);

        npc_ai_idle_auto_aggro(200_002, &mut mgr);

        let npc = mgr.get_entity(200_002).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle);
        assert!(npc.threat_list.is_empty());
    }

    /// `aggression > 0` but the witness shares the NPC's faction — no
    /// aggro. Same-faction NPCs in the same AoI must not auto-aggro each
    /// other (matters once the NPCs-can-see-NPCs faction logic ships).
    #[test]
    fn aggression_skips_same_faction_witnesses() {
        // NPC faction 0 == player faction 0 (player default).
        let mut mgr = make_mgr_with_npc_and_player(200_003, 0, 1, [5.0, 0.0, 0.0]);
        if let Some(npc) = mgr.get_entity_mut(200_003) {
            npc.aggression = 1;
        }

        npc_ai_idle_auto_aggro(200_003, &mut mgr);

        let npc = mgr.get_entity(200_003).unwrap();
        assert_eq!(npc.ai_state, AiState::Idle, "same faction must not aggro");
        assert!(npc.threat_list.is_empty());
    }

    /// Three-bucket selector: NPC with two abilities, one on cooldown, one
    /// fresh — selector returns the fresh one. Pins that the cooldown gate
    /// participates in the partition.
    #[test]
    fn selector_skips_cooling_ability() {
        let mut mgr = make_mgr_with_npc_and_player(200_004, 10, 1, [5.0, 0.0, 0.0]);
        mgr.ability_defs.insert(221, make_ability_def(221, 0));
        mgr.ability_defs.insert(592, make_ability_def(592, 0));
        if let Some(npc) = mgr.get_entity_mut(200_004) {
            npc.abilities.add_ability(221);
            npc.abilities.add_ability(592);
            // Sort order is ascending, so 221 comes first — put it on
            // cooldown so the selector must skip past it to pick 592.
            npc.abilities
                .start_ability_cooldown(221, std::time::Duration::from_secs(10));
        }

        let chosen = choose_npc_ability(200_004, &mgr);
        assert_eq!(chosen, Some(592), "cooling 221 → selector must pick 592");
    }

    /// Three-bucket selector: ability with `required_ammo > 0` goes into
    /// the needs_ammo bucket and gets skipped. NPCs don't track ammo in
    /// this codebase, so any ammo-bearing ability is unfireable; the
    /// selector treats it correctly anyway.
    #[test]
    fn selector_skips_needs_ammo_ability() {
        let mut mgr = make_mgr_with_npc_and_player(200_005, 10, 1, [5.0, 0.0, 0.0]);
        mgr.ability_defs.insert(221, make_ability_def(221, 0));
        // Synthetic ability 200 with required_ammo > 0.
        mgr.ability_defs.insert(200, make_ability_def(200, 5));
        if let Some(npc) = mgr.get_entity_mut(200_005) {
            npc.abilities.add_ability(200);
            npc.abilities.add_ability(221);
        }

        let chosen = choose_npc_ability(200_005, &mgr);
        assert_eq!(
            chosen,
            Some(221),
            "ammo-bearing 200 → selector must pick 221"
        );
    }

    /// Three-bucket selector: every ability is on cooldown → return None
    /// so the AI tick holds fire rather than misfiring.
    #[test]
    fn selector_returns_none_when_all_cooling() {
        let mut mgr = make_mgr_with_npc_and_player(200_006, 10, 1, [5.0, 0.0, 0.0]);
        mgr.ability_defs.insert(221, make_ability_def(221, 0));
        if let Some(npc) = mgr.get_entity_mut(200_006) {
            npc.abilities.add_ability(221);
            npc.abilities
                .start_ability_cooldown(221, std::time::Duration::from_secs(10));
        }

        let chosen = choose_npc_ability(200_006, &mgr);
        assert_eq!(chosen, None, "all cooling → hold fire");
    }

    /// Three-bucket selector: NPC with no abilities at all (misconfigured
    /// template) falls back to `NPC_DEFAULT_ABILITY` so the tick never
    /// wedges silently.
    #[test]
    fn selector_falls_back_to_default_when_no_abilities() {
        let mgr = make_mgr_with_npc_and_player(200_007, 10, 1, [5.0, 0.0, 0.0]);
        // Empty ability bucket (the fixture already removed NPC_DEFAULT_ABILITY).

        let chosen = choose_npc_ability(200_007, &mgr);
        assert_eq!(
            chosen,
            Some(crate::cell::combat::NPC_DEFAULT_ABILITY),
            "empty bucket → fallback to NPC_DEFAULT_ABILITY",
        );
    }
}
