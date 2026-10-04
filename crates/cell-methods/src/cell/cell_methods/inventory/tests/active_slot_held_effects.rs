//! AB-08: a weapon swap that revokes a weapon-granted ability takes off that
//! ability's held ledger entries (a toggle left on, a passive), which would
//! otherwise outlive it with nothing to remove them. Entries of abilities
//! the player keeps, and timed entries, are untouched.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{BandolierItem, TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::{ACCURACY, DEFENSE};

use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::make_test_space_mgr;

const PISTOL_ITEM_ID: i32 = 55;
const STRIKE: i32 = 594;
const PISTOL_RANGED: i32 = 579;
const EVENT_RANGED: i32 = 7;

fn held(effect_id: i32, ability_id: i32, stat: i32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id,
        ability_id,
        invoker_id: 1,
        effect_flags: 1,
        moniker_ids: vec![],
        stats: vec![(stat, 100)],
        duration_secs: None,
        stacking: TimedStacking::PerSource,
        invoker_identity: Default::default(),
    }
}

/// **Regression guard.** Unequipping the pistol revokes 579; its held
/// entry comes off and Accuracy returns to its base, while Strike's held
/// entry (a kept ability) stays. Fails without the strip in
/// `swap_weapon_granted_abilities_for_slot`.
#[tokio::test]
async fn a_revoked_weapon_ability_loses_its_held_entries() {
    use crate::cell::content::build_engine;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_RANGED), PISTOL_RANGED);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: PISTOL_ITEM_ID,
                clip_size: 12,
                default_ammo_type: 1,
                current_ammo: 12,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
        e.abilities.add_ability(STRIKE);
        e.abilities
            .swap_weapon_granted_abilities([PISTOL_RANGED].into_iter().collect());
        e.pending_slot_swap_at = Some(Instant::now());
    }
    let accuracy = |m: &crate::cell::space_manager::SpaceManager, s: i32| {
        m.get_entity(1).unwrap().stats.get(s).unwrap().cur
    };
    let base = accuracy(&mgr, ACCURACY);
    let now = Instant::now();
    mgr.apply_timed_effect(1, held(9001, PISTOL_RANGED, ACCURACY), now);
    mgr.apply_timed_effect(1, held(9002, STRIKE, DEFENSE), now);
    assert_eq!(accuracy(&mgr, ACCURACY), base + 100, "fixture");
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(64);
    let engine = build_engine(None).await;
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes()); // server slot 1, empty
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.abilities.has_ability(PISTOL_RANGED), "fixture: revoked");
    let ids: Vec<i32> = e.stat_buffs.entries.iter().map(|b| b.effect_id).collect();
    assert_eq!(ids, vec![9002], "only the kept ability's entry remains");
    assert_eq!(accuracy(&mgr, ACCURACY), base, "the revoked +100 restored");
}
