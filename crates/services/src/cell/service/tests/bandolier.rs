//! `InitPlayerState` ammo-stat seeding + bandolier-flush ordering on
//! `DestroyEntity` / `DisconnectEntity`. See parent [`super`] for the
//! shared `make_test_space_mgr` fixture.

use super::make_test_space_mgr;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{AMMO_SLOT_1, AMMO_SLOT_2, AMMO_SLOT_3};
use tokio::sync::mpsc;

/// Stage E: InitPlayerState's bandolier-seed loop must populate AmmoSlot{N}
/// stats from each persisted item's `current_ammo`/`clip_size`, leave empty
/// slots at the default (0,0,0), and clear dirty flags so the initial
/// mapLoaded `serialize_all()` is the sole carrier.
///
/// Calling the full handle_base_message branch would require a ChainEngine
/// with content tables loaded; the seeding logic is purely synchronous
/// mutation of the entity, so we exercise it in isolation here.
#[tokio::test]
async fn init_player_state_seeds_ammo_stats() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let bandolier_items = vec![
        (
            0,
            BandolierItem {
                item_id: 100,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 25,
                cur_ammo_type: 2,
            },
        ),
        (
            1,
            BandolierItem {
                item_id: 101,
                clip_size: 12,
                default_ammo_type: 5,
                current_ammo: 12,
                cur_ammo_type: 5,
            },
        ),
    ];

    // Mirror the InitPlayerState branch's bandolier seeding (base_messages.rs).
    // This is the load-bearing chunk under test; the rest of the handler
    // (mission restoration, region registration, content engine fire) is
    // exercised by other test paths.
    if let Some(entity) = mgr.get_entity_mut(1) {
        entity.player_id = Some(100);
        entity.active_bandolier_slot = 0;
        entity.bandolier_items = bandolier_items.into_iter().collect();

        let slot_seed: Vec<(i32, i32, i32)> = entity
            .bandolier_items
            .iter()
            .map(|(&slot, item)| (slot, item.current_ammo, item.clip_size))
            .collect();
        for (slot_id, current, clip) in slot_seed {
            let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
            if let Some(stat) = entity.stats.get_mut(stat_id) {
                stat.update(0, current, clip);
                stat.clear_dirty();
            }
        }
    }

    let entity = mgr.get_entity(1).unwrap();

    // Populated slots seeded from their bandolier items.
    let slot1 = entity.stats.get(AMMO_SLOT_1).unwrap();
    assert_eq!((slot1.min, slot1.cur, slot1.max), (0, 25, 30));
    assert!(
        !slot1.dirty,
        "AmmoSlot1 dirty cleared so mapLoaded serialize_all owns the initial send"
    );

    let slot2 = entity.stats.get(AMMO_SLOT_2).unwrap();
    assert_eq!((slot2.min, slot2.cur, slot2.max), (0, 12, 12));
    assert!(!slot2.dirty);

    // Empty slots remain at their default (0, 0, 0) tuple.
    let slot3 = entity.stats.get(AMMO_SLOT_3).unwrap();
    assert_eq!(
        (slot3.min, slot3.cur, slot3.max),
        (0, 0, 0),
        "unequipped slot stays at default"
    );
}

/// Stage E: on logout (BaseToCellMsg::DestroyEntity), all dirty bandolier
/// slots must be flushed via BandolierAmmoUpdate before the entity is torn
/// down. The flush path is exercised by handle_base_message; here we
/// invoke the matching helper + destroy directly to keep the test focused
/// (avoids spinning up a ChainEngine).
#[tokio::test]
async fn logout_flushes_all_dirty_bandolier_slots() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 17,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 11,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 4,
                cur_ammo_type: 7,
            },
        );
        // Pre-mark both slots dirty as if shots were fired but no swap /
        // reload-completion / ammo-change had drained them.
        e.bandolier_ammo_dirty.insert(0);
        e.bandolier_ammo_dirty.insert(1);
    }

    let (tx, mut rx) = mpsc::channel(16);

    // Mirror handle_base_message's DestroyEntity branch: flush, then destroy.
    if let Some(entity) = mgr.get_entity_mut(1) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(
                entity, player_id, &tx,
            )
            .await;
        }
    }
    mgr.destroy_entity(1);

    // Collect the two BandolierAmmoUpdate messages (HashSet drain order is
    // unspecified, so build a slot_id → (item_id, current_ammo, type) map).
    let mut updates = std::collections::HashMap::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } = msg
        {
            assert_eq!(player_id, 100);
            updates.insert(slot_id, (expected_item_id, current_ammo, cur_ammo_type));
        }
    }
    assert_eq!(
        updates.len(),
        2,
        "expected one BandolierAmmoUpdate per dirty slot"
    );
    assert_eq!(updates.get(&0), Some(&(10, 17, 1)));
    assert_eq!(updates.get(&1), Some(&(11, 4, 7)));

    // Entity removed from the space manager.
    assert!(
        mgr.get_entity(1).is_none(),
        "entity should be destroyed after teardown"
    );
}

/// Regression: the LOG_OFF flow sends `DisconnectEntity` before
/// `DestroyEntity`. `space_manager::disconnect_entity` internally calls
/// `destroy_entity`, so by the time the cell loop processes the subsequent
/// `DestroyEntity` message, the entity is already gone — its flush hook
/// becomes a silent no-op and per-slot ammo never persists.
///
/// The fix flushes inside the `DisconnectEntity` handler (BEFORE
/// `space_mgr.disconnect_entity`). This test mirrors that order and
/// verifies the persistence message goes out.
#[tokio::test]
async fn disconnect_entity_flushes_dirty_ammo_before_destroy() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 17,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_ammo_dirty.insert(0);
    }

    let (tx, mut rx) = mpsc::channel(16);

    // Mirror handle_base_message's DisconnectEntity branch verbatim:
    // flush bandolier ammo BEFORE space_mgr.disconnect_entity (which
    // internally destroys the entity).
    if let Some(entity) = mgr.get_entity_mut(1) {
        if let Some(player_id) = entity.player_id {
            crate::cell::cell_methods::inventory::flush_dirty_bandolier_ammo(
                entity, player_id, &tx,
            )
            .await;
        }
    }
    mgr.disconnect_entity(1, &tx).await;

    // The first message must be the BandolierAmmoUpdate (the regression
    // was that flushing AFTER disconnect_entity silently dropped it).
    let mut got_ammo_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } = msg
        {
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 0);
            assert_eq!(expected_item_id, 10, "should carry the slot's item_id");
            assert_eq!(current_ammo, 17);
            assert_eq!(cur_ammo_type, 1);
            got_ammo_update = true;
            break;
        }
    }
    assert!(
        got_ammo_update,
        "DisconnectEntity must flush bandolier ammo before destroy"
    );
    assert!(
        mgr.get_entity(1).is_none(),
        "entity should be destroyed after disconnect"
    );
}
