//! NPC aggro and threat list management.
//!
//! Threat is accumulated per attacker on each NPC. The NPC AI picks its
//! current target as the entity with the highest threat. First hit also
//! transitions the NPC from `Idle` to `Fighting`.

/// Leash distance in world units — if an NPC's target moves further than this
/// from the NPC's spawn position, the NPC resets and walks home.
pub const LEASH_DISTANCE: f32 = 50.0;

/// Maximum attack range in world units for NPC ranged attacks.
/// NPCs won't fire until the target is within this distance.
pub const NPC_ATTACK_RANGE: f32 = 30.0;

/// Default NPC attack ability ID: "Pistol Shot" (ability 592, ranged DD).
/// Was incorrectly 597 ("Heal Focus") — a self-heal, not an attack.
pub const NPC_DEFAULT_ABILITY: i32 = 592;

/// Generate threat on an NPC target from an attacker.
///
/// Transitions the NPC from Idle to Fighting on first hit, accumulates
/// threat so the NPC knows who to attack back, and (if the attacker is a
/// player) mirrors the addition into the player's `threatened_mobs` set
/// so multi-mob aggro tracking stays consistent.
///
/// Returns `Some(new_state_field)` when the attacker just entered combat
/// (i.e., their `threatened_mobs` was empty before this call) — the
/// caller is responsible for broadcasting `onStateFieldUpdate` to AoI
/// witnesses so the in-combat HUD/cursor flips. Returns `None` when no
/// broadcast is needed.
#[must_use]
pub fn generate_threat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    attacker_id: u32,
    target_id: u32,
    threat_amount: f32,
) -> Option<u32> {
    use cimmeria_entity::cell_entity::AiState;

    let target_is_npc = if let Some(target) = space_mgr.get_entity_mut(target_id) {
        if target.is_player {
            return None;
        }
        if target.ai_state == AiState::Idle {
            target.ai_state = AiState::Fighting;
            tracing::info!(
                npc_id = target_id, attacker = attacker_id,
                "NPC aggro: Idle -> Fighting"
            );
        }
        *target.threat_list.entry(attacker_id).or_insert(0.0) += threat_amount;
        true
    } else {
        false
    };

    if target_is_npc {
        enter_player_combat(space_mgr, attacker_id, target_id)
    } else {
        None
    }
}

/// Mirror an NPC→player threat-list addition onto the player's
/// `threatened_mobs` set. Idempotent — re-adding an already-tracked mob
/// is a no-op. Returns `Some(new_state_field)` only when this addition
/// is the first one and `BSF_IN_COMBAT` flips on; the caller broadcasts.
///
/// Reference: `python/cell/SGWPlayer.py:onAddedToThreatList` (944-953).
pub fn enter_player_combat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    player_id: u32,
    mob_id: u32,
) -> Option<u32> {
    let player = space_mgr.get_entity_mut(player_id)?;
    if !player.is_player {
        return None;
    }
    let was_empty = player.threatened_mobs.is_empty();
    if !player.threatened_mobs.insert(mob_id) {
        // Already in the set — no transition.
        return None;
    }
    if was_empty {
        let old = player.state_field;
        player.state_field |= super::state::BSF_IN_COMBAT;
        if player.state_field != old {
            tracing::debug!(
                player_id, mob_id, new_state = player.state_field,
                "enter_player_combat: BSF_InCombat set (first threatened mob)"
            );
            return Some(player.state_field);
        }
    }
    None
}

/// Inverse of [`enter_player_combat`]: drop a mob from the player's
/// `threatened_mobs` set. If the set becomes empty and `BSF_IN_COMBAT`
/// was set, clear it and return the new `state_field` for the caller
/// to broadcast.
///
/// Reference: `python/cell/SGWPlayer.py:onRemovedFromThreatList` (957-965).
pub fn exit_player_combat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    player_id: u32,
    mob_id: u32,
) -> Option<u32> {
    let player = space_mgr.get_entity_mut(player_id)?;
    if !player.is_player {
        return None;
    }
    if !player.threatened_mobs.remove(&mob_id) {
        // Wasn't in the set — nothing to do (idempotent).
        return None;
    }
    if player.threatened_mobs.is_empty() {
        let old = player.state_field;
        player.state_field &= !super::state::BSF_IN_COMBAT;
        if player.state_field != old {
            tracing::debug!(
                player_id, mob_id, new_state = player.state_field,
                "exit_player_combat: BSF_InCombat cleared (last threatened mob removed)"
            );
            return Some(player.state_field);
        }
    }
    None
}

/// Called when an NPC dies. Iterates the dying NPC's `threat_list` and
/// removes the NPC from each aggroed player's `threatened_mobs` set.
/// Returns `(player_id, new_state_field)` pairs for which `BSF_IN_COMBAT`
/// just cleared so the caller can broadcast `onStateFieldUpdate` to each
/// affected player's witnesses.
///
/// Does NOT clear the NPC's own `threat_list` — caller decides whether to
/// keep it for damage attribution (XP, loot tagging) or wipe it.
pub fn clear_dead_npc_from_all_player_threat(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    npc_id: u32,
) -> Vec<(u32, u32)> {
    // Snapshot the threat list — exit_player_combat takes &mut so we can't
    // hold a borrow on the NPC while iterating its keys.
    let aggroed_players: Vec<u32> = space_mgr
        .get_entity(npc_id)
        .map(|n| n.threat_list.keys().copied().collect())
        .unwrap_or_default();

    aggroed_players
        .into_iter()
        .filter_map(|player_id| {
            exit_player_combat(space_mgr, player_id, npc_id).map(|state| (player_id, state))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_default_ability_is_pistol_shot() {
        // 592 = Pistol Shot (ranged DD). Was previously 597 (Heal Focus, self-heal).
        assert_eq!(NPC_DEFAULT_ABILITY, 592);
    }

    #[test]
    fn leash_distance_is_50() {
        assert_eq!(LEASH_DISTANCE, 50.0);
    }

    use crate::cell::combat::state::BSF_IN_COMBAT;
    use crate::cell::space_manager::SpaceManager;

    fn make_test_space_mgr_with_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Create a player entity (mark is_player so the new threat helpers
        // recognize it; create_entity defaults to is_player=false).
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.get_entity_mut(1).unwrap().is_player = true;

        // Create an NPC entity
        mgr.spawn_npc(100, "Agnos", [15.0, 0.0, 15.0], [0.0; 3]).unwrap();

        mgr
    }

    fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.create_entity(id, "Agnos", [x, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.get_entity_mut(id).unwrap().is_player = true;
    }

    fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.spawn_npc(id, "Agnos", [x, 0.0, 15.0], [0.0; 3]).unwrap();
    }

    // ── generate_threat: NPC-side state preserved from pre-#92 behavior ────

    #[test]
    fn generate_threat_transitions_idle_to_fighting() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Idle);

        let _ = generate_threat(&mut mgr, 1, 100, 50.0);

        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.ai_state, AiState::Fighting);
        assert_eq!(npc.threat_list[&1], 50.0);
    }

    #[test]
    fn generate_threat_accumulates() {
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        let _ = generate_threat(&mut mgr, 1, 100, 30.0);

        assert_eq!(mgr.get_entity(100).unwrap().threat_list[&1], 80.0);
    }

    #[test]
    fn generate_threat_multiple_attackers() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_player(&mut mgr, 2, 20.0);

        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        let _ = generate_threat(&mut mgr, 2, 100, 100.0);

        let npc = mgr.get_entity(100).unwrap();
        assert_eq!(npc.threat_list.len(), 2);
        assert_eq!(npc.threat_list[&1], 50.0);
        assert_eq!(npc.threat_list[&2], 100.0);

        let top = npc.threat_list.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(&id, _)| id);
        assert_eq!(top, Some(2));
    }

    #[test]
    fn generate_threat_ignores_player_targets() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        // NPC entity 100 attacking player entity 1 — should be a no-op on
        // the player side and return None (no combat-enter broadcast).
        let result = generate_threat(&mut mgr, 100, 1, 50.0);
        assert_eq!(result, None);

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.ai_state, AiState::Idle);
        assert!(player.threat_list.is_empty());
    }

    #[test]
    fn generate_threat_stays_fighting_on_second_hit() {
        use cimmeria_entity::cell_entity::AiState;
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Fighting);

        let _ = generate_threat(&mut mgr, 1, 100, 25.0);
        assert_eq!(mgr.get_entity(100).unwrap().ai_state, AiState::Fighting);
    }

    // ── enter_player_combat primitives ─────────────────────────────────────

    #[test]
    fn enter_player_combat_first_mob_sets_bsf_and_returns_state() {
        let mut mgr = make_test_space_mgr_with_npc();

        let result = enter_player_combat(&mut mgr, 1, 100);

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.contains(&100));
        assert_eq!(player.threatened_mobs.len(), 1);
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0, "BSF_InCombat must be set");
        assert_eq!(result, Some(player.state_field));
    }

    #[test]
    fn enter_player_combat_subsequent_mob_no_broadcast() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        let _ = enter_player_combat(&mut mgr, 1, 100);
        // Second mob enters set; BSF was already set, so no broadcast needed.
        let result = enter_player_combat(&mut mgr, 1, 101);
        assert_eq!(result, None);

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.threatened_mobs.len(), 2);
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn enter_player_combat_idempotent_for_already_present_mob() {
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = enter_player_combat(&mut mgr, 1, 100);
        let result = enter_player_combat(&mut mgr, 1, 100);

        assert_eq!(result, None, "re-adding same mob must not re-broadcast");
        assert_eq!(mgr.get_entity(1).unwrap().threatened_mobs.len(), 1);
    }

    #[test]
    fn enter_player_combat_ignores_non_player_entity() {
        let mut mgr = make_test_space_mgr_with_npc();
        // Try to "enter combat" on the NPC (entity 100, is_player=false).
        let result = enter_player_combat(&mut mgr, 100, 1);
        assert_eq!(result, None);
        assert!(mgr.get_entity(100).unwrap().threatened_mobs.is_empty());
        assert_eq!(mgr.get_entity(100).unwrap().state_field & BSF_IN_COMBAT, 0);
    }

    // ── exit_player_combat primitives ──────────────────────────────────────

    #[test]
    fn exit_player_combat_only_mob_clears_bsf_and_returns_state() {
        let mut mgr = make_test_space_mgr_with_npc();
        let _ = enter_player_combat(&mut mgr, 1, 100);

        let result = exit_player_combat(&mut mgr, 1, 100);

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.is_empty());
        assert_eq!(player.state_field & BSF_IN_COMBAT, 0, "BSF_InCombat must be cleared");
        assert_eq!(result, Some(player.state_field));
    }

    #[test]
    fn exit_player_combat_one_of_many_no_broadcast() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);
        let _ = enter_player_combat(&mut mgr, 1, 100);
        let _ = enter_player_combat(&mut mgr, 1, 101);

        // Drop only one — set is still non-empty, no broadcast.
        let result = exit_player_combat(&mut mgr, 1, 100);
        assert_eq!(result, None);

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.threatened_mobs.len(), 1);
        assert!(player.threatened_mobs.contains(&101));
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0, "BSF must remain set");
    }

    #[test]
    fn exit_player_combat_not_in_set_is_idempotent() {
        let mut mgr = make_test_space_mgr_with_npc();
        // Player isn't in combat at all.
        let result = exit_player_combat(&mut mgr, 1, 100);
        assert_eq!(result, None);
        assert!(mgr.get_entity(1).unwrap().threatened_mobs.is_empty());
    }

    #[test]
    fn exit_player_combat_ignores_non_player_entity() {
        let mut mgr = make_test_space_mgr_with_npc();
        let result = exit_player_combat(&mut mgr, 100, 1);
        assert_eq!(result, None);
    }

    // ── generate_threat: returns combat-enter state on first add ──────────

    #[test]
    fn generate_threat_returns_state_on_player_first_aggro() {
        let mut mgr = make_test_space_mgr_with_npc();

        let result = generate_threat(&mut mgr, 1, 100, 50.0);
        assert!(result.is_some(), "first aggro must return new state for broadcast");

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.contains(&100));
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn generate_threat_returns_none_on_subsequent_hits() {
        let mut mgr = make_test_space_mgr_with_npc();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        // Second hit on same mob — already in set, no transition.
        let result = generate_threat(&mut mgr, 1, 100, 30.0);
        assert_eq!(result, None);
    }

    #[test]
    fn generate_threat_returns_none_when_attacker_is_npc() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        // NPC 101 attacking NPC 100 (e.g., pet) — NPC 100 enters Fighting,
        // but no player is involved so no combat-enter broadcast.
        let result = generate_threat(&mut mgr, 101, 100, 50.0);
        assert_eq!(result, None);

        // NPC 100's threat_list still got the NPC attacker so AI works.
        assert!(mgr.get_entity(100).unwrap().threat_list.contains_key(&101));
    }

    // ── clear_dead_npc_from_all_player_threat ──────────────────────────────

    #[test]
    fn dead_npc_clears_only_player_with_single_threat_source() {
        // 1 player aggroed by 1 mob; mob dies → BSF clears for the player.
        let mut mgr = make_test_space_mgr_with_npc();
        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        assert_ne!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);

        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert_eq!(to_broadcast.len(), 1);
        assert_eq!(to_broadcast[0].0, 1, "player_id must be in the broadcast list");

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.is_empty());
        assert_eq!(player.state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn dead_npc_does_not_clear_player_with_remaining_threat_sources() {
        // 1 player aggroed by 2 mobs; only 1 dies → BSF stays set.
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);
        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        let _ = generate_threat(&mut mgr, 1, 101, 50.0);
        assert_eq!(mgr.get_entity(1).unwrap().threatened_mobs.len(), 2);

        // NPC 100 dies — player still has 101 on the list.
        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert!(to_broadcast.is_empty(), "no broadcast: player still in combat");

        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.threatened_mobs.len(), 1);
        assert!(player.threatened_mobs.contains(&101));
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0, "BSF must remain set");

        // Now kill the second mob — BSF clears.
        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 101);
        assert_eq!(to_broadcast.len(), 1);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn dead_npc_clears_all_aggroed_players() {
        // 2 players each aggroed by the same 1 mob; mob dies → BSF clears
        // for both players. Mirrors the multi-attacker scenario the killer-
        // only fix would have left half-broken (non-killer stays in combat).
        let mut mgr = make_test_space_mgr_with_npc();
        add_player(&mut mgr, 2, 20.0);
        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        let _ = generate_threat(&mut mgr, 2, 100, 50.0);

        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert_eq!(to_broadcast.len(), 2);
        let updated_players: Vec<u32> = to_broadcast.iter().map(|(p, _)| *p).collect();
        assert!(updated_players.contains(&1));
        assert!(updated_players.contains(&2));

        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
        assert_eq!(mgr.get_entity(2).unwrap().state_field & BSF_IN_COMBAT, 0);
    }

    #[test]
    fn dead_npc_with_only_npc_threat_entries_emits_no_broadcasts() {
        // NPC 100's threat list contains only NPC 101 (e.g., pet vs mob).
        // When 100 dies, no player needs a broadcast.
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);
        let _ = generate_threat(&mut mgr, 101, 100, 50.0);

        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert!(to_broadcast.is_empty());
    }

    // ── End-to-end lifecycle ───────────────────────────────────────────────

    #[test]
    fn lifecycle_one_mob_aggro_kill_clears_combat() {
        let mut mgr = make_test_space_mgr_with_npc();

        // Aggro
        let entered = generate_threat(&mut mgr, 1, 100, 50.0);
        assert!(entered.is_some(), "first aggro should signal combat enter");
        assert_ne!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);

        // Kill
        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert_eq!(to_broadcast.len(), 1);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
        assert!(mgr.get_entity(1).unwrap().threatened_mobs.is_empty());
    }

    #[test]
    fn lifecycle_two_mobs_kill_one_then_other() {
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        // First aggro: combat enters.
        let entered = generate_threat(&mut mgr, 1, 100, 50.0);
        assert!(entered.is_some());

        // Second aggro on different mob: no transition, no broadcast.
        let second = generate_threat(&mut mgr, 1, 101, 30.0);
        assert_eq!(second, None);
        assert_eq!(mgr.get_entity(1).unwrap().threatened_mobs.len(), 2);

        // Kill mob 100: still in combat (mob 101 is alive).
        let after_first_kill = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert!(after_first_kill.is_empty());
        let player = mgr.get_entity(1).unwrap();
        assert_eq!(player.threatened_mobs.len(), 1);
        assert_ne!(player.state_field & BSF_IN_COMBAT, 0);

        // Kill mob 101: combat clears now.
        let after_second_kill = clear_dead_npc_from_all_player_threat(&mut mgr, 101);
        assert_eq!(after_second_kill.len(), 1);
        assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);
        assert!(mgr.get_entity(1).unwrap().threatened_mobs.is_empty());
    }
}
