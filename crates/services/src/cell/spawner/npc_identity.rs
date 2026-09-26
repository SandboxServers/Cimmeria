//! What an NPC fights with, as telemetry fields: its ability set, the event
//! set each ability animates through, and the weapon it is drawn holding.
//!
//! Shared by the `spawner.npc_behaviour` spawn row and the `.bug`
//! `playtest.bookmark.entity` row, so "why does this guard fire a pistol shot
//! while holding an SMG" is answered from one row instead of a seed read
//! (NA44, handoff §2 and §25).

use cimmeria_entity::cell_entity::CellEntity;

use crate::cell::space_manager::SpaceManager;

/// The entity's known ability ids, sorted so rows compare across spawns.
pub(crate) fn sorted_ability_ids(e: &CellEntity) -> Vec<i32> {
    let mut ids = e.abilities.known_ability_ids();
    ids.sort_unstable();
    ids
}

/// The `event_set_id` each ability in `ability_ids` animates through, in the
/// same order. `0` means the ability has no event set (NULL in the seed) or
/// its definition is not loaded; either way the attack sends no `onSequence`
/// and the client shows damage with no fire animation.
pub(crate) fn ability_event_set_ids(space_mgr: &SpaceManager, ability_ids: &[i32]) -> Vec<i32> {
    ability_ids
        .iter()
        .map(|id| {
            space_mgr
                .ability_defs
                .get(id)
                .and_then(|def| def.event_set_id)
                .unwrap_or(0)
        })
        .collect()
}

/// The weapon mesh the entity is drawn holding. Players carry it in
/// `weapon_visual` (the active bandolier slot). NPCs never set that field:
/// their weapon is one of the template `components` (`WP-Human.WP_SMG_1A`),
/// so fall back to the first `WP` component. Empty when it holds nothing.
pub(crate) fn weapon_visual(e: &CellEntity) -> String {
    e.weapon_visual
        .clone()
        .or_else(|| {
            e.components
                .iter()
                .find(|c| is_weapon_component(c))
                .cloned()
        })
        .unwrap_or_default()
}

/// `WP-Human.WP_SMG_1A`, `WP-Jaffa.WP_Staff_Plasma_4A`: the package or the
/// object name starts with `WP`.
fn is_weapon_component(c: &str) -> bool {
    c.starts_with("WP-") || c.starts_with("WP_") || c.contains(".WP_")
}

#[cfg(test)]
mod tests {
    use super::is_weapon_component;

    #[test]
    fn weapon_components_are_recognised_and_armour_is_not() {
        assert!(is_weapon_component("WP-Human.WP_SMG_1A"));
        assert!(is_weapon_component("WP-Jaffa.WP_Staff_Plasma_4A"));
        assert!(is_weapon_component("WP-Goauld.WP_Ribbon_Elec_1A"));
        assert!(!is_weapon_component("AR_H_Ablative.AR_HM_AT3_AT300"));
        assert!(!is_weapon_component("NPC_Human.NPC_HM_Marsh_Head_BC"));
    }
}
