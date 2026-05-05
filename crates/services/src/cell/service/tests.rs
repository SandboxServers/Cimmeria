//! CellService tick + handler tests — exercise reload-completion, bandolier
//! seeding, logout flushes, disconnect-flush ordering, and NPC AI state
//! transitions without spinning up a full ChainEngine.

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, BandolierItem};
use cimmeria_entity::stats::{AMMO_SLOT_1, AMMO_SLOT_2, AMMO_SLOT_3, HEALTH};
use tokio::sync::mpsc;

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

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
    }
    // connect_entity inserts the entity into space.players, which
    // `all_player_entity_ids()` reads. Without it the tick skips the entity.
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    super::ticks::reload_completion_tick(&tx, &mut mgr).await;

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
            assert!(count >= 1);
            let mut found_ammo = false;
            for i in 0..count as usize {
                let off = 4 + i * 16;
                let stat_id =
                    i32::from_le_bytes([args[off], args[off + 1], args[off + 2], args[off + 3]]);
                let cur = i32::from_le_bytes([
                    args[off + 8],
                    args[off + 9],
                    args[off + 10],
                    args[off + 11],
                ]);
                if stat_id == AMMO_SLOT_1 {
                    assert_eq!(cur, 30);
                    found_ammo = true;
                }
            }
            assert!(found_ammo, "onStatUpdate missing AmmoSlot1=30");
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }

    // Second: BandolierAmmoUpdate for persistence.
    let m2 = rx.try_recv().expect("expected BandolierAmmoUpdate");
    match m2 {
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
                expected_item_id, 1,
                "should carry the slot's item_id for TOCTOU guard"
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
    super::ticks::reload_completion_tick(&tx, &mut mgr).await;

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
    let mut saw_ability_end = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_SEQUENCE && args.len() >= 22 {
                let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                if seq_id != END_SEQ_ID {
                    continue;
                }
                let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let primary = args[12];
                let view_type = args[21];
                assert_eq!(source_id, 1, "source = entity_id");
                assert_eq!(target_id, 1, "reload-end targets self");
                assert_eq!(primary, 1, "primary target flag set");
                assert_eq!(
                    view_type, 0,
                    "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's fire path \
                     so reload-end animates consistently with weapon fire animations"
                );
                saw_ability_end = true;
            }
        }
    }
    assert!(
        saw_ability_end,
        "reload_completion_tick must emit onSequence with the Ability_End \
         sequence id when the reload ability has an event_set_id; \
         legacy parity with `AbilityManager.py:671-673` (afterWarmup plays \
         Ability_End once the warmup elapses)"
    );
}

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
            super::super::cell_methods::inventory::flush_dirty_bandolier_ammo(
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
            super::super::cell_methods::inventory::flush_dirty_bandolier_ammo(
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

// ── NPC AI tick ───────────────────────────────────────────────────────
//
// These cover the four arms of `npc_ai_tick` that previously had zero
// coverage — Fighting → Idle (no threat), Fighting → Leashing (target
// past LEASH_DISTANCE from spawn), stationary-no-pathfind, and
// Leashing → Idle (snap back + heal).

/// Build a non-instanced "Castle" space and seed an NPC at id=200 in
/// AiState::Fighting with the given spawn position. Returns the
/// SpaceManager. Caller layers in the threat list and ability defs.
///
/// We use a non-instanced world here because the previous fixture
/// (Castle_CellBlock, instanced) gave each subsequent `create_entity`
/// call a fresh space — putting the NPC and any added targets in
/// DIFFERENT instances. `has_line_of_sight(npc, target)` then falls
/// back to true and `find_path` to None, masking what the in-range /
/// LOS branches of `npc_ai_fight` actually do. The non-instanced
/// fixture co-locates everyone so those branches actually exercise.
fn make_ai_fixture(npc_spawn: [f32; 3], npc_pos: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", npc_pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04; // SGWMob — required for all_npc_entity_ids()
        npc.ai_state = AiState::Fighting;
        npc.spawn_position = Some(Vector3::new(npc_spawn[0], npc_spawn[1], npc_spawn[2]));
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr
}

/// Fighting NPC with an empty threat list resets to Idle. The
/// regression guard for the early-return that re-enables this NPC
/// to be re-aggrod by the next attacker.
#[tokio::test]
async fn npc_ai_fighting_with_empty_threat_resets_to_idle() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(8);
    super::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    assert!(matches!(
        mgr.get_entity(200).unwrap().ai_state,
        AiState::Idle
    ));
}

/// Target sitting past `LEASH_DISTANCE` (50.0) from the NPC's spawn
/// triggers AiState::Leashing and clears the threat list. Pin the
/// transition so a refactor that drops the leash branch can't
/// silently let mobs path across the whole zone.
#[tokio::test]
async fn npc_ai_target_beyond_leash_distance_triggers_leashing() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Target player at distance 100 from spawn (LEASH_DISTANCE=50).
    mgr.create_entity(100, "Castle", [100.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        p.player_id = Some(1);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
    }
    let (tx, _rx) = mpsc::channel(8);
    super::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(matches!(npc.ai_state, AiState::Leashing));
    assert!(
        npc.threat_list.is_empty(),
        "leashing must clear the threat list"
    );
}

/// A dead target is removed from the threat list while the OTHER
/// live threats remain — so the next tick has a target to pick. Pin
/// both the surgical removal of just the dead one AND the AI staying
/// Fighting. An implementation that scans through and bulk-clears
/// the whole list on first dead encounter would silently break
/// multi-target combat.
#[tokio::test]
async fn npc_ai_dead_target_is_removed_but_other_threats_remain() {
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    // Dead target close to NPC.
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Live secondary target, also near the NPC.
    mgr.create_entity(101, "Castle", [12.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 0, 100);
            h.clear_dirty();
        }
    }
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        // 100 has the highest threat (so it's selected first → dead → removed),
        // 101 stays as the next-tick target.
        npc.threat_list.insert(100, 5.0);
        npc.threat_list.insert(101, 2.0);
    }
    let (tx, _rx) = mpsc::channel(8);
    super::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.threat_list.contains_key(&100),
        "dead target must be removed from threat list"
    );
    assert_eq!(
        npc.threat_list.get(&101),
        Some(&2.0),
        "live secondary threat must survive the dead-target prune"
    );
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "AI stays Fighting so next tick picks up another target"
    );
}

/// Stationary NPC out of attack range / LOS does NOT pathfind. Pin
/// so a refactor that runs the pathfinder unconditionally doesn't
/// turn turrets into chasers.
#[tokio::test]
async fn npc_ai_stationary_does_not_pathfind_when_out_of_range() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Target at distance 40 (within LEASH=50 but past NPC_ATTACK_RANGE=30).
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        npc.is_stationary = true;
    }
    let (tx, _rx) = mpsc::channel(8);
    super::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "stationary NPC must not populate a nav path; got {:?}",
        npc.nav_path
    );
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "stationary NPC stays Fighting; only the leash branch transitions"
    );
}

/// Leashing tick: NPC snaps back to spawn, health restores to max,
/// AI resets to Idle, threat list clears, cooldowns clear. The
/// canary for the "leash never returns to Idle" hang.
#[tokio::test]
async fn npc_ai_leashing_snaps_to_spawn_restores_health_and_idles() {
    let mut mgr = make_ai_fixture([0.0; 3], [40.0, 0.0, 40.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Leashing;
        // Seed pre-leash state: a damaged NPC with stale threat targets
        // and an active cooldown carried over from the Fighting phase.
        // The leash tick must wipe ALL of these — pin each one so a
        // regression that only handles ai_state but leaves threat or
        // cooldowns dangling gets caught.
        npc.threat_list.insert(100, 5.0);
        npc.threat_list.insert(101, 2.0);
        npc.abilities
            .start_ability_cooldown(592, std::time::Duration::from_secs(60));
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 5, 100); // damaged
            h.clear_dirty();
        }
    }
    let (tx, _rx) = mpsc::channel(16);
    super::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(matches!(npc.ai_state, AiState::Idle));
    assert_eq!(
        npc.position,
        Vector3::new(0.0, 0.0, 0.0),
        "leash must snap NPC back to spawn"
    );
    assert_eq!(
        npc.stats.get(HEALTH).unwrap().cur,
        100,
        "leash must restore health to max"
    );
    assert!(
        npc.threat_list.is_empty(),
        "leash must wipe pre-seeded threat targets"
    );
    assert!(
        !npc.abilities.is_on_cooldown(592),
        "leash must clear pre-seeded cooldown"
    );
}
