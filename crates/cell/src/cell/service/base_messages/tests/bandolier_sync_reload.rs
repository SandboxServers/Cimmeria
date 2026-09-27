use super::*;

/// `SyncBandolierItems` is the dispatch path for drag/right-click equip
/// from the inventory window. With the `reloadOnActivate` option set,
/// equipping a weapon with a partial clip into the active slot must
/// automatically queue a reload — the same way the
/// `UpdateBandolierItem` path does for chain-engine grants.
///
/// Bug shape this catches: removing the
/// `maybe_trigger_reload_on_activate` call from the
/// `active_slot_gained_weapon` branch of `handle_sync_bandolier_items`.
/// Without it, players who toggled reloadOnActivate in the menu and
/// drag a partial-clip weapon into their active slot have the weapon
/// equipped but not topped up — works for chain grants only.
#[tokio::test]
async fn sync_bandolier_items_active_slot_gained_partial_clip_triggers_reload_on_activate() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    // Reload ability def so handle_reload's warmup/cooldown lookup works.
    mgr.ability_defs.insert(
        596, // ABILITY_RELOAD_WEAPON
        cimmeria_entity::abilities::AbilityDef {
            ability_id: 596,
            is_ranged: false,
            min_range: 0,
            name: "Reload".into(),
            warmup: 1.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // already drawn — skip Phase A
        e.active_bandolier_slot = 0;
        e.bandolier_items.clear();
        // User has opted into reload-on-activate.
        e.system_options.reload_on_activate = true;
    }
    mgr.connect_entity(1);

    // The item to drag into slot 0 — partial clip (10 of 15).
    let item = BandolierItem {
        instance_id: 0,
        item_id: 55,
        clip_size: 15,
        default_ammo_type: 2,
        current_ammo: 10,
        cur_ammo_type: 2,
    };

    let (tx, _rx) = mpsc::channel(32);
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

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "sync_bandolier_items into active slot with reloadOnActivate=on \
         and a partial clip must start a reload — drag/right-click equip \
         path must match the chain-grant path's reload-on-activate behavior",
    );
}

/// Negative guard: reloadOnActivate=off must NOT reload on drag/equip
/// even when the dragged weapon has a partial clip.
#[tokio::test]
async fn sync_bandolier_items_with_option_off_does_not_reload_on_activate() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.ability_defs.insert(
        596,
        cimmeria_entity::abilities::AbilityDef {
            ability_id: 596,
            is_ranged: false,
            min_range: 0,
            name: "Reload".into(),
            warmup: 1.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.weapon_holstered = false;
        e.active_bandolier_slot = 0;
        e.bandolier_items.clear();
        // Default — option off; the user has NOT opted in.
        assert!(!e.system_options.reload_on_activate);
    }
    mgr.connect_entity(1);

    let item = BandolierItem {
        instance_id: 0,
        item_id: 55,
        clip_size: 15,
        default_ammo_type: 2,
        current_ammo: 10,
        cur_ammo_type: 2,
    };

    let (tx, _rx) = mpsc::channel(32);
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

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "sync_bandolier_items must NOT auto-reload when the option is off",
    );
}
