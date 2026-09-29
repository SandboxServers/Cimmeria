//! The duel harm gate as the hit pipeline applies it (SS-D3, D-SS20):
//! whether a player-on-player hit may still land, and the source a
//! non-lethal duel clamp is logged under. Split out of `damage_apply/mod.rs`
//! to keep it under the file-size cap.

use cimmeria_cell_world::cell::duel;
use cimmeria_cell_world::cell::duel::DuelResources;

use super::super::super::combat;
use super::super::super::space_manager::SpaceManager;

/// `Some(reason)` when `attacker` and `target` are two different players
/// and the harm gate (`combat::player_may_attack`) no longer admits the
/// hit, for instance because an earlier hit of the same ability ended their
/// duel.
pub(super) fn player_hit_refusal(
    space_mgr: &SpaceManager,
    attacker: u32,
    target: u32,
) -> Option<&'static str> {
    if attacker == target {
        return None;
    }
    let (Some(a), Some(t)) = (space_mgr.get_entity(attacker), space_mgr.get_entity(target)) else {
        return None;
    };
    (a.is_player && t.is_player && !combat::player_may_attack(a, t, space_mgr.resources.duels()))
        .then_some("not_duel_opponent")
}

/// The `duel.lethal_clamped` source for a clamp in this function.
pub(super) fn clamp_source(path: &'static str, ability_id: i32) -> duel::ClampSource {
    duel::ClampSource {
        path,
        ability_id: Some(ability_id),
        effect_id: None,
    }
}
