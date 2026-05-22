use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{AMMO_SLOT_1, AMMO_SLOT_2};
use tokio::sync::mpsc;

use crate::cell::abilities::handle_use_ability;
use crate::cell::messages::CellToBaseMsg;

use super::super::constants::GENERICPROPERTY_AMMO_TYPE_ID;
use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::{make_test_space_mgr, register_test_fire_ability};

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
        // Weapon already drawn so the attack-while-holstered queue
        // doesn't intercept the fire calls — this test is about
        // per-slot ammo preservation across the swap.
        e.weapon_holstered = false;
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

    // Pre-stamp pending_slot_swap_at so the handler treats this
    // dispatch as the re-entry-from-tick path and runs the
    // immediate swap. Without this the choreography branch fires
    // Item_Unequip and defers the actual slot change — this test
    // verifies per-slot ammo preservation across the SWAP, not the
    // choreography timing.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }

    // ── Swap to slot 1 (server-internal indexing) ──────────────────
    // Wire slots are 1-indexed (legacy convention from `Bag.py:369` /
    // `SGWPlayer.py:2192`), so swapping to server slot 1 means sending
    // wire slot 2.
    let mut swap_args = Vec::with_capacity(8);
    swap_args.extend_from_slice(&3i32.to_le_bytes()); // bag_id=3 (bandolier)
    swap_args.extend_from_slice(&2i32.to_le_bytes()); // wire slot 2 → server slot 1
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

    // Stage D message order on swap:
    //   BandolierAmmoUpdate(prev=0, ammo=28)
    //   → ActiveSlotUpdate(1)                          (base persistence)
    //   → onActiveSlotUpdate(bagId=3, wireSlot=2)       (client UI indicator)
    //   → onEntityProperty(AmmoTypeId, slot1.cur_ammo_type=7)
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
        CellToBaseMsg::ActiveSlotUpdate {
            entity_id,
            player_id,
            slot_id,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 1);
        }
        other => panic!("expected ActiveSlotUpdate, got {other:?}"),
    }
    let m3 = rx.try_recv().expect("expected onActiveSlotUpdate");
    match m3 {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(
                method_index,
                crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE,
                "third swap msg should be onActiveSlotUpdate (the client UI indicator)"
            );
            let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let wire_slot = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            assert_eq!(bag_id, 3, "bandolier bag");
            assert_eq!(
                wire_slot, 2,
                "wire slot must be server slot + 1 (legacy `Bag.py:369`)"
            );
        }
        other => panic!("expected EntityMethodCall(onActiveSlotUpdate), got {other:?}"),
    }
    let m4 = rx
        .try_recv()
        .expect("expected onEntityProperty(AmmoTypeId)");
    match m4 {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(
                method_index,
                crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY,
                "fourth swap msg should be onEntityProperty"
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

    // Pre-stamp again so the swap-back also takes the re-entry path.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }

    // ── Swap back to slot 0 (wire 1 → server 0) ─────────────────────
    let mut swap_back = Vec::with_capacity(8);
    swap_back.extend_from_slice(&3i32.to_le_bytes());
    swap_back.extend_from_slice(&1i32.to_le_bytes()); // wire slot 1 → server slot 0
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
        // Pre-stamp pending_slot_swap_at so the handler treats this
        // as the re-entry-from-tick path and runs the immediate swap.
        // This test verifies in-flight-reload cancellation, not the
        // choreography timing.
        e.pending_slot_swap_at = Some(std::time::Instant::now());
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

    // Swap to slot 1 (server-internal). Wire slot is 1-indexed, so we send 2.
    let mut swap = Vec::with_capacity(8);
    swap.extend_from_slice(&3i32.to_le_bytes());
    swap.extend_from_slice(&2i32.to_le_bytes()); // wire slot 2 → server slot 1
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
