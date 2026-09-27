use super::super::*;

/// A GM's `.`-command is consumed by the console and never broadcast to
/// witnesses (never appears in others' chat).
#[tokio::test]
async fn gm_dot_command_is_intercepted_not_broadcast() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.connect_entity(2);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.access_level = 2; // GameMaster
        e.witnesses.insert(cimmeria_common::EntityId(2));
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);

    handle_chat_message(1, "Gm", 0, CHAN_SAY, ".players", &tx, &mut mgr, &engine).await;

    // Witness (entity 2) must receive NOTHING — the command was consumed.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            assert_ne!(entity_id, 2, "GM .-command must not broadcast to witnesses");
        }
    }
}

/// A non-GM's `.`-text is ordinary chat and DOES broadcast.
#[tokio::test]
async fn non_gm_dot_text_is_normal_chat() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.connect_entity(2);
    if let Some(e) = mgr.get_entity_mut(1) {
        // access_level stays 0 (Player)
        e.witnesses.insert(cimmeria_common::EntityId(2));
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);

    handle_chat_message(1, "Joe", 0, CHAN_SAY, ".hello", &tx, &mut mgr, &engine).await;

    // Witness (entity 2) should receive the chat broadcast.
    let mut witness_got_chat = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            if entity_id == 2 {
                witness_got_chat = true;
            }
        }
    }
    assert!(
        witness_got_chat,
        "non-GM .-text must broadcast as normal chat"
    );
}
