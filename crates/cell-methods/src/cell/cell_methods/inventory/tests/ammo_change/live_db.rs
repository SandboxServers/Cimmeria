//! Live-DB guards: `requestAmmoChange` refuses exactly what the seeded
//! `resources.items.ammo_types` column refuses, with the `WeaponDef` cache
//! built by the cell's own startup loader (`load_item_defs`).
//!
//! AM-F's widening (D-AM10) gives the Standard Pistol and SMG families all
//! five bullet specials; weapons outside those families keep their old
//! list. The two tests below pick one of each from the loaded cache rather
//! than by id, so a reseed that renumbers items doesn't break them.

use cimmeria_entity::ammo_type::{
    BULLET_ARMOR_PIERCING, BULLET_EMP, BULLET_EXPLOSIVE, BULLET_HOLLOW_POINT, BULLET_INCENDIARY,
    DART_POISON,
};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

use super::{feedback_text, only_feedback_line, player_with_weapon, request, send, ENTITY};

const BULLET_SPECIALS: [i32; 5] = [
    BULLET_ARMOR_PIERCING,
    BULLET_HOLLOW_POINT,
    BULLET_INCENDIARY,
    BULLET_EMP,
    BULLET_EXPLOSIVE,
];
/// Sentinel instance id for the fixture slot (never written to the DB).
const LIVE_INSTANCE: i32 = 0x7000_3031;

/// A widened weapon accepts every bullet special, and refuses a dart type
/// its column does not list, with the feedback line.
#[tokio::test]
async fn live_db_request_ammo_change_follows_a_widened_weapons_column() {
    let pool = require_db_or_skip!();
    let defs = spawner::load_item_defs(&pool)
        .await
        .expect("load_item_defs");
    let (&design, def) = defs
        .iter()
        .filter(|(_, d)| {
            BULLET_SPECIALS
                .iter()
                .all(|t| d.allowed_ammo_types.contains(t))
        })
        .min_by_key(|(id, _)| **id)
        .expect("AM-F widened at least one weapon to every bullet special");
    assert!(
        !def.allowed_ammo_types.contains(&DART_POISON) && def.default_ammo_type != DART_POISON,
        "a bullet weapon must not list a dart type"
    );

    for special in BULLET_SPECIALS {
        let mut mgr = player_with_weapon();
        mgr.item_defs = defs.clone();
        let e = mgr.get_entity_mut(ENTITY).unwrap();
        let slot = e.bandolier_items.get_mut(&0).unwrap();
        slot.item_id = design;
        slot.instance_id = LIVE_INSTANCE;

        let msgs = send(&mut mgr, &request(LIVE_INSTANCE, special)).await;
        assert!(
            matches!(
                msgs.first(),
                Some(CellToBaseMsg::BandolierAmmoUpdate { cur_ammo_type, .. })
                    if *cur_ammo_type == special
            ),
            "weapon {design} must accept special {special}: {msgs:?}"
        );
    }

    let mut mgr = player_with_weapon();
    mgr.item_defs = defs.clone();
    let slot = mgr
        .get_entity_mut(ENTITY)
        .unwrap()
        .bandolier_items
        .get_mut(&0)
        .unwrap();
    slot.item_id = design;
    slot.instance_id = LIVE_INSTANCE;
    let msgs = send(&mut mgr, &request(LIVE_INSTANCE, DART_POISON)).await;
    assert_eq!(
        feedback_text(&only_feedback_line(&msgs)),
        "That weapon cannot use that ammo type."
    );
}

/// A weapon whose column does not list Hollow Point refuses it: the special
/// is gated per weapon by the seed, not granted to every gun.
#[tokio::test]
async fn live_db_request_ammo_change_refuses_a_special_the_column_omits() {
    let pool = require_db_or_skip!();
    let defs = spawner::load_item_defs(&pool)
        .await
        .expect("load_item_defs");
    let (&design, _) = defs
        .iter()
        .filter(|(_, d)| {
            !d.allowed_ammo_types.contains(&BULLET_HOLLOW_POINT)
                && d.default_ammo_type != BULLET_HOLLOW_POINT
        })
        .min_by_key(|(id, _)| **id)
        .expect("some weapon keeps a list without Hollow Point");

    let mut mgr = player_with_weapon();
    mgr.item_defs = defs;
    let slot = mgr
        .get_entity_mut(ENTITY)
        .unwrap()
        .bandolier_items
        .get_mut(&0)
        .unwrap();
    slot.item_id = design;
    slot.instance_id = LIVE_INSTANCE;

    let msgs = send(&mut mgr, &request(LIVE_INSTANCE, BULLET_HOLLOW_POINT)).await;
    assert_eq!(
        feedback_text(&only_feedback_line(&msgs)),
        "That weapon cannot use that ammo type.",
        "weapon {design}"
    );
    let e = mgr.get_entity(ENTITY).unwrap();
    assert_eq!(e.bandolier_items[&0].cur_ammo_type, 1);
}
