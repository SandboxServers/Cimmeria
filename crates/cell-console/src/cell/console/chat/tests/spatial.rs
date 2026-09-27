use super::super::*;

#[tokio::test]
async fn broadcast_to_nonexistent_entity_is_noop() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let engine = ChainEngine::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);

    handle_chat_message(999, "Bob", 0, CHAN_SAY, "Hello", &tx, &mut mgr, &engine).await;

    // No messages should be sent
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn broadcast_say_to_witnesses() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    // Create two players near each other
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.connect_entity(2);

    // Manually add witness relationships (normally done by AoI tick)
    if let Some(e) = mgr.get_entity_mut(1) {
        e.witnesses.insert(cimmeria_common::EntityId(2));
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();

    handle_chat_message(
        1,
        "Alice",
        0,
        CHAN_SAY,
        "Hello world",
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Should get 2 messages: one for witness (entity 2) + one for sender (entity 1)
    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        msgs.push(msg);
    }
    assert_eq!(msgs.len(), 2);

    // Check the first is to witness entity 2
    match &msgs[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(*entity_id, 2);
            assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
        }
        _ => panic!("Expected EntityMethodCall"),
    }

    // Check the second is to sender entity 1
    match &msgs[1] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(*entity_id, 1);
            assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
        }
        _ => panic!("Expected EntityMethodCall"),
    }
}

/// Regression guard: an NPC in the sender's own AoI (`entity.witnesses`
/// stores what the sender *sees*, not who sees the sender — see
/// `SpaceManager::get_witnesses_of`) must never become a chat broadcast
/// target. Before the fix, `broadcast_to_witnesses` dispatched an
/// `EntityMethodCall { entity_id: <npc_id>, .. }` for every NPC in
/// range; base then tried to resolve that NPC id through
/// `entity_to_addr` (which only ever holds player entries) and logged
/// `AoI reliable: no client addr for witness -- packet dropped`
/// (`reason = entity_to_addr_miss`) once per NPC, per chat line, for
/// every NPC near a talking player. Reverting the `is_player` filter in
/// `broadcast_to_witnesses` makes this test fail by emitting an
/// `EntityMethodCall` addressed to the NPC.
#[tokio::test]
async fn broadcast_say_skips_npc_witnesses() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    // One player (the speaker), one other player (a real witness), and
    // one NPC that happens to be in the speaker's AoI too.
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(100008, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.connect_entity(2);
    // Entity 100008 is never `connect_entity`'d, so it stays an NPC
    // (`is_player == false`) exactly like a Castle spawn-set mob.

    // Manually add witness relationships (normally done by AoI tick) --
    // the speaker sees both the other player AND the NPC.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.witnesses.insert(cimmeria_common::EntityId(2));
        e.witnesses.insert(cimmeria_common::EntityId(100008));
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();

    handle_chat_message(
        1,
        "Alice",
        0,
        CHAN_SAY,
        "Hello world",
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        msgs.push(msg);
    }

    // Exactly 2 messages: the real player witness (2) + the sender echo
    // (1). The NPC (100008) must never appear as an `entity_id`.
    assert_eq!(
        msgs.len(),
        2,
        "NPC witness must not receive its own EntityMethodCall: {msgs:?}"
    );
    for msg in &msgs {
        if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            assert_ne!(
                *entity_id, 100008,
                "chat must never target an NPC id -- it has no client and \
                 entity_to_addr resolution always misses for it"
            );
        }
    }
}
