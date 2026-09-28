//! #1017: an ability flagged `UseWeaponRange` (4) is range-checked against
//! the equipped weapon's reach, not its own `max_range` (or the 30 m
//! default). The client's range getters and the 2009 Python reference
//! (`AbilityManager.py:555`) both swap in the weapon's `{min, max}` pair.

use cimmeria_entity::abilities::{WeaponRanges, AF_USE_WEAPON_RANGE};
use cimmeria_entity::cell_entity::BandolierItem;

use super::warmup::{after_warmup, effect_results, warmup_mgr, WARMUP_ABILITY};
use super::*;
use crate::cell::abilities::resolve_warmups;
use crate::test_support::NoContentEvents;

/// SR1 .50-Cal Rifle: seeded 2-40 m ranged, 0-2 m melee.
const RIFLE_ITEM: i32 = 3287;
const RIFLE: WeaponRanges = WeaponRanges {
    min_ranged: 2.0,
    max_ranged: 40.0,
    min_melee: 0.0,
    max_melee: 2.0,
};

/// Put the rifle in `entity_id`'s active bandolier slot and its reach in the
/// weapon table.
fn equip_rifle(mgr: &mut SpaceManager, entity_id: u32) {
    mgr.weapon_ranges.insert(RIFLE_ITEM, RIFLE);
    let p = mgr.get_entity_mut(entity_id).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 77,
            item_id: RIFLE_ITEM,
            clip_size: 30,
            default_ammo_type: 0,
            current_ammo: 30,
            cur_ammo_type: 0,
        },
    );
}

/// A ranged ability flagged `UseWeaponRange`, `max_range` 0 (as every
/// seeded weapon attack is): 30 m on its own.
fn weapon_range_ability() -> AbilityDef {
    let mut def = make_ability(581, 0, 0);
    def.flags = AF_USE_WEAPON_RANGE;
    def.is_ranged = true;
    def
}

/// Fire `def` at a hostile `distance` metres away, optionally with the rifle
/// equipped. Returns whether the cast drew the out-of-range refusal (42).
async fn refused(def: &AbilityDef, distance: f32, rifle: bool) -> bool {
    let mut mgr = make_mgr();
    make_player(&mut mgr, 1, [0.0; 3]);
    if rifle {
        equip_rifle(&mut mgr, 1);
    }
    mgr.create_entity(2, "Castle_CellBlock", [distance, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(2).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    mgr.get_entity_mut(1)
        .unwrap()
        .abilities
        .add_ability(def.ability_id);
    mgr.ability_defs.insert(def.ability_id, def.clone());
    let (tx, mut rx) = mpsc::channel(64);
    handle_use_ability(1, def.ability_id, 2, &tx, &mut mgr).await;
    drain(&mut rx).iter().any(|m| {
        matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, method_index, args }
            if *method_index == method_idx::ON_ERROR_CODE
                && args.len() == 7
                && u16::from_le_bytes([args[5], args[6]]) == 42)
    })
}

/// Revert proof: with the flag ignored the ability's own 30 m default
/// refuses the target at 35 m.
#[tokio::test]
async fn weapon_range_ability_with_a_40m_weapon_reaches_35m() {
    assert!(
        !refused(&weapon_range_ability(), 35.0, true).await,
        "the rifle reaches 40 m: a target at 35 m is in range"
    );
}

#[tokio::test]
async fn weapon_range_ability_with_a_40m_weapon_refuses_45m() {
    assert!(
        refused(&weapon_range_ability(), 45.0, true).await,
        "the rifle reaches 40 m: a target at 45 m is out of range"
    );
}

/// The weapon's minimum applies too (the rifle's ranged pair is 2-40 m).
#[tokio::test]
async fn weapon_range_ability_refuses_inside_the_weapons_minimum() {
    assert!(refused(&weapon_range_ability(), 1.0, true).await);
}

/// No weapon equipped: the ability's own range (30 m default) applies.
#[tokio::test]
async fn weapon_range_ability_without_a_weapon_uses_its_own_range() {
    assert!(!refused(&weapon_range_ability(), 29.0, false).await);
    assert!(refused(&weapon_range_ability(), 35.0, false).await);
}

/// An unflagged ability ignores the rifle.
#[tokio::test]
async fn unflagged_ability_ignores_the_weapon() {
    let mut def = weapon_range_ability();
    def.flags = 0;
    assert!(refused(&def, 35.0, true).await);
}

/// The warmup fire-time re-check uses the weapon's reach too. The target
/// walks from 3 m to 35 m during the warmup. Revert proof: with the flag
/// ignored the fire is refused at 35 m (30 m default) and nothing lands.
#[tokio::test]
async fn warmup_fire_uses_the_weapons_reach() {
    let mut mgr = warmup_mgr();
    equip_rifle(&mut mgr, 1);
    {
        let def = mgr.ability_defs.get_mut(&WARMUP_ABILITY).unwrap();
        def.flags |= AF_USE_WEAPON_RANGE;
        def.max_range = 0.0;
    }
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(1, WARMUP_ABILITY, 2, &tx, &mut mgr).await);
    drain(&mut rx);

    mgr.get_entity_mut(2).unwrap().position.x = 35.0;
    resolve_warmups(after_warmup(), &tx, &mut mgr, &NoContentEvents).await;
    let msgs = drain(&mut rx);
    assert_eq!(
        effect_results(&msgs, 1),
        1,
        "the 40 m rifle reaches 35 m, so the warmup fires; got {msgs:?}"
    );
}
