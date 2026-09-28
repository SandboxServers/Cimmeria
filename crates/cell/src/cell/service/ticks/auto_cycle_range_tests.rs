//! Range tests for `auto_cycle_tick`'s pre-gate: a target the launch would
//! refuse on range is skipped silently, the loop stays armed, and nothing
//! reaches the wire. The fixture is `auto_cycle_tests::make_auto_cycle_mgr`
//! (player 1 at the origin, hostile NPC 50 at 5 m, ability 7).

use super::tests::{empty_engine, make_auto_cycle_mgr};
use super::*;

/// Assert the tick skipped: loop armed, no cooldown, nothing sent.
fn assert_skipped_silently(mgr: &SpaceManager, rx: &mut mpsc::Receiver<CellToBaseMsg>) {
    let p = mgr.get_entity(1).unwrap();
    assert!(p.abilities.auto_cycle, "the loop must stay armed");
    assert!(
        !p.abilities.is_on_cooldown(7),
        "the skip must not commit a fire (no cooldown started)"
    );
    assert!(
        rx.try_recv().is_err(),
        "the skip must be wire-silent: no onErrorCode, no onTimerUpdate"
    );
}

/// #1016: a target inside the ability's `min_range` is skipped like an
/// out-of-range one. Revert proof: without the minimum in the pre-gate the
/// tick calls `handle_use_ability`, which refuses with `onErrorCode` 42 on
/// every tick while the target stays close.
#[tokio::test]
async fn auto_cycle_tick_skips_a_target_inside_min_range_silently() {
    let mut mgr = make_auto_cycle_mgr();
    mgr.ability_defs.get_mut(&7).unwrap().min_range = 8.0;

    let (tx, mut rx) = mpsc::channel(64);
    auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

    assert_skipped_silently(&mgr, &mut rx);
}

/// #1017: a `UseWeaponRange` auto-attack with a 40 m weapon keeps firing at
/// a target 35 m away. Revert proof: with the flag ignored the pre-gate
/// uses the ability's 30 m and skips the target, so nothing fires.
#[tokio::test]
async fn auto_cycle_tick_uses_the_weapons_reach_for_a_weapon_range_ability() {
    use cimmeria_entity::abilities::{WeaponRanges, AF_USE_WEAPON_RANGE};
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_auto_cycle_mgr();
    {
        let def = mgr.ability_defs.get_mut(&7).unwrap();
        def.flags |= AF_USE_WEAPON_RANGE;
        def.is_ranged = true;
        def.max_range = 0.0;
    }
    mgr.weapon_ranges.insert(
        3287,
        WeaponRanges {
            min_ranged: 2.0,
            max_ranged: 40.0,
            min_melee: 0.0,
            max_melee: 2.0,
        },
    );
    {
        let p = mgr.get_entity_mut(1).unwrap();
        p.active_bandolier_slot = 0;
        p.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 77,
                item_id: 3287,
                clip_size: 30,
                default_ammo_type: 0,
                current_ammo: 30,
                cur_ammo_type: 0,
            },
        );
    }
    mgr.get_entity_mut(50).unwrap().position = cimmeria_common::Vector3::new(35.0, 0.0, 0.0);

    let (tx, _rx) = mpsc::channel(64);
    auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

    assert!(
        mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "the 40 m weapon reaches 35 m: the tick must re-fire"
    );
}
