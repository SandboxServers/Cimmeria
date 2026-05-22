use super::*;

#[tokio::test]
async fn destroy_entity_flushes_dirty_bandolier_and_destroys_entity() {
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

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // A BandolierAmmoUpdate must be sent while handling destroy.
    let mut got_flush = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::BandolierAmmoUpdate { player_id, .. } = msg {
            assert_eq!(player_id, 100);
            got_flush = true;
        }
    }
    assert!(
        got_flush,
        "DestroyEntity must flush dirty bandolier ammo before tearing down"
    );
    assert!(
        mgr.get_entity(1).is_none(),
        "entity must be destroyed after flush"
    );
}

#[tokio::test]
async fn entity_move_updates_position_in_space_manager() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 1,
            position: [10.0, 20.0, 30.0],
            direction: [0, 0, 0],
            velocity: [1.0, 2.0, 3.0],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(entity.position.x, 10.0);
    assert_eq!(entity.position.y, 20.0);
    assert_eq!(entity.position.z, 30.0);
    assert_eq!(entity.velocity, [1.0, 2.0, 3.0]);
}

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

/// `BaseToCellMsg::AdvanceRingDestination` is the cross-world ring
/// transport's deferred-load callback. After the source ring's
/// `Effect::TeleportCrossWorld` fires, the destination ring's FSM
/// sits in `RemoteLoadWait` until base sends this message back
/// (after the destination world's `onClientReady`). The handler
/// must forward to `ring_transport::handle_remote_player_loaded`
/// without crashing when the destination ring isn't loaded — the
/// integration-shaped fail-soft path that lets a ring not pre-loaded
/// in this cell instance be a quiet no-op rather than a panic.
#[tokio::test]
async fn advance_ring_destination_forwards_without_panic_when_ring_absent() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Player is on Castle but no ring transporter region 34 is
    // loaded — handle_remote_player_loaded must short-circuit
    // gracefully rather than panic on the missing region lookup.
    mgr.create_entity(2, "Castle", [466.365, 70.397, 991.466], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::AdvanceRingDestination {
            entity_id: 2,
            region_id: 34,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // The entity must still exist — AdvanceRingDestination doesn't
    // tear anything down on its own; it just records a load on the
    // destination ring's FSM (which is a no-op when the ring isn't
    // loaded). Pinning post-state-equality guards against a future
    // refactor that accidentally couples the dispatcher to entity
    // teardown.
    assert!(
        mgr.get_entity(2).is_some(),
        "AdvanceRingDestination must not destroy the recipient entity"
    );
}
