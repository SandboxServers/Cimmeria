use cimmeria_entity::cell_entity::BandolierItem;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::constants::GENERICPROPERTY_AMMO_TYPE_ID;
use super::super::dispatch::dispatch;
use super::super::REQUEST_AMMO_CHANGE;
use super::make_test_space_mgr;

/// Stage E: requestAmmoChange must update the slot's `cur_ammo_type`,
/// emit exactly one BandolierAmmoUpdate (drained immediately, NOT pending
/// for logout flush), and — when the slot is active — push an
/// `onEntityProperty(AmmoTypeId)` to the client.
#[tokio::test]
async fn request_ammo_change_updates_slot_and_sends_property() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 42,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 20,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, mut rx) = mpsc::channel(16);

    // Args: item_id=42, ammo_type=3 (8 bytes LE).
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&42i32.to_le_bytes());
    args.extend_from_slice(&3i32.to_le_bytes());

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    dispatch(1, REQUEST_AMMO_CHANGE, &args, &tx, &mut mgr, &engine).await;

    // Slot mutated, dirty NOT set (drained immediately by the handler).
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(entity.bandolier_items[&0].cur_ammo_type, 3);
    assert!(
        !entity.bandolier_ammo_dirty.contains(&0),
        "dirty flag should be drained immediately"
    );

    // First message: BandolierAmmoUpdate carrying the new type + existing ammo.
    let m1 = rx.try_recv().expect("expected BandolierAmmoUpdate");
    match m1 {
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } => {
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 0);
            assert_eq!(
                expected_item_id, 42,
                "should carry the slot's item_id for TOCTOU guard"
            );
            assert_eq!(current_ammo, 20);
            assert_eq!(cur_ammo_type, 3);
        }
        other => panic!("expected BandolierAmmoUpdate, got {other:?}"),
    }

    // Second message: onEntityProperty(AmmoTypeId, 3) since slot is active.
    let m2 = rx.try_recv().expect("expected onEntityProperty");
    match m2 {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(
                method_index,
                crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
            );
            let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            assert_eq!(prop_id, GENERICPROPERTY_AMMO_TYPE_ID);
            assert_eq!(value, 3);
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    // No additional messages.
    assert!(rx.try_recv().is_err(), "no further rx messages expected");
}

/// CodeRabbit #12 (and earlier rejects-zero): non-positive ammo_type is
/// rejected before mutating local state. Without this, a negative value
/// would update `bandolier_items` + send `onEntityProperty`, then fail
/// the DB write (`cur_ammo_type >= 0` CHECK) — leaving cell + client
/// state ahead of persistence.
#[tokio::test]
async fn request_ammo_change_rejects_non_positive() {
    for bad_ammo_type in [0i32, -1, -42] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 42,
                    clip_size: 30,
                    default_ammo_type: 1,
                    current_ammo: 20,
                    cur_ammo_type: 1,
                },
            );
            e.active_bandolier_slot = 0;
        }

        let (tx, mut rx) = mpsc::channel(8);

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&42i32.to_le_bytes());
        args.extend_from_slice(&bad_ammo_type.to_le_bytes());

        let engine = cimmeria_content_engine::chain::ChainEngine::new();
        let handled = dispatch(1, REQUEST_AMMO_CHANGE, &args, &tx, &mut mgr, &engine).await;

        assert!(
            handled,
            "REQUEST_AMMO_CHANGE should be claimed (bad_ammo_type={bad_ammo_type})"
        );
        assert!(
            rx.try_recv().is_err(),
            "no rx messages for bad_ammo_type={bad_ammo_type}"
        );
        let entity = mgr.get_entity(1).unwrap();
        assert_eq!(
            entity.bandolier_items[&0].cur_ammo_type, 1,
            "cur_ammo_type should be unchanged for bad_ammo_type={bad_ammo_type}"
        );
        assert!(
            !entity.bandolier_ammo_dirty.contains(&0),
            "no dirty flag for bad_ammo_type={bad_ammo_type}"
        );
    }
}

/// CodeRabbit #15: requestAmmoChange must reject ammo subtypes that aren't
/// in the weapon's `allowed_ammo_types` whitelist (mirrors
/// `resources.items.ammo_types`). Items with no cache entry fall through
/// (matching legacy `pass` for unknown weapons) — already covered by
/// the existing happy-path test.
#[tokio::test]
async fn request_ammo_change_rejects_unlisted_subtype() {
    use crate::cell::spawner::WeaponDef;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    // Seed the item_defs cache so the validation runs (without an
    // entry, the handler falls through and accepts any positive value).
    mgr.item_defs.insert(
        42,
        WeaponDef {
            clip_size: 30,
            default_ammo_type: 1,
            allowed_ammo_types: vec![1, 3, 5], // 7 not allowed
            holster_animation_duration: std::time::Duration::from_millis(600),
        },
    );

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 42,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 20,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, mut rx) = mpsc::channel(8);

    // ammo_type = 7 — positive, but not in the weapon's whitelist.
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&42i32.to_le_bytes());
    args.extend_from_slice(&7i32.to_le_bytes());

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let handled = dispatch(1, REQUEST_AMMO_CHANGE, &args, &tx, &mut mgr, &engine).await;

    assert!(handled);
    assert!(
        rx.try_recv().is_err(),
        "no rx messages for unlisted ammo_type"
    );
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.bandolier_items[&0].cur_ammo_type, 1,
        "cur_ammo_type unchanged"
    );
    assert!(!entity.bandolier_ammo_dirty.contains(&0));
}
