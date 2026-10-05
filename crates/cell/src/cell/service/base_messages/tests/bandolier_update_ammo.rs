use super::*;

/// The player's AmmoSlot{N} `(min, cur, max)` entry in an `onStatUpdate`
/// payload: `u32 count` then `count` × `(id, min, cur, max)` as LE i32.
fn ammo_slot_entry(payload: &[u8], stat_id: i32) -> Option<(i32, i32, i32)> {
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

/// Class Start v6 L4: the base's equip epilogue sends `UpdateBandolierItem`
/// with the loaded clip of a granted firearm, and the cell must mirror it
/// into AmmoSlot{N} and push it in `onStatUpdate`, because the client's
/// bandolier counter reads the stat, not the item.
///
/// Bug shape: before CS-01b this handler never touched the stat, so a GM
/// give or a loot pickup into the bandolier (no chain to seed the stat
/// first) showed no rounds although the cell held a full clip. Removing the
/// `stat.update` (or the push) in `handle_update_bandolier_item` fails here.
#[tokio::test]
async fn update_bandolier_item_reports_the_loaded_clip_in_onstatupdate() {
    const SLOT: i32 = 1;
    const CLIP: i32 = 15; // Standard Pistol 55

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
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let item = BandolierItem {
        instance_id: 9,
        item_id: 55,
        clip_size: CLIP,
        default_ammo_type: 1,
        current_ammo: BandolierItem::granted_ammo(CLIP),
        cur_ammo_type: 1,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: SLOT,
            item,
            make_active: false,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + SLOT;
    let stat = mgr.get_entity(1).unwrap().stats.get(stat_id).unwrap();
    assert_eq!(
        (stat.min, stat.cur, stat.max),
        (0, CLIP, CLIP),
        "AmmoSlot{{N}} mirrors the granted slot's loaded clip",
    );

    let mut sent = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STAT_UPDATE {
                sent = sent.or(ammo_slot_entry(&args, stat_id));
            }
        }
    }
    assert_eq!(
        sent,
        Some((0, CLIP, CLIP)),
        "onStatUpdate must carry AmmoSlot{{N}} = (0, clip, clip) for the granted weapon",
    );
}
