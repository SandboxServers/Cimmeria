use super::*;

/// `InventoryItemMoveApplied` with target=bandolier (3) and source≠3
/// fires the `OnItemEquipped` content event. Pin the dispatch path
/// end-to-end by registering a chain that reacts with
/// `IncrementCounter` and asserting the entity's counter moved.
#[tokio::test]
async fn item_move_applied_into_bandolier_fires_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9100,
        name: "test: bandolier-equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: Some(55) },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_bandolier_equip".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 1, // backpack
            target_container_id: 3, // bandolier
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert_eq!(
        entity.counters.get("test_bandolier_equip"),
        Some(&1),
        "move into bandolier (target=3, source≠3) must fire OnItemEquipped",
    );
}

/// A move WITHIN the bandolier (source=3, target=3 — the player
/// reordering their bandolier slots) must NOT fire `OnItemEquipped`.
/// Without this guard, every drag between bandolier slots would
/// re-fire equip chains and re-grant whatever they grant.
#[tokio::test]
async fn item_move_within_bandolier_does_not_fire_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9101,
        name: "test: any equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: None },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_within_bandolier".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 3, // bandolier → bandolier
            target_container_id: 3,
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("test_within_bandolier"),
        "bandolier-internal move must not fire OnItemEquipped; got {:?}",
        entity.counters,
    );
}

/// A move OUT of the bandolier (source=3, target=1) must not fire
/// `OnItemEquipped` either — that's an unequip, not an equip.
#[tokio::test]
async fn item_move_out_of_bandolier_does_not_fire_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9102,
        name: "test: any equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: None },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_unequip_path".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 3, // bandolier → backpack
            target_container_id: 1,
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("test_unequip_path"),
        "unequip (source=3, target=1) must not fire OnItemEquipped",
    );
}

#[tokio::test]
async fn item_used_fires_on_item_use_content_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        // Set HP below max so ChangeStat +10 actually advances
        if let Some(h) = e.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 50, 100);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 5001,
        name: "test-item-use-chain".into(),
        enabled: true,
        trigger: Trigger::OnItemUse { item_id: 42 },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::ItemUsed {
            entity_id: 1,
            instance_id: 1001,
            type_id: 42,
            target_id: 0,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // ChangeStat emits onStatUpdate via the executor
    let msg = rx
        .try_recv()
        .expect("ItemUsed must fire OnItemUse chain and produce onStatUpdate");
    match msg {
        crate::cell::messages::CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
        }
        other => panic!("expected EntityMethodCall(onStatUpdate), got {other:?}"),
    }
}

#[tokio::test]
async fn item_used_drops_event_when_no_player_id() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = false; // NPC — no player_id
    }
    mgr.connect_entity(1);

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::ItemUsed {
            entity_id: 1,
            instance_id: 1001,
            type_id: 42,
            target_id: 0,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "entity without player_id must drop ItemUsed event silently"
    );
}
