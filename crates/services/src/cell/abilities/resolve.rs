//! Per-weapon ability resolution.
//!
//! The cell receives `useAbility` calls with a specific `ability_id`
//! from the client, but the **server-driven right-click on a hostile
//! NPC** path in [`super::super::cell_methods::player::interaction`]
//! needs to pick the ability itself. Before this module, that site
//! hardcoded `592` (Pistol Shot) regardless of the equipped weapon,
//! so a player wielding a P90 still fired Pistol Shot animations and
//! the SMG's per-weapon binding (ability 559 "Automatic Weapon Auto
//! Attack") was never used — bug landed in .
//!
//! Resolution flow:
//!
//! 1. Look up the player's **active bandolier slot** → get the weapon
//!    `item_id`.
//! 2. Query `space_mgr.item_event_set_abilities` for
//!    `(item_id, event_id)` where `event_id` is one of the
//!    `EVENT_ITEM_*` constants (RANGED=7, MELEE=6, USE=5).
//! 3. Return `Some(ability_id)` on hit, `None` on miss.
//!
//! These helpers do NOT pick a fallback — callers do. The right-click
//! hostile-NPC path uses `592 Pistol Shot` as its fallback so an unbound
//! item still produces a responsive click; other call sites pick what
//! makes sense for their context (e.g., `594 Strike` for a melee path).
//! Centralising the lookup means the call sites can rely on a single
//! `(item_id, event_id) → Option<i32>` shape without each one
//! re-implementing the lookup-plus-default pattern.
//!
//! Reference: [`docs/protocol/item-sequence-lookup.md`](../../../../../docs/protocol/item-sequence-lookup.md),
//! `deprecated/python/cell/SGWBeing.py:517-523` (`getItemSequence`).

use super::super::space_manager::SpaceManager;

/// Resolve the ability bound to `item_id` for `event_id`, or `None` if
/// no binding exists in `resources.items_event_sets`. Pure lookup; no
/// fallback decision — caller picks the default per call site.
pub fn ability_for_item(space_mgr: &SpaceManager, item_id: i32, event_id: i32) -> Option<i32> {
    space_mgr
        .item_event_set_abilities
        .get(&(item_id, event_id))
        .copied()
}

/// Resolve the ability that fires when the player wielding the active
/// bandolier slot triggers `event_id` (typically `EVENT_ITEM_RANGED`).
///
/// Returns `None` when:
/// - The entity has no active bandolier slot occupied (unarmed)
/// - The slotted item has no `items_event_sets` row for `event_id`
///
/// Callers MUST decide what to do with `None` — for the right-click
/// hostile-NPC path the previous behavior was "fire Pistol Shot 592",
/// which is preserved as an explicit fallback at the call site rather
/// than baked into this helper. See
/// `crates/services/src/cell/cell_methods/player/interaction.rs` for
/// the canonical pattern.
pub fn ability_for_active_weapon(
    space_mgr: &SpaceManager,
    entity_id: u32,
    event_id: i32,
) -> Option<i32> {
    let item_id = space_mgr.get_entity(entity_id).and_then(|e| {
        let slot = e.active_bandolier_slot;
        e.bandolier_items.get(&slot).map(|b| b.item_id)
    })?;
    ability_for_item(space_mgr, item_id, event_id)
}

/// True if `ability_id` is bound to the player's active bandolier weapon
/// via `items_event_sets` for any of the weapon-relevant event ids
/// (RANGED, MELEE, USE_ABILITY).
///
/// Used by [`super::use_ability::handle_use_ability`] as a fallback when
/// `entity.abilities.has_ability` returns false. Weapon-granted abilities
/// are resolved at fire time from `items_event_sets` rather than being
/// injected into `entity.abilities` on equip — `entity.abilities` holds
/// the player's *trained / known / archetype-starter* set, not their
/// currently-equipped weapon's abilities.
///
/// Without this fallback the use_ability gate rejects every weapon-driven
/// fire with `useAbility: entity does not have ability`. Live observation
/// 2026-06-02 (player 72.206.34.241): 25 consecutive useAbility(579)
/// rejections for a player holding the pistol that grants 579 via
/// `items_event_sets` row `(item_id=55, ability_id=579, event_id=7)`.
pub fn is_ability_granted_by_active_weapon(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> bool {
    let item_id = match space_mgr.get_entity(entity_id).and_then(|e| {
        let slot = e.active_bandolier_slot;
        e.bandolier_items.get(&slot).map(|b| b.item_id)
    }) {
        Some(id) => id,
        None => return false,
    };
    // Probe every weapon-relevant event id. A single weapon often grants
    // distinct ranged + melee abilities (the pistol has 579 for RANGED
    // and 708 for MELEE), and a player can legitimately fire any of them
    // while the same weapon is active. Iterating means callers don't have
    // to know which event_id their `ability_id` was bound under.
    use crate::cell::spawner::{EVENT_ITEM_MELEE, EVENT_ITEM_RANGED, EVENT_ITEM_USE_ABILITY};
    for event_id in [EVENT_ITEM_RANGED, EVENT_ITEM_MELEE, EVENT_ITEM_USE_ABILITY] {
        if space_mgr.item_event_set_abilities.get(&(item_id, event_id)) == Some(&ability_id) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::cell_entity::BandolierItem;

    fn mgr_with_one_player(item_id: i32, slot: i32) -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.active_bandolier_slot = slot;
            e.bandolier_items.insert(
                slot,
                BandolierItem {
                    item_id,
                    clip_size: 30,
                    default_ammo_type: 0,
                    current_ammo: 30,
                    cur_ammo_type: 0,
                },
            );
        }
        mgr
    }

    #[test]
    fn unbound_item_returns_none() {
        let mgr = mgr_with_one_player(55, 0);
        // No bindings seeded.
        assert_eq!(ability_for_item(&mgr, 55, 7), None);
        assert_eq!(ability_for_active_weapon(&mgr, 1, 7), None);
    }

    #[test]
    fn pistol_ranged_returns_pistol_auto_attack() {
        // Pin: pistol (item 55) ranged event (7) → ability 579
        // "Pistol Auto Attack" per seed data. This guards the bug from
        //  — pre-fix, any item resolved to ability 592.
        let mut mgr = mgr_with_one_player(55, 0);
        mgr.item_event_set_abilities.insert((55, 7), 579);
        assert_eq!(ability_for_item(&mgr, 55, 7), Some(579));
        assert_eq!(ability_for_active_weapon(&mgr, 1, 7), Some(579));
    }

    #[test]
    fn p90_ranged_returns_smg_auto_attack() {
        // Pin: P90/SMG (item 21) ranged → ability 559 "Automatic Weapon
        // Auto Attack". Different ability than pistol — this is the
        // specific bug shape the user observed: same Pistol Shot
        // animation for both weapons.
        let mut mgr = mgr_with_one_player(21, 0);
        mgr.item_event_set_abilities.insert((21, 7), 559);
        assert_eq!(ability_for_active_weapon(&mgr, 1, 7), Some(559));
    }

    #[test]
    fn unarmed_player_returns_none() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.active_bandolier_slot = 0;
            // No bandolier items.
        }
        assert_eq!(ability_for_active_weapon(&mgr, 1, 7), None);
    }

    #[test]
    fn melee_lookup_separate_from_ranged() {
        // Pistol has different melee (708) vs ranged (579) bindings —
        // ensure event_id discrimination works.
        let mut mgr = mgr_with_one_player(55, 0);
        mgr.item_event_set_abilities.insert((55, 7), 579);
        mgr.item_event_set_abilities.insert((55, 6), 708);
        assert_eq!(ability_for_active_weapon(&mgr, 1, 7), Some(579));
        assert_eq!(ability_for_active_weapon(&mgr, 1, 6), Some(708));
    }
}
