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

    // A BandolierAmmoUpdate must be sent exactly once while handling destroy.
    let mut flush_count = 0u32;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::BandolierAmmoUpdate { player_id, .. } = msg {
            assert_eq!(player_id, 100);
            flush_count += 1;
        }
    }
    assert_eq!(
        flush_count, 1,
        "DestroyEntity must flush exactly one BandolierAmmoUpdate before tearing down"
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

#[tokio::test]
async fn minigame_result_victory_fires_on_victory_chains() {
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
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
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
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code: 1, // victory
            on_victory_chains: vec![9999],
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
        .expect("victory chain must fire and produce onStatUpdate");
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
async fn minigame_result_defeat_does_not_fire_chains() {
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
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
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
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code: 0, // defeat
            on_victory_chains: vec![9999],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "defeat (result_code != 1) must not fire victory chains"
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

/// Regression guard for the disconnect-mid-trade hook in
/// `BaseToCellMsg::DestroyEntity`. When a player disconnects while
/// mid-trade, the surviving partner MUST receive
/// `onTradeResults(Cancelled)` and have their session state cleared.
/// Without this hook the partner is left with a dangling
/// `trade_partner_entity_id` pointing at a destroyed entity — every
/// subsequent state-machine step against that ghost session crashes
/// or silently fails.
///
/// Python relied on BigWorld GC for this teardown; Rust has to hook
/// it explicitly. The hook lives in
/// `cell::cell_methods::player::trade::cancel_trade_on_disconnect`
/// and is called from BOTH `DestroyEntity` and `DisconnectEntity`
/// arms — this test pins the `DestroyEntity` arm, the next pins
/// `DisconnectEntity`.
///
/// Revert-verifier: deleting the
/// `cell_methods::player::trade::cancel_trade_on_disconnect(...).await`
/// call from the DestroyEntity arm causes the surviving partner's
/// trade_partner_entity_id to remain `Some(1)` after the destroy,
/// and no Cancelled packet to fire. Both assertions fail.
#[tokio::test]
async fn destroy_entity_cancels_in_flight_trade_with_surviving_partner() {
    use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Hand-wire a Locked-and-known trade session on both sides.
    for &eid in &[1u32, 2u32] {
        if let Some(e) = mgr.get_entity_mut(eid) {
            e.is_player = true;
            e.player_id = Some(eid as i32 * 100);
            e.trade_partner_entity_id = Some(if eid == 1 { 2 } else { 1 });
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_NONE,
            });
        }
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Entity 1 is gone; entity 2 (the surviving partner) must have its
    // trade state cleared.
    assert!(mgr.get_entity(1).is_none(), "entity 1 must be destroyed");
    let survivor = mgr
        .get_entity(2)
        .expect("entity 2 (surviving partner) must still exist");
    assert!(
        survivor.trade_partner_entity_id.is_none(),
        "surviving partner's trade_partner_entity_id MUST be cleared on \
         disconnect. A regression that drops the trade-cancellation hook \
         from DestroyEntity leaves this pointing at a destroyed entity."
    );
    assert!(
        survivor.trade_proposal.is_none(),
        "surviving partner's trade_proposal must also be cleared"
    );

    // The surviving partner (entity 2) must have received an
    // onTradeResults(Cancelled) — that's the wire-level confirmation
    // their client needs to close the trade dialog. The disconnecting
    // entity (1) may or may not get one too, but we don't care — it's
    // about to be destroyed.
    use crate::cell::client_methods::player::ON_TRADE_RESULTS;
    let mut survivor_got_cancelled = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS && args.len() >= 8 {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    survivor_got_cancelled = true;
                }
            }
        }
    }
    assert!(
        survivor_got_cancelled,
        "surviving partner MUST receive onTradeResults(Cancelled=2) — \
         distinct from user-initiated cancel which sends Completed (1). \
         The disconnect path is a fault path; the surviving partner needs \
         the dialog dismissed without ambiguity about whose fault it was."
    );
}

/// Symmetric guard for the `DisconnectEntity` arm. Many disconnect
/// paths reach the cell via `DisconnectEntity` first (the underlying
/// `space_mgr.disconnect_entity` then calls `destroy_entity`
/// internally), so the trade-cancellation hook must fire BEFORE that
/// internal destroy — otherwise the entity is gone by the time the
/// hook runs and the partner cleanup is a silent no-op.
///
/// Revert-verifier: removing the
/// `cancel_trade_on_disconnect(...).await` call from the
/// DisconnectEntity arm (or moving it AFTER
/// `space_mgr.disconnect_entity`) makes this test fail because the
/// surviving partner's trade state stays populated.
#[tokio::test]
async fn disconnect_entity_cancels_in_flight_trade_with_surviving_partner() {
    use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    for &eid in &[1u32, 2u32] {
        if let Some(e) = mgr.get_entity_mut(eid) {
            e.is_player = true;
            e.player_id = Some(eid as i32 * 100);
            e.trade_partner_entity_id = Some(if eid == 1 { 2 } else { 1 });
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_NONE,
            });
        }
    }
    // Mark entity 1 as connected so the disconnect path actually runs.
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Entity 2 (surviving partner) must have its trade state cleared.
    let survivor = mgr
        .get_entity(2)
        .expect("entity 2 (surviving partner) must still exist");
    assert!(
        survivor.trade_partner_entity_id.is_none(),
        "DisconnectEntity arm MUST call cancel_trade_on_disconnect for \
         the disconnecting entity BEFORE the internal destroy fires. \
         If the hook is dropped or runs in the wrong order, the partner's \
         trade state stays populated (pointing at a destroyed entity)."
    );
    assert!(
        survivor.trade_proposal.is_none(),
        "surviving partner's trade_proposal must also be cleared"
    );

    use crate::cell::client_methods::player::ON_TRADE_RESULTS;
    let mut survivor_got_cancelled = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS && args.len() >= 8 {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    survivor_got_cancelled = true;
                }
            }
        }
    }
    assert!(
        survivor_got_cancelled,
        "surviving partner MUST receive onTradeResults(Cancelled) via \
         the DisconnectEntity arm too"
    );
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
