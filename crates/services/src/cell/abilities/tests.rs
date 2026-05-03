//! Tests for the abilities module.
//!
//! - Wire-format spot checks for `serialize_timer_update` (the public helper).
//! - Determinism + uniqueness asserts for `pseudo_random_seed`.
//! - End-to-end fire of `handle_use_ability` covering the ammo-consume path.

use cimmeria_entity::abilities::{TIMER_ABILITY_COOLDOWN, serialize_timer_update};
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::handle_use_ability;
use super::rng::pseudo_random_seed;

#[test]
fn timer_update_has_correct_format() {
    let data = serialize_timer_update(597, TIMER_ABILITY_COOLDOWN, 100, 5.0, 12345.0);
    assert_eq!(data.len(), 21);
    let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(id, 597);
    assert_eq!(data[4], 2); // TIMER_ABILITY_COOLDOWN
    let source = i32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    assert_eq!(source, 100);
}

#[test]
fn pseudo_random_seed_is_deterministic() {
    let s1 = pseudo_random_seed(1, 597, 0);
    let s2 = pseudo_random_seed(1, 597, 0);
    assert_eq!(s1, s2);
}

#[test]
fn pseudo_random_seed_varies_with_inputs() {
    // Each input dimension must perturb the seed independently — otherwise
    // rapid-fire shots on the same target (entity, ability fixed; only
    // sequence varies) would share suspiciously similar beta samples.
    let s1 = pseudo_random_seed(1, 597, 0);
    let s2 = pseudo_random_seed(2, 597, 0);
    let s3 = pseudo_random_seed(1, 598, 0);
    let s4 = pseudo_random_seed(1, 597, 1);
    assert_ne!(s1, s2);
    assert_ne!(s1, s3);
    assert_ne!(s1, s4);
}

#[test]
fn pseudo_random_seed_avoids_obvious_collisions() {
    // Spot-check: sequential triples must produce well-separated seeds, not
    // a tight cluster that the beta sampler would map to similar samples.
    use std::collections::HashSet;
    let mut seen: HashSet<u64> = HashSet::new();
    for entity_id in 1..=10 {
        for ability_id in 1..=10 {
            for seq in 0..10 {
                let s = pseudo_random_seed(entity_id, ability_id, seq);
                assert!(seen.insert(s), "collision at ({entity_id}, {ability_id}, {seq})");
            }
        }
    }
}

/// Stage E: firing an ability with `required_ammo > 0` must decrement the
/// active slot's `current_ammo`, mirror that into the AmmoSlot{N} stat,
/// mark the slot dirty for batched persistence, and emit `onStatUpdate`
/// (method 20) carrying the new AmmoSlot value.
#[tokio::test]
async fn consume_ammo_writes_ammoslot_stat_and_marks_dirty() {
    use cimmeria_entity::abilities::AbilityDef;
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::AMMO_SLOT_1;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#).unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

    // A ranged ability with required_ammo = 1, no warmup, short cooldown.
    // event_set_id = None → no onSequence sends, keeping rx noise down.
    let ability_id = 4242;
    mgr.ability_defs.insert(ability_id, AbilityDef {
        ability_id,
        name: "test_fire".to_string(),
        cooldown: 0.5,
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
    });

    // Player setup: bandolier slot 0 holds a 30-round magazine, full.
    // is_player must be true so `send_entity_method` routes the
    // onStatUpdate via the player path (EntityMethodCall).
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.abilities.add_ability(ability_id);
        e.bandolier_items.insert(0, BandolierItem {
            item_id: 1,
            clip_size: 30,
            default_ammo_type: 2,
            current_ammo: 30,
            cur_ammo_type: 2,
        });
        if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
            stat.update(0, 30, 30);
            stat.clear_dirty();
        }
    }

    let (tx, mut rx) = mpsc::channel(32);
    // target_id = 0 → no target path (skips combat resolution and
    // damage-side stat sends). The consume + ammo-stat-flush block at
    // the no-target branch still runs.
    handle_use_ability(1, ability_id, 0, &tx, &mut mgr).await;

    // ── Entity-state assertions ─────────────────────────────────────
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(entity.bandolier_items[&0].current_ammo, 29, "current_ammo should decrement by required_ammo");
    assert_eq!(entity.stats.get(AMMO_SLOT_1).unwrap().cur, 29, "AmmoSlot1 stat should mirror current_ammo");
    assert!(entity.bandolier_ammo_dirty.contains(&0), "active slot should be marked dirty for batched persist");

    // ── Wire-message assertions ─────────────────────────────────────
    // Drain rx and find the onStatUpdate (method 20). Other messages
    // (onTimerUpdate=12, onStateFieldUpdate=19) may also arrive.
    let mut found_stat_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } = msg {
            if method_index == 20 && entity_id == 1 {
                // Stat list payload: count:u32, then per-stat (id:i32, min:i32, cur:i32, max:i32).
                assert!(args.len() >= 4, "stat update payload too short");
                let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert!(count >= 1, "stat update should contain at least one entry");
                // Walk the entries to find AmmoSlot1.
                let mut found_ammo = false;
                for i in 0..count as usize {
                    let off = 4 + i * 16;
                    let stat_id = i32::from_le_bytes([args[off], args[off+1], args[off+2], args[off+3]]);
                    let cur = i32::from_le_bytes([args[off+8], args[off+9], args[off+10], args[off+11]]);
                    if stat_id == AMMO_SLOT_1 {
                        assert_eq!(cur, 29, "onStatUpdate should report AmmoSlot1.cur = 29");
                        found_ammo = true;
                    }
                }
                assert!(found_ammo, "onStatUpdate did not contain AmmoSlot1 entry");
                found_stat_update = true;
            }
        }
    }
    assert!(found_stat_update, "expected an EntityMethodCall(method=20, onStatUpdate) carrying AmmoSlot1");
}
