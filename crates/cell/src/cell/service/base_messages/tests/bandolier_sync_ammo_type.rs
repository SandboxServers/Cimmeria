use super::*;

/// **Regression guard: in-game equip must emit AmmoTypeId.**
/// Right-click / drag equip into the bandolier must emit
/// `onEntityProperty(AmmoTypeId, cur_ammo_type)` so the client's
/// fire-animation gate opens immediately. Without this the client
/// retains `AmmoTypeId=0` (no ammo) and the weapon-shot ability plays
/// no animation until the player manually swaps to another bandolier
/// slot and back (which fires `handle_request_active_slot_change`, the
/// path that already emits AmmoTypeId).
///
/// Pinned via the wire packet: collect every `onEntityProperty` emit
/// from the sync, find the one carrying `prop_id == 3`
/// (`GENERICPROPERTY_AmmoTypeId`), assert the value matches the
/// equipped weapon's `cur_ammo_type`. Reverting the emit in
/// `handle_sync_bandolier_items` (the `active_slot_gained_weapon`
/// branch) makes this assertion fail.
#[tokio::test]
async fn sync_bandolier_items_active_slot_gained_weapon_emits_ammo_type_id() {
    use crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID;

    const AMMO_TYPE: i32 = 2;

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
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = true;
        e.active_bandolier_slot = 0;
        e.bandolier_items.clear();
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_EQUIP), 1872);

    let item = BandolierItem {
        instance_id: 0,
        item_id: 55,
        clip_size: 15,
        default_ammo_type: AMMO_TYPE,
        current_ammo: 0,
        cur_ammo_type: AMMO_TYPE,
    };

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, item)],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Scan for the AmmoTypeId entity-property emit. The wire payload
    // is 8 bytes: [prop_id: i32 LE][value: i32 LE]. Pin both the
    // packet and its content — a refactor that emits the right
    // method but wrong prop_id (or right prop_id but wrong value)
    // would still let the bug surface in playtest.
    let mut saw_ammo_type_id = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY
                && args.len() == 8
            {
                let prop_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
                let value = i32::from_le_bytes(args[4..8].try_into().unwrap());
                if prop_id == GENERICPROPERTY_AMMO_TYPE_ID {
                    assert_eq!(
                        value, AMMO_TYPE,
                        "AmmoTypeId emit must carry the equipped weapon's cur_ammo_type"
                    );
                    saw_ammo_type_id = true;
                }
            }
        }
    }
    assert!(
        saw_ammo_type_id,
        "in-game equip into bandolier must emit onEntityProperty(AmmoTypeId, cur_ammo_type) — \
         without this the client's fire-animation gate stays closed (AmmoTypeId=0 = no ammo) \
         until the player manually swaps bandolier slots and back",
    );
}

/// Parallel guard for the **unequip** path of `SyncBandolierItems` —
/// when the active slot loses its weapon, the client's AmmoTypeId
/// must reset to 0 (mirrors python `SGWPlayer.py:522`'s
/// `activeItem.ammoType if activeItem else 0`). Without this, the
/// client keeps the just-removed weapon's ammo type, which surfaces
/// as a stale ammo-bar indicator after unequip.
#[tokio::test]
async fn sync_bandolier_items_active_slot_lost_weapon_emits_ammo_type_id_zero() {
    use crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID;

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
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false;
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 55,
                clip_size: 15,
                default_ammo_type: 2,
                current_ammo: 7,
                cur_ammo_type: 2,
            },
        );
        e.combat_exit_at = Some(std::time::Instant::now());
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let mut saw_zero_ammo_type = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY
                && args.len() == 8
            {
                let prop_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
                let value = i32::from_le_bytes(args[4..8].try_into().unwrap());
                if prop_id == GENERICPROPERTY_AMMO_TYPE_ID {
                    assert_eq!(
                        value, 0,
                        "unequip must emit AmmoTypeId=0 (no weapon equipped) — \
                         keeping the prior weapon's ammo type leaves a stale \
                         ammo-bar indicator on the client"
                    );
                    saw_zero_ammo_type = true;
                }
            }
        }
    }
    assert!(
        saw_zero_ammo_type,
        "unequip from active bandolier slot must emit onEntityProperty(AmmoTypeId, 0)",
    );
}

/// Negative case: a resync where the active slot's item is unchanged
/// must NOT emit AmmoTypeId. Without this guard, every stash-slot
/// shuffle would re-send AmmoTypeId — wire spam, and (worse) could
/// stomp a `requestAmmoChange` that the player issued in the same
/// tick window.
#[tokio::test]
async fn sync_bandolier_items_active_slot_unchanged_does_not_emit_ammo_type_id() {
    use crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID;

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
        e.weapon_holstered = false;
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 55,
                clip_size: 15,
                default_ammo_type: 2,
                current_ammo: 7,
                cur_ammo_type: 2,
            },
        );
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(
                0,
                BandolierItem {
                    instance_id: 0,
                    item_id: 55,
                    clip_size: 15,
                    default_ammo_type: 2,
                    current_ammo: 7,
                    cur_ammo_type: 2,
                },
            )],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY
                && args.len() == 8
            {
                let prop_id = i32::from_le_bytes(args[0..4].try_into().unwrap());
                assert_ne!(
                    prop_id, GENERICPROPERTY_AMMO_TYPE_ID,
                    "active-slot-unchanged resync must NOT emit AmmoTypeId — \
                     stash-slot shuffles shouldn't re-broadcast the active weapon's \
                     ammo subtype, and emitting unconditionally could stomp an \
                     in-flight requestAmmoChange",
                );
            }
        }
    }
}
