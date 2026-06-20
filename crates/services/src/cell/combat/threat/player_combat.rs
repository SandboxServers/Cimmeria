//! Player-side combat state: `threatened_mobs` set and `BSF_IN_COMBAT`.
//!
//! Mirrors NPC→player threat additions onto the player's `threatened_mobs`
//! set, flipping `BSF_IN_COMBAT` on the first add and off on the last
//! removal. Also handles the OOC holster-defer timer and the dead-NPC
//! sweep that drops a dying mob from every aggroed player.
//!
//! Reference: `python/cell/SGWPlayer.py` (onAddedToThreatList /
//! onRemovedFromThreatList).

/// Mirror an NPC→player threat-list addition onto the player's
/// `threatened_mobs` set. Idempotent — re-adding an already-tracked mob
/// is a no-op. Returns `Some(new_state_field)` only when this addition
/// is the first one and `BSF_IN_COMBAT` flips on; the caller broadcasts.
///
/// Reference: `python/cell/SGWPlayer.py:onAddedToThreatList` (944-953).
///
/// The span is at `trace` level so it doesn't generate one SigNoz event
/// per call (per-damage-tick this gets called many times per actual aggro
/// transition). The `info!` event below — gated on the actual
/// `was_empty && state changed` transition — is the queryable signal
/// for "this is the moment of aggro". See issue #408.
#[tracing::instrument(name = "threat.enter_combat", level = "trace", skip_all)]
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
    // Re-entering combat within the OOC grace window cancels both
    // phases of the deferred holster — `combat_exit_at` (Phase 1
    // pending) and `holster_animation_complete_at` (Phase 2 pending,
    // mid-animation). Re-aggro mid-animation must stop the Phase 2
    // appearance change from yanking the mesh away after the player's
    // already drawn for combat again.
    player.combat_exit_at = None;
    player.holster_animation_complete_at = None;
    if was_empty {
        let old = player.state_field;
        player.state_field |= super::super::state::BSF_IN_COMBAT;
        if player.state_field != old {
            // Draw the weapon: holster follows BSF_InCombat. The bool
            // return is observable to the caller via the entity itself
            // (the rebroadcast decision lives at the cell→base message
            // dispatch site, which already has `space_mgr` in hand).
            let _ = player.sync_holster_to_combat(true);
            // Promoted from debug! to info! with stable `target: "threat"`
            // so SigNoz `groupBy event` counts real combat-enter
            // transitions (one per actual aggro, not one per damage tick).
            // See issue #408.
            tracing::info!(
                target: "threat",
                event = "enter_combat",
                player_id,
                mob_id,
                new_state = player.state_field,
                weapon_holstered = player.weapon_holstered,
                "enter_player_combat: BSF_InCombat set (first threatened mob); weapon drawn"
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
#[tracing::instrument(
    name = "threat.exit_combat",
    level = "trace",
    skip_all,
    fields(player_id, mob_id)
)]
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
        player.state_field &= !super::super::state::BSF_IN_COMBAT;
        if player.state_field != old {
            // Don't holster yet — stamp the OOC timer instead. The
            // `holster_timer_tick` re-holsters + rebroadcasts once
            // `OOC_HOLSTER_DELAY` elapses; re-aggro before then cancels
            // (cleared in `enter_player_combat`). BSF_InCombat clears
            // immediately on the wire so HUD/cursor flips don't lag.
            player.combat_exit_at = Some(std::time::Instant::now());
            tracing::info!(
                target: "threat",
                event = "exit_combat",
                player_id,
                mob_id,
                new_state = player.state_field,
                weapon_holstered = player.weapon_holstered,
                "exit_player_combat: BSF_InCombat cleared; OOC holster timer armed"
            );
            return Some(player.state_field);
        }
    }
    None
}

/// Called when an NPC dies. Iterates the dying NPC's `threat_list` and
/// removes the NPC from each aggroed player's `threatened_mobs` set.
/// Returns `(player_id, new_state_field)` pairs for which `BSF_IN_COMBAT`
/// just cleared so the caller can send `onStateFieldUpdate` to each
/// affected player (via `send_entity_method`, which for player entities
/// routes to that player's own client — not their AoI witnesses).
///
/// Does NOT clear the NPC's own `threat_list` — caller decides whether to
/// keep it for damage attribution (XP, loot tagging) or wipe it.
#[tracing::instrument(
    name = "threat.clear_dead_npc",
    level = "trace",
    skip_all,
    fields(npc_id)
)]
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
    use crate::cell::combat::state::BSF_IN_COMBAT;
    use crate::cell::combat::threat::aggro::generate_threat;
    use crate::cell::space_manager::SpaceManager;

    fn make_test_space_mgr_with_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Create a player entity (mark is_player so the new threat helpers
        // recognize it; create_entity defaults to is_player=false).
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(1).unwrap().is_player = true;

        // Create an NPC entity
        mgr.spawn_npc(100, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();

        mgr
    }

    fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.create_entity(id, "Agnos", [x, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(id).unwrap().is_player = true;
    }

    fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
        mgr.spawn_npc(id, "Agnos", [x, 0.0, 15.0], [0.0; 3])
            .unwrap();
    }

    // ── enter_player_combat primitives ─────────────────────────────────────

    #[test]
    fn enter_player_combat_first_mob_sets_bsf_and_returns_state() {
        let mut mgr = make_test_space_mgr_with_npc();

        let result = enter_player_combat(&mut mgr, 1, 100);

        let player = mgr.get_entity(1).unwrap();
        assert!(player.threatened_mobs.contains(&100));
        assert_eq!(player.threatened_mobs.len(), 1);
        assert_ne!(
            player.state_field & BSF_IN_COMBAT,
            0,
            "BSF_InCombat must be set"
        );
        assert_eq!(result, Some(player.state_field));
    }

    #[test]
    fn enter_player_combat_draws_weapon_when_bsf_flips() {
        // Phase 2 invariant: when BSF_InCombat goes off → on,
        // `weapon_holstered` flips true → false in the same call so the
        // dispatch site can request a `BeingAppearance` rebroadcast
        // immediately after the `onStateFieldUpdate`. The bug shape this
        // catches: someone refactors `enter_player_combat` to no longer
        // touch `weapon_holstered`, players spawn-holstered forever, no
        // weapon visible in combat (matches the symptom that drove
        // Phase 2 in the first place).
        let mut mgr = make_test_space_mgr_with_npc();
        // Pre-condition: spawn-holstered default.
        assert!(
            mgr.get_entity(1).unwrap().weapon_holstered,
            "fixture invariant: players start weapon-holstered"
        );

        let _ = enter_player_combat(&mut mgr, 1, 100);

        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "BSF_InCombat set ⇒ weapon must be drawn (holstered=false)",
        );
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
        assert_eq!(
            player.state_field & BSF_IN_COMBAT,
            0,
            "BSF_InCombat must be cleared"
        );
        assert_eq!(result, Some(player.state_field));
    }

    #[test]
    fn exit_player_combat_defers_holster_via_combat_exit_at() {
        // Phase 3: re-holstering is deferred to the OOC
        // grace tick. `exit_player_combat` stamps `combat_exit_at` and
        // leaves the weapon drawn — chaining mobs (kill A, aggro B
        // within the grace window) needs to skip the visible flicker.
        // The actual re-holster happens in `holster_timer_tick`.
        //
        // Bug shape this catches: a refactor that goes back to flipping
        // `weapon_holstered` here would reintroduce the flicker.
        let mut mgr = make_test_space_mgr_with_npc();
        let _ = enter_player_combat(&mut mgr, 1, 100);
        assert!(
            !mgr.get_entity(1).unwrap().weapon_holstered,
            "fixture invariant: entering combat must have drawn the weapon"
        );

        let _ = exit_player_combat(&mut mgr, 1, 100);

        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "weapon must stay drawn — re-holster is deferred to holster_timer_tick",
        );
        assert!(
            player.combat_exit_at.is_some(),
            "combat_exit_at must be stamped so the tick scan can pick this player up",
        );
    }

    #[test]
    fn enter_player_combat_cancels_pending_holster() {
        // Phase 3: re-aggro inside the OOC grace window
        // must wipe the pending holster, otherwise the tick scan
        // would still fire and holster a player who's now back in
        // combat. Bug shape: a refactor moves the
        // `combat_exit_at = None` line into the `was_empty` branch,
        // leaving the cancel logic dead for the 2nd-mob case.
        let mut mgr = make_test_space_mgr_with_npc();
        add_npc(&mut mgr, 101, 25.0);

        let _ = enter_player_combat(&mut mgr, 1, 100);
        let _ = exit_player_combat(&mut mgr, 1, 100);
        assert!(
            mgr.get_entity(1).unwrap().combat_exit_at.is_some(),
            "fixture invariant: exit must have stamped the timer"
        );

        // Re-aggro on a different mob inside the grace window.
        let _ = enter_player_combat(&mut mgr, 1, 101);

        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.combat_exit_at.is_none(),
            "re-aggro within the grace window must cancel the pending holster",
        );
        assert!(
            !player.weapon_holstered,
            "and the weapon must still be drawn",
        );
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

    // ── clear_dead_npc_from_all_player_threat ──────────────────────────────

    #[test]
    fn dead_npc_clears_only_player_with_single_threat_source() {
        // 1 player aggroed by 1 mob; mob dies → BSF clears for the player.
        let mut mgr = make_test_space_mgr_with_npc();
        let _ = generate_threat(&mut mgr, 1, 100, 50.0);
        assert_ne!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);

        let to_broadcast = clear_dead_npc_from_all_player_threat(&mut mgr, 100);
        assert_eq!(to_broadcast.len(), 1);
        assert_eq!(
            to_broadcast[0].0, 1,
            "player_id must be in the broadcast list"
        );

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
        assert!(
            to_broadcast.is_empty(),
            "no broadcast: player still in combat"
        );

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
}
