//! The bandolier ammo counter (`AmmoSlot{N}`, stats 49-53) after a grant,
//! an equip and a slot swap.
//!
//! The client's counter reads the stat, not the `BandolierItem`, so every
//! path that changes a slot's item must mirror its count into the stat and
//! push `onStatUpdate`. OD-CS13: a granted gun shows 0 rounds; equipping and
//! swapping never change a gun's count.

use super::*;

/// The `(min, cur, max)` of `stat_id` in an `onStatUpdate` payload:
/// `u32 count` then `count` x `(id, min, cur, max)` as LE i32.
fn stat_entry(payload: &[u8], stat_id: i32) -> Option<(i32, i32, i32)> {
    let read = |at: usize| i32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    let count = u32::from_le_bytes(payload[0..4].try_into().unwrap()) as usize;
    assert_eq!(
        payload.len(),
        4 + count * 16,
        "onStatUpdate is count + 16-byte entries"
    );
    (0..count)
        .map(|i| 4 + i * 16)
        .find(|&at| read(at) == stat_id)
        .map(|at| (read(at + 4), read(at + 8), read(at + 12)))
}

/// The last `(min, cur, max)` of `stat_id` any `onStatUpdate` in `rx` carried.
fn last_sent(rx: &mut mpsc::Receiver<CellToBaseMsg>, stat_id: i32) -> Option<(i32, i32, i32)> {
    let mut sent = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                sent = stat_entry(&args, stat_id).or(sent);
            }
        }
    }
    sent
}

/// One connected player, active slot 0, clean stats.
fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.active_bandolier_slot = 0;
        e.weapon_holstered = false;
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);
    mgr
}

fn gun(instance_id: i32, item_id: i32, clip_size: i32, current_ammo: i32) -> BandolierItem {
    BandolierItem {
        instance_id,
        item_id,
        clip_size,
        default_ammo_type: 1,
        current_ammo,
        cur_ammo_type: 1,
    }
}

fn ammo_stat(mgr: &SpaceManager, slot: i32) -> (i32, i32, i32) {
    let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot;
    let s = mgr.get_entity(1).unwrap().stats.get(stat_id).unwrap();
    (s.min, s.cur, s.max)
}

/// The base's equip epilogue sends `UpdateBandolierItem` for a granted gun
/// with `current_ammo` 0 (OD-CS13). The cell must mirror that into
/// `AmmoSlot{N}` as `(0, 0, clip)` and push it, so the counter shows an
/// empty 15-round clip rather than no weapon.
///
/// Bug shape: before CS-01b this handler never touched the stat, so a GM
/// give or loot pickup into the bandolier left the counter blank (max 0).
/// Removing the `stat.update` or the push fails here.
#[tokio::test]
async fn update_bandolier_item_reports_the_granted_guns_empty_clip() {
    const SLOT: i32 = 1;
    const CLIP: i32 = 15; // Standard Pistol 55

    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: SLOT,
            item: gun(9, 55, CLIP, 0),
            make_active: false,
        },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    assert_eq!(
        ammo_stat(&mgr, SLOT),
        (0, 0, CLIP),
        "AmmoSlot mirrors 0 of 15"
    );
    assert_eq!(
        last_sent(&mut rx, cimmeria_entity::stats::AMMO_SLOT_1 + SLOT),
        Some((0, 0, CLIP)),
        "onStatUpdate must carry AmmoSlot{{N}} = (0, 0, clip) for the granted gun",
    );
}

/// Equipping a partly (9/15) or fully (30/30) loaded gun through
/// `SyncBandolierItems` keeps its count, on the item and on the counter.
#[tokio::test]
async fn equip_keeps_a_partly_or_fully_loaded_clip() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(32);
    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, gun(11, 55, 15, 9)), (1, gun(12, 21, 30, 30))],
        },
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.bandolier_items[&0].current_ammo, 9,
        "partly loaded pistol keeps 9"
    );
    assert_eq!(e.bandolier_items[&1].current_ammo, 30, "full SMG keeps 30");
    assert_eq!(ammo_stat(&mgr, 0), (0, 9, 15));
    assert_eq!(ammo_stat(&mgr, 1), (0, 30, 30));
    assert_eq!(
        last_sent(&mut rx, cimmeria_entity::stats::AMMO_SLOT_1),
        Some((0, 9, 15)),
        "the counter shows the equipped pistol's 9 rounds",
    );
}

/// Swapping the active slot between two loaded guns keeps both counts.
/// The swap first plays the holster choreography and defers; the second
/// call is the tick's re-entry that performs the change.
#[tokio::test]
async fn slot_swap_keeps_both_clips() {
    let mut mgr = make_mgr();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(0, gun(11, 55, 15, 7));
        e.bandolier_items.insert(1, gun(12, 21, 30, 30));
    }
    let (tx, _rx) = mpsc::channel(64);
    // bag 3, wire slot 2 (server slot 1).
    let mut args = Vec::new();
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());
    for _ in 0..2 {
        crate::cell::cell_methods::inventory::handle_request_active_slot_change(
            1, &args, &tx, &mut mgr,
        )
        .await;
    }

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(e.active_bandolier_slot, 1, "the swap landed on slot 1");
    assert_eq!(
        e.bandolier_items[&0].current_ammo, 7,
        "the holstered pistol keeps 7"
    );
    assert_eq!(
        e.bandolier_items[&1].current_ammo, 30,
        "the drawn SMG keeps 30"
    );
}
