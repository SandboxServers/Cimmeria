//! `reload_completion_tick` refill + wire-frame coverage. See parent
//! [`super`] for the shared `make_test_space_mgr` fixture.

use super::make_test_space_mgr;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::AMMO_SLOT_1;
use tokio::sync::mpsc;

/// Stage E: when reload_complete_at has elapsed, the tick must
/// (a) refill the active slot's magazine to clip_size, (b) sync the
/// AmmoSlot{N} stat, (c) clear reload_complete_at, (d) drain the slot's
/// dirty flag (the BandolierAmmoUpdate emitted next persists it), and
/// (e) emit onStatUpdate (method 20) plus the BandolierAmmoUpdate.
#[tokio::test]
async fn reload_completion_tick_refills_and_sends_stat() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                // Distinct from item_id (design id) so the reload-tick persist
                // assertion proves the guard keys on the instance PK.
                instance_id: 1001,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 5,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Already-elapsed deadline so the tick promotes immediately.
        e.reload_complete_at =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        e.reload_slot_id = Some(0);
        if let Some(s) = e.stats.get_mut(AMMO_SLOT_1) {
            s.update(0, 5, 30);
            s.clear_dirty();
        }
        // Clear any other stats that `create_entity` may have left dirty so
        // the wire assertion below can pin `count == 1` (only AmmoSlot1).
        e.stats.clear_dirty();
    }
    // connect_entity inserts the entity into space.players, which
    // `all_player_entity_ids()` reads. Without it the tick skips the entity.
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::reload_completion_tick(&tx, &mut mgr).await;

    // ── Entity-state assertions ─────────────────────────────────────
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.bandolier_items[&0].current_ammo, 30,
        "magazine refilled to clip_size"
    );
    assert_eq!(
        entity.stats.get(AMMO_SLOT_1).unwrap().cur,
        30,
        "AmmoSlot1 stat refilled"
    );
    assert!(
        entity.reload_complete_at.is_none(),
        "reload_complete_at cleared"
    );
    assert!(entity.reload_slot_id.is_none(), "reload_slot_id cleared");
    assert!(
        !entity.bandolier_ammo_dirty.contains(&0),
        "active slot's dirty flag drained"
    );

    // ── Wire-message assertions ─────────────────────────────────────
    // First: onStatUpdate (method 20) carrying AmmoSlot1=30.
    //
    // The dirty set is cleared above to ensure `serialize_dirty` emits
    // exactly the one stat the tick mutates (AMMO_SLOT_1). A regression
    // that bundles unrelated stats into the same dirty payload would fail
    // the `count == 1` assertion immediately.
    let m1 = rx.try_recv().expect("expected onStatUpdate");
    match m1 {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, 20);
            let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            assert_eq!(
                count, 1,
                "reload-tick onStatUpdate must carry exactly AMMO_SLOT_1; \
                 got {count} stats in payload"
            );
            // Stat tuple at offset 4 (16 bytes per stat): stat_id, min, cur, max.
            let stat_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
            let min = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
            let cur = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
            let max = i32::from_le_bytes([args[16], args[17], args[18], args[19]]);
            assert_eq!(stat_id, AMMO_SLOT_1);
            assert_eq!(min, 0);
            assert_eq!(cur, 30, "magazine refilled to clip_size on the wire");
            assert_eq!(max, 30);
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    // Second: BandolierAmmoUpdate for persistence.
    let m2 = rx.try_recv().expect("expected BandolierAmmoUpdate");
    match m2 {
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
        } => {
            assert_eq!(player_id, 100);
            assert_eq!(slot_id, 0);
            assert_eq!(
                expected_instance_id, 1001,
                "should carry the slot's instance PK (sgw_inventory.item_id) for TOCTOU guard, not the design id"
            );
            assert_eq!(current_ammo, 30);
            assert_eq!(cur_ammo_type, 2);
        }
        other => panic!("expected BandolierAmmoUpdate, got {other:?}"),
    }

    assert!(rx.try_recv().is_err(), "no further rx messages expected");
}

/// Mirror of `reload_completion_tick_refills_and_sends_stat` but with the
/// reload ability's `event_set_id` seeded so the post-warmup `Ability_End`
/// ON_SEQUENCE branch actually fires. Pins the byte layout (sequence_id at
/// offset 0, source/target at offsets 4 and 8) and the ViewType=0
/// (KISMET_VIEW_Witness) choice that mirrors `use_ability.rs`'s fire path.
///
/// Without this guard the new branch in `reload_completion_tick` is dead
/// code in the test corpus — the original tick-refill test deliberately
/// leaves `ability_defs` empty so the `event_set_id` lookup short-circuits.
#[tokio::test]
async fn reload_completion_tick_emits_ability_end_sequence_when_event_set_present() {
    use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
    use crate::cell::spawner::EVENT_ABILITY_END;
    use cimmeria_entity::abilities::AbilityDef;

    const ABILITY_RELOAD_WEAPON: i32 = 596;
    const EVENT_SET_ID: i32 = 7777;
    const END_SEQ_ID: i32 = 9001;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 1001,
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        e.reload_complete_at =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        e.reload_slot_id = Some(0);
    }
    mgr.connect_entity(1);

    // Seed the reload ability's event_set + the Ability_End sequence so the
    // tick can actually look up a sequence_id to send.
    mgr.ability_defs.insert(
        ABILITY_RELOAD_WEAPON,
        AbilityDef {
            ability_id: ABILITY_RELOAD_WEAPON,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 2.0,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: Some(EVENT_SET_ID),
            velocity: 0.0,
        },
    );
    mgr.sequence_map
        .insert((EVENT_SET_ID, EVENT_ABILITY_END), END_SEQ_ID);

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::ticks::reload_completion_tick(&tx, &mut mgr).await;

    // Drain to find the ON_SEQUENCE call; the tick also emits onStatUpdate
    // and BandolierAmmoUpdate, asserted in the sibling test, so we only
    // care about the new Ability_End packet here.
    // ON_SEQUENCE wire layout (26 bytes):
    //   sequence_id   i32 LE  @ 0..4
    //   source_id     i32 LE  @ 4..8
    //   target_id     i32 LE  @ 8..12
    //   primary       u8      @ 12
    //   impact_time   f32 LE  @ 13..17
    //   nvp_count     u32 LE  @ 17..21
    //   view_type     u8      @ 21
    //   instance_id   i32 LE  @ 22..26
    let mut ability_end_count = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_SEQUENCE {
                assert_eq!(
                    args.len(),
                    26,
                    "ON_SEQUENCE payload must be exactly 26 bytes — any drift in \
                     the serializer would silently corrupt the kismet event frame \
                     on the wire"
                );
                let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                if seq_id != END_SEQ_ID {
                    continue;
                }
                let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let primary = args[12];
                let impact_time = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);
                let nvp_count = u32::from_le_bytes([args[17], args[18], args[19], args[20]]);
                let view_type = args[21];
                let instance_id = i32::from_le_bytes([args[22], args[23], args[24], args[25]]);
                assert_eq!(source_id, 1, "source = entity_id");
                assert_eq!(target_id, 1, "reload-end targets self");
                assert_eq!(primary, 1, "primary target flag set");
                assert_eq!(impact_time, 0.0, "no projectile impact time for reload");
                assert_eq!(nvp_count, 0, "no name-value pairs in payload");
                assert_eq!(
                    view_type, 0,
                    "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's fire path \
                     so reload-end animates consistently with weapon fire animations"
                );
                assert_eq!(instance_id, 0, "no effect instance for the reload sequence");
                ability_end_count += 1;
            }
        }
    }
    assert_eq!(
        ability_end_count, 1,
        "reload_completion_tick must emit exactly one onSequence with the \
         Ability_End sequence id when the reload ability has an event_set_id; \
         legacy parity with `AbilityManager.py:671-673` (afterWarmup plays \
         Ability_End once the warmup elapses). Got {ability_end_count}."
    );
}
