//! The duel's combat source: both duelists are in combat with each other
//! while the duel is engaged, and leave it together at the end.
//!
//! A player's `BSF_InCombat` is derived from their `threatened_mobs` set
//! (`cell-combat`'s `threat::player_combat`): the bit is on while the set is
//! non-empty. Player-on-player damage creates no threat
//! (`generate_threat` ignores player targets), so without this a duelist
//! would fight with the out-of-combat regen, holster and reload rules. The
//! duel puts the opponent's entity id in the set at the engage and takes it
//! out at the end, so the two sides always move together and the bit clears
//! only when nothing else (a mob) still has the player in combat.
//!
//! This is the same transition `enter_player_combat` / `exit_player_combat`
//! make for a mob, restated here because the duel lives in `cell-world` and
//! those live in `cell-combat`, which depends on this crate.

use cimmeria_wire::state_field::BSF_IN_COMBAT;

use crate::cell::space_manager::SpaceManager;

/// Add `opponent_eid` as a combat source of `eid`. Returns the new
/// `state_field` when `BSF_InCombat` turned on, for the caller to send.
pub(super) fn enter(mgr: &mut SpaceManager, eid: u32, opponent_eid: u32) -> Option<u32> {
    let player = mgr.get_entity_mut(eid)?;
    let was_empty = player.threatened_mobs.is_empty();
    if !player.threatened_mobs.insert(opponent_eid) {
        return None;
    }
    // Same as a mob's aggro: re-entering combat cancels a pending holster.
    player.combat_exit_at = None;
    player.holster_animation_complete_at = None;
    if !was_empty {
        return None;
    }
    let old = player.state_field;
    player.state_field |= BSF_IN_COMBAT;
    let _ = player.sync_holster_to_combat(true);
    (player.state_field != old).then_some(player.state_field)
}

/// Remove `opponent_eid` as a combat source of `eid`. Returns the new
/// `state_field` when `BSF_InCombat` turned off (nothing else holds the
/// player in combat). The holster waits for the out-of-combat timer, as it
/// does after a mob fight.
pub(super) fn exit(mgr: &mut SpaceManager, eid: u32, opponent_eid: u32) -> Option<u32> {
    let player = mgr.get_entity_mut(eid)?;
    if !player.threatened_mobs.remove(&opponent_eid) || !player.threatened_mobs.is_empty() {
        return None;
    }
    let old = player.state_field;
    player.state_field &= !BSF_IN_COMBAT;
    if player.state_field == old {
        return None;
    }
    player.combat_exit_at = Some(std::time::Instant::now());
    Some(player.state_field)
}
