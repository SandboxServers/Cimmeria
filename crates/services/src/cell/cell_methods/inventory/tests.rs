use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{AMMO_SLOT_1, AMMO_SLOT_2};
use tokio::sync::mpsc;

use crate::cell::abilities::handle_use_ability;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::constants::GENERICPROPERTY_AMMO_TYPE_ID;
use super::dispatch::dispatch;
use super::{REQUEST_ACTIVE_SLOT_CHANGE, REQUEST_AMMO_CHANGE};

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// Register a no-warmup, ranged ability with `required_ammo = 1` and no
/// event-set (silences onSequence noise during tests).
fn register_test_fire_ability(mgr: &mut SpaceManager, ability_id: i32) {
    mgr.ability_defs.insert(
        ability_id,
        AbilityDef {
            ability_id,
            name: format!("test_fire_{ability_id}"),
            cooldown: 0.001, // very short so back-to-back fires aren't gated
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );
}

/// Stage E: per-slot ammo must survive an active-slot swap.
///
/// Simplified vs the prompt — instead of running the full handle_use_ability
/// twice on slot 0 then once on slot 1 (which would require waiting out
/// cooldowns), we exercise the swap path directly: fire once, swap, fire
/// once, swap back, and verify both slots' ammo is preserved across the
/// swap. The slot-swap message order assertion is the load-bearing piece.
#[tokio::test]
async fn slot_swap_preserves_per_slot_ammo() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let ability_id = 5001;
    register_test_fire_ability(&mut mgr, ability_id);

    // Two bandolier slots: slot 0 (clip 30, full), slot 1 (clip 12, full).
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.abilities.add_ability(ability_id);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 11,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 12,
                cur_ammo_type: 7,
            },
        );
        e.active_bandolier_slot = 0;
        if let Some(s) = e.stats.get_mut(AMMO_SLOT_1) {
            s.update(0, 30, 30);
            s.clear_dirty();
        }
        if let Some(s) = e.stats.get_mut(AMMO_SLOT_2) {
            s.update(0, 12, 12);
            s.clear_dirty();
        }
    }

    let (tx, mut rx) = mpsc::channel(64);

    // Fire two shots on slot 0 → current_ammo == 28. Reset the cooldown
    // between shots instead of sleeping past it: a 1ms cooldown plus a 5ms
    // sleep is enough on a quiet box but flakes under CI scheduler stalls.
    handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.clear_all_cooldowns();
    }
    handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
    assert_eq!(
        mgr.get_entity(1).unwrap().bandolier_items[&0].current_ammo,
        28
    );

    // Drain rx so the swap-message assertions below can scan a clean buffer.
    while rx.try_recv().is_ok() {}

    // ── Swap to slot 1 ──────────────────────────────────────────────
    // Args: bag_id=3 (bandolier), slot_id=1.
    let mut swap_args = Vec::with_capacity(8);
    swap_args.extend_from_slice(&3i32.to_le_bytes());
    swap_args.extend_from_slice(&1i32.to_le_bytes());
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    dispatch(
        1,
        REQUEST_ACTIVE_SLOT_CHANGE,
        &swap_args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Stage D message order on swap: BandolierAmmoUpdate(prev=0, ammo=28)
    // → ActiveSlotUpdate(1) → onEntityProperty(AmmoTypeId, slot1.cur_ammo_type=7).
    let m1 = rx
        .try_recv()
        .expect("expected BandolierAmmoUpdate(prev slot)");
    match m1 {
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } => {
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 0, "first swap msg should flush prev slot 0");
            assert_eq!(
                expected_item_id, 10,
                "should carry slot 0's item_id for TOCTOU guard"
            );
            assert_eq!(current_ammo, 28);
            assert_eq!(cur_ammo_type, 1);
        }
        other => panic!("expected BandolierAmmoUpdate, got {other:?}"),
    }
    let m2 = rx.try_recv().expect("expected ActiveSlotUpdate");
    match m2 {
        CellToBaseMsg::ActiveSlotUpdate { player_id, slot_id } => {
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 1);
        }
        other => panic!("expected ActiveSlotUpdate, got {other:?}"),
    }
    let m3 = rx
        .try_recv()
        .expect("expected onEntityProperty(AmmoTypeId)");
    match m3 {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(
                method_index,
                crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                "third swap msg should be onEntityProperty"
            );
            let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            assert_eq!(prop_id, GENERICPROPERTY_AMMO_TYPE_ID);
            assert_eq!(value, 7, "ammo type should be slot 1's cur_ammo_type");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    // Fire once on slot 1 → current_ammo == 11.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.clear_all_cooldowns();
    }
    handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;
    assert_eq!(
        mgr.get_entity(1).unwrap().bandolier_items[&1].current_ammo,
        11
    );

    // ── Swap back to slot 0 ─────────────────────────────────────────
    let mut swap_back = Vec::with_capacity(8);
    swap_back.extend_from_slice(&3i32.to_le_bytes());
    swap_back.extend_from_slice(&0i32.to_le_bytes());
    dispatch(
        1,
        REQUEST_ACTIVE_SLOT_CHANGE,
        &swap_back,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Slot 0's ammo is preserved across the swap.
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.bandolier_items[&0].current_ammo, 28,
        "slot 0 ammo preserved across swap"
    );
    assert_eq!(
        entity.bandolier_items[&1].current_ammo, 11,
        "slot 1 ammo preserved across swap"
    );
    assert_eq!(
        entity.stats.get(AMMO_SLOT_1).unwrap().cur,
        28,
        "AmmoSlot1 stat still reads 28"
    );
    assert_eq!(entity.active_bandolier_slot, 0);
}

/// CodeRabbit #4: starting a reload on slot 0, swapping to slot 1, then
/// letting the warmup elapse must NOT refill slot 1 with slot 0's clip
/// size. The fix cancels the in-flight reload on swap, so the tick has
/// nothing to promote and slot 1's ammo stays where it was.
#[tokio::test]
async fn slot_swap_cancels_in_flight_reload() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

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
                current_ammo: 5,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 11,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 8,
                cur_ammo_type: 7,
            },
        );
        e.active_bandolier_slot = 0;
        // Simulate a reload of slot 0 currently warming up.
        e.reload_complete_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(10));
        e.reload_slot_id = Some(0);
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    let engine = build_engine(None).await;

    // Swap to slot 1.
    let mut swap = Vec::with_capacity(8);
    swap.extend_from_slice(&3i32.to_le_bytes());
    swap.extend_from_slice(&1i32.to_le_bytes());
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &swap, &tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).unwrap();
    assert!(
        entity.reload_complete_at.is_none(),
        "swap should cancel in-flight reload"
    );
    assert!(
        entity.reload_slot_id.is_none(),
        "swap should clear pinned slot"
    );
    assert_eq!(
        entity.bandolier_items[&0].current_ammo, 5,
        "cancelled reload must NOT refill slot 0"
    );
    assert_eq!(
        entity.bandolier_items[&1].current_ammo, 8,
        "slot 1 untouched"
    );
    assert_eq!(entity.active_bandolier_slot, 1);

    // Drain the rx so the channel doesn't error if assertions above
    // succeeded (slot-swap emits ActiveSlotUpdate and onEntityProperty).
    while rx.try_recv().is_ok() {}
}

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

/// CodeRabbit out-of-diff: REQUEST_ACTIVE_SLOT_CHANGE must reject slot_id
/// outside 0..5 before mutating active_bandolier_slot or sending
/// ActiveSlotUpdate. A forged value would otherwise leave the entity in
/// an impossible state.
#[tokio::test]
async fn request_active_slot_change_rejects_out_of_range_slot() {
    use crate::cell::content::build_engine;

    for bad_slot in [-1i32, 5, 99, i32::MAX] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.active_bandolier_slot = 0;
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes()); // bag_id = 3
        args.extend_from_slice(&bad_slot.to_le_bytes()); // out-of-range slot

        let handled = dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;
        assert!(
            handled,
            "handler claims the index even when slot is invalid (bad_slot={bad_slot})"
        );
        assert!(
            rx.try_recv().is_err(),
            "no messages emitted for bad_slot={bad_slot}"
        );
        assert_eq!(
            mgr.get_entity(1).unwrap().active_bandolier_slot,
            0,
            "active_bandolier_slot must not change for bad_slot={bad_slot}"
        );
    }
}
