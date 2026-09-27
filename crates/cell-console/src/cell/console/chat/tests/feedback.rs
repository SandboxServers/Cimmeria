use super::super::*;
use super::decode_on_player_communication;

/// Unsupported channels (team/squad/command/officer/tell) are not
/// distributed on the cell -- but unlike a silent drop, the sender must
/// get a feedback line so a chat message never just vanishes (project
/// rule: every button press gets visible feedback). No witness ever
/// receives anything for these channels.
#[tokio::test]
async fn non_cell_channel_feeds_back_to_sender_only() {
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
        e.witnesses.insert(cimmeria_common::EntityId(2));
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    let engine = ChainEngine::new();

    // Tell channel is not distributed on the cell, but the sender must
    // still hear back -- not dead silence.
    handle_chat_message(1, "Bob", 0, CHAN_TELL, "Hi", &tx, &mut mgr, &engine).await;

    let msg = rx
        .try_recv()
        .expect("sender must receive a feedback line, not silence");
    let CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    } = msg
    else {
        panic!("expected EntityMethodCall");
    };
    assert_eq!(entity_id, 1, "feedback must go to the sender only");
    assert_eq!(method_index, ON_PLAYER_COMMUNICATION);
    let (flags, channel, text) = decode_on_player_communication(&args);
    assert_eq!(flags, 0);
    assert_eq!(
        channel, CHAN_FEEDBACK,
        "feedback rides the registered feedback channel"
    );
    assert!(
        text.contains("not supported yet"),
        "feedback text must explain the channel is unsupported, got: {text}"
    );
    assert!(
        text.contains(&CHAN_TELL.to_string()),
        "feedback text must name the offending channel id, got: {text}"
    );

    // No second message -- in particular, the witness (entity 2) never
    // hears about an unsupported-channel attempt.
    assert!(
        rx.try_recv().is_err(),
        "witness must not receive anything for an unsupported channel"
    );
}

/// The server channel is system-broadcast-only
/// (`CHANNEL_FLAG_DisallowPlayerMessages` in the legacy
/// `ChatChannelManager`) -- a player's own chat send on it must never be
/// silently dropped, and must never reach witnesses as if it were a
/// normal broadcast.
#[tokio::test]
async fn server_channel_rejects_player_speech_with_feedback() {
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
        e.witnesses.insert(cimmeria_common::EntityId(2));
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_chat_message(
        1,
        "Bob",
        0,
        CHAN_SERVER,
        "is this thing on?",
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let msg = rx
        .try_recv()
        .expect("sender must receive a feedback line explaining the rejection");
    let CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    } = msg
    else {
        panic!("expected EntityMethodCall");
    };
    assert_eq!(entity_id, 1, "feedback must go to the sender only");
    assert_eq!(method_index, ON_PLAYER_COMMUNICATION);

    let (flags, channel, text) = decode_on_player_communication(&args);
    assert_eq!(flags, 0);
    assert_eq!(
        channel, CHAN_FEEDBACK,
        "feedback rides the registered feedback channel"
    );
    assert!(
        text.contains("system messages only"),
        "feedback text must explain the server channel is system-only, got: {text}"
    );

    // No second message: the witness must never see a player message
    // that was rejected as system-only, nor a broadcast of it.
    assert!(
        rx.try_recv().is_err(),
        "witness must not receive anything for a rejected server-channel send"
    );
}
