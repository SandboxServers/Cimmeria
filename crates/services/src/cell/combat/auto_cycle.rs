//! Server-side auto-cycle (auto-fire) lifecycle primitives.
//!
//! The cell method 83 `setAutoCycle(enabled)` arms a flag, but the actual
//! re-fire loop is server-driven: every cooldown expiry the
//! [`crate::cell::service::ticks::auto_cycle_tick`] driver scans for armed
//! players with stashed `(ability_id, target_id)` pairs and re-invokes
//! [`crate::cell::abilities::handle_use_ability`].
//!
//! This module owns the three transition primitives the loop hinges on:
//!
//! - [`arm_auto_cycle`] — stash the ability/target pair + set `BSF_AUTO_CYCLING`.
//!   Called at first-commit time when `auto_cycle == true`.
//! - [`clear_auto_cycle`] — drop the stash, clear the bit, return the new
//!   `state_field` if it transitioned. Called on every stop path: explicit
//!   `setAutoCycle(0)`, manual fire of a different ability,
//!   `AF_DEACTIVATE_AUTO_CYCLE` flag, target death, or out-of-range/leashed.
//! - [`clear_auto_cycle_for_target`] — sweep all players currently auto-cycling
//!   at a given (now-dead/despawned) target id. Called from the death-transition
//!   broadcast site.
//!
//! Every transition mirrors the [`super::threat::enter_player_combat`] /
//! [`super::threat::exit_player_combat`] shape: mutate the entity, return
//! `Some(new_state_field)` only when the BSF bit actually flipped, and let
//! the caller decide whether to broadcast (because the broadcast site holds
//! the `mpsc::Sender` and has the routing rules).

use crate::cell::space_manager::SpaceManager;

use super::state::BSF_AUTO_CYCLING;

/// Arm the auto-cycle loop on `player_id` with the given ability + target.
///
/// Stashes both ids on the `AbilityManager` and sets `BSF_AUTO_CYCLING` on
/// the entity's `state_field`. Returns `Some(new_state_field)` only when the
/// bit just transitioned from `0 → 1` so the caller knows to broadcast
/// `onStateFieldUpdate`. Re-arming on an already-cycling player updates the
/// stash but returns `None` (the bit was already set; no client refresh
/// needed).
///
/// No-op (returns `None`) for non-player entities or missing entity ids.
#[must_use]
pub fn arm_auto_cycle(
    space_mgr: &mut SpaceManager,
    player_id: u32,
    ability_id: i32,
    target_id: i32,
) -> Option<u32> {
    let player = space_mgr.get_entity_mut(player_id)?;
    if !player.is_player {
        return None;
    }
    player.abilities.auto_cycle_ability_id = Some(ability_id);
    player.abilities.auto_cycle_target_id = Some(target_id);
    if player.set_state_flag(BSF_AUTO_CYCLING) {
        Some(player.state_field)
    } else {
        None
    }
}

/// Clear the auto-cycle loop on `player_id`.
///
/// Drops the stashed ability/target ids, flips `auto_cycle` to `false`, and
/// clears `BSF_AUTO_CYCLING`. Returns `Some(new_state_field)` only when the
/// bit just transitioned from `1 → 0` so the caller knows to broadcast
/// `onStateFieldUpdate`. Idempotent — calling on an already-cleared player
/// is a no-op and returns `None`.
///
/// No-op (returns `None`) for non-player entities or missing entity ids.
#[must_use]
pub fn clear_auto_cycle(space_mgr: &mut SpaceManager, player_id: u32) -> Option<u32> {
    let player = space_mgr.get_entity_mut(player_id)?;
    if !player.is_player {
        return None;
    }
    player.abilities.auto_cycle = false;
    player.abilities.auto_cycle_ability_id = None;
    player.abilities.auto_cycle_target_id = None;
    if player.unset_state_flag(BSF_AUTO_CYCLING) {
        Some(player.state_field)
    } else {
        None
    }
}

/// Sweep all players whose `auto_cycle_target_id` matches the given (just-dead
/// or despawned) entity and clear their loops.
///
/// Returns `(player_id, new_state_field)` pairs for which `BSF_AUTO_CYCLING`
/// just cleared so the caller can broadcast `onStateFieldUpdate` to each one.
/// Mirrors [`super::threat::clear_dead_npc_from_all_player_threat`] in shape
/// and purpose: the death-transition burst should clear the loop for *every*
/// player currently auto-cycling at the dying target, not just the killer.
pub fn clear_auto_cycle_for_target(
    space_mgr: &mut SpaceManager,
    target_id: u32,
) -> Vec<(u32, u32)> {
    let target_i32 = target_id as i32;
    let cycling: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.abilities.auto_cycle && e.abilities.auto_cycle_target_id == Some(target_i32)
            })
        })
        .collect();

    cycling
        .into_iter()
        .filter_map(|pid| clear_auto_cycle(space_mgr, pid).map(|s| (pid, s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr
    }

    fn make_player(mgr: &mut SpaceManager, id: u32) {
        mgr.create_entity(id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(id) {
            p.is_player = true;
            p.player_id = Some(100 + id as i32);
            p.abilities.auto_cycle = true; // armed via setAutoCycle(1) earlier
        }
        // `clear_auto_cycle_for_target` iterates `all_player_entity_ids()`,
        // which only returns CONNECTED players (the `space.players` set is
        // populated by `connect_entity`). Skipping this leaves the sweep
        // blind to the test fixture — pin it here so future tests don't
        // hit the same trap.
        mgr.connect_entity(id);
    }

    /// First arm flips the BSF bit and returns the new state for broadcast.
    /// Pin so a refactor that flips the bit unconditionally (or never)
    /// regresses the client button-highlight handshake.
    #[test]
    fn arm_first_time_returns_new_state_with_bsf_set() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);

        let result = arm_auto_cycle(&mut mgr, 1, 592, 100);
        assert!(result.is_some(), "first arm must return new state");

        let p = mgr.get_entity(1).unwrap();
        assert_eq!(p.abilities.auto_cycle_ability_id, Some(592));
        assert_eq!(p.abilities.auto_cycle_target_id, Some(100));
        assert_ne!(p.state_field & BSF_AUTO_CYCLING, 0);
        assert_eq!(result, Some(p.state_field));
    }

    /// Re-arming on an already-cycling player updates the stash but does
    /// NOT re-broadcast — the bit didn't transition, so the client doesn't
    /// need another `onStateFieldUpdate`. Idempotency guard.
    #[test]
    fn arm_idempotent_when_already_set() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);
        let _ = arm_auto_cycle(&mut mgr, 1, 592, 100);

        // Re-arm with a different target/ability — stash updates, no broadcast.
        let result = arm_auto_cycle(&mut mgr, 1, 593, 101);
        assert_eq!(result, None, "second arm must NOT signal broadcast");

        let p = mgr.get_entity(1).unwrap();
        assert_eq!(
            p.abilities.auto_cycle_ability_id,
            Some(593),
            "stash must update even when bit doesn't flip",
        );
        assert_eq!(p.abilities.auto_cycle_target_id, Some(101));
    }

    /// Clear after arm flips the bit back off and returns the new state.
    /// The stash is wiped — the next arm starts from scratch.
    #[test]
    fn clear_after_arm_returns_new_state_with_bsf_unset() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);
        let _ = arm_auto_cycle(&mut mgr, 1, 592, 100);

        let result = clear_auto_cycle(&mut mgr, 1);
        assert!(result.is_some(), "clear after arm must return new state");

        let p = mgr.get_entity(1).unwrap();
        assert!(!p.abilities.auto_cycle);
        assert_eq!(p.abilities.auto_cycle_ability_id, None);
        assert_eq!(p.abilities.auto_cycle_target_id, None);
        assert_eq!(p.state_field & BSF_AUTO_CYCLING, 0);
        assert_eq!(result, Some(p.state_field));
    }

    /// Clear on a never-armed player is a no-op — no broadcast, no state
    /// change, no panic. Defensive guard for the death-sweep helper which
    /// can call clear on any player that happened to be in the same space.
    #[test]
    fn clear_idempotent_when_not_set() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.auto_cycle = false; // un-arm so it's really not set
        }
        let result = clear_auto_cycle(&mut mgr, 1);
        assert_eq!(result, None, "clear on un-armed player must NOT broadcast");
    }

    /// `clear_auto_cycle_for_target` sweeps every player auto-cycling at
    /// the given target and returns them all for broadcast. Pins the
    /// multi-attacker invariant: if 3 players are all auto-firing at the
    /// same NPC and it dies, all 3 must get their `BSF_AUTO_CYCLING`
    /// cleared in the same death burst.
    #[test]
    fn target_sweep_clears_every_player_cycling_at_dying_target() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);
        make_player(&mut mgr, 2);
        make_player(&mut mgr, 3);

        let _ = arm_auto_cycle(&mut mgr, 1, 592, 999);
        let _ = arm_auto_cycle(&mut mgr, 2, 592, 999);
        let _ = arm_auto_cycle(&mut mgr, 3, 592, 999);

        let broadcasts = clear_auto_cycle_for_target(&mut mgr, 999);
        assert_eq!(broadcasts.len(), 3, "all 3 players must be cleared");

        for &(pid, _state) in &broadcasts {
            let p = mgr.get_entity(pid).unwrap();
            assert!(!p.abilities.auto_cycle, "player {pid} must be un-armed");
            assert_eq!(p.state_field & BSF_AUTO_CYCLING, 0);
        }
    }

    /// Target sweep ignores players auto-cycling at a different target.
    /// Pin: the sweep must filter by `auto_cycle_target_id`, not nuke
    /// every armed player.
    #[test]
    fn target_sweep_ignores_players_cycling_at_other_targets() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1);
        make_player(&mut mgr, 2);

        let _ = arm_auto_cycle(&mut mgr, 1, 592, 999);
        let _ = arm_auto_cycle(&mut mgr, 2, 592, 888);

        let broadcasts = clear_auto_cycle_for_target(&mut mgr, 999);
        assert_eq!(broadcasts.len(), 1, "only player 1 must be cleared");
        assert_eq!(broadcasts[0].0, 1);

        // Player 2 must remain armed at its own target.
        let p2 = mgr.get_entity(2).unwrap();
        assert!(p2.abilities.auto_cycle);
        assert_eq!(p2.abilities.auto_cycle_target_id, Some(888));
        assert_ne!(p2.state_field & BSF_AUTO_CYCLING, 0);
    }

    /// Non-player entities can't auto-cycle. Defensive guard so a stray
    /// call against an NPC id (e.g., from a debug command) is a no-op
    /// rather than mutating NPC state.
    #[test]
    fn arm_and_clear_no_op_for_non_player() {
        let mut mgr = make_mgr();
        mgr.spawn_npc(50, "Castle", [0.0; 3], [0.0; 3]).unwrap();

        assert_eq!(arm_auto_cycle(&mut mgr, 50, 592, 1), None);
        assert_eq!(clear_auto_cycle(&mut mgr, 50), None);

        let npc = mgr.get_entity(50).unwrap();
        assert_eq!(npc.state_field & BSF_AUTO_CYCLING, 0);
        assert!(npc.abilities.auto_cycle_ability_id.is_none());
    }
}
