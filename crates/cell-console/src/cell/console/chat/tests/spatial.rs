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

    // `say` is echoed locally by the client (chat-speaker-echo.md), so the
    // server must send exactly 1 message: the witness (entity 2), never the
    // sender (entity 1). Sending both is the double-echo bug.
    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        msgs.push(msg);
    }
    assert_eq!(
        msgs.len(),
        1,
        "say must not echo back to the speaker: {msgs:?}"
    );

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
///
/// Uses `CHAN_EMOTE`, not `CHAN_SAY`: this test is about the NPC filter, not
/// the say/emote echo split (`say_does_not_echo_to_speaker_but_emote_and_yell_do`
/// covers that), so it keeps the original "echo + one real witness = 2
/// messages" shape.
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
        CHAN_EMOTE,
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
    // (1, expected for emote). The NPC (100008) must never appear as an
    // `entity_id`.
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

/// Two players, `1` ("Alice") witnessing `2` ("Bob") and `3` ("Carol").
fn three_player_space() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    for (id, x) in [(1u32, 10.0f32), (2, 15.0), (3, 20.0)] {
        mgr.create_entity(id, "Agnos", [x, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(id);
    }
    let alice = mgr.get_entity_mut(1).unwrap();
    alice.witnesses.insert(cimmeria_common::EntityId(2));
    alice.witnesses.insert(cimmeria_common::EntityId(3));
    mgr
}

/// Entity ids that received a message after one line from Alice (1).
async fn recipients_of_alice(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    channel: u8,
) -> Vec<u32> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();
    handle_chat_message(1, "Alice", 0, channel, "Hello", &tx, mgr, &engine).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            out.push(entity_id);
        }
    }
    out.sort_unstable();
    out
}

/// CAT-L-01 / D-SS15: Bob ignores Alice, so Bob gets none of her say, emote
/// or yell; Carol still does. Alice gets her own echo for emote/yell (the
/// client doesn't show those locally) but not for say (it does) -- see
/// `say_does_not_echo_to_speaker_but_emote_and_yell_do`. Fails when the
/// ignore filter in `spatial::broadcast_to_witnesses` is removed.
#[tokio::test]
async fn spatial_chat_skips_ignoring_witness() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = three_player_space();
    mgr.get_entity_mut(2)
        .unwrap()
        .ignore_names
        .insert("Alice".to_string());
    for (channel, expected) in [
        (CHAN_SAY, vec![3]),
        (CHAN_EMOTE, vec![1, 3]),
        (CHAN_YELL, vec![1, 3]),
    ] {
        assert_eq!(
            recipients_of_alice(&mut mgr, channel).await,
            expected,
            "channel {channel}: the ignoring witness (2) must be skipped, \
             the other witness (3) kept"
        );
    }
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.spatial_ignored"))
        .expect("the withheld line must log chat.spatial_ignored");
    assert!(ev.has_field("reason", "witness_ignores_speaker"));
    assert!(
        ev.has_field("target_entity_id", "2"),
        "the withheld witness is named (instrumentation rule 5)"
    );
}

/// D-SS15 is one-directional: Alice ignoring Bob does not stop Bob hearing
/// Alice. Guards against a symmetric filter (PR #585's shape).
///
/// Uses `CHAN_EMOTE`, not `CHAN_SAY`: this test is about Ignore
/// directionality, not the say/emote echo split, so it keeps the speaker
/// echo (1) in its expected set unconditionally.
#[tokio::test]
async fn spatial_chat_reaches_witness_the_speaker_ignores() {
    let mut mgr = three_player_space();
    mgr.get_entity_mut(1)
        .unwrap()
        .ignore_names
        .insert("Bob".to_string());
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_EMOTE).await,
        vec![1, 2, 3]
    );
}

/// D-SS13 fold: an Ignore entry stored as "alice" (the contact-list window
/// keeps whatever case was typed) still withholds Alice's lines. Fails when
/// the spatial filter compares names exactly.
///
/// Uses `CHAN_EMOTE` for the same reason as
/// `spatial_chat_reaches_witness_the_speaker_ignores` above.
#[tokio::test]
async fn spatial_chat_ignore_matches_case_insensitively() {
    let mut mgr = three_player_space();
    mgr.get_entity_mut(2)
        .unwrap()
        .ignore_names
        .insert("aLiCe".to_string());
    assert_eq!(recipients_of_alice(&mut mgr, CHAN_EMOTE).await, vec![1, 3]);
}

/// A lone `emote`/`yell` speaker (nobody in range) still gets their own
/// echo -- the client does not show those locally. A lone `say` speaker gets
/// NOTHING from the server: the client already showed its own `say` line
/// (chat-speaker-echo.md), and echoing it again is the double-line bug.
/// Fails when the early return on an empty witness list is restored (the
/// emote/yell case), or when `say` gains a speaker echo again (the say
/// case).
#[tokio::test]
async fn lone_speaker_gets_no_say_echo_but_does_get_emote_and_yell_echo() {
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    // An NPC in range is not a chat recipient, so the player list is empty.
    mgr.create_entity(100009, "Agnos", [12.0, 0.0, 12.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .witnesses
        .insert(cimmeria_common::EntityId(100009));
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_SAY).await,
        Vec::<u32>::new(),
        "say: nothing -- the client already showed its own line"
    );
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_EMOTE).await,
        vec![1],
        "emote: the speaker's own echo, since the client doesn't show it locally"
    );
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_YELL).await,
        vec![1],
        "yell: the speaker's own echo, since the client doesn't show it locally"
    );
}

/// The direct regression guard for the speaker double-echo bug
/// (chat-speaker-echo.md): a `say` line must reach the speaker's own client
/// through the client's native local echo ONLY, never a second time from the
/// server. `emote` and `yell` have no client-side echo, so the server must
/// still send those to the speaker. Fails if the `CHAN_SAY` skip in
/// `broadcast_to_witnesses` is removed (the say assertion) or if
/// `emote`/`yell` stop being echoed (the other two).
#[tokio::test]
async fn say_does_not_echo_to_speaker_but_emote_and_yell_do() {
    let mut mgr = three_player_space();
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_SAY).await,
        vec![2, 3],
        "say: witnesses only -- no echo back to the speaker (1)"
    );
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_EMOTE).await,
        vec![1, 2, 3],
        "emote: witnesses AND the speaker's own echo"
    );
    assert_eq!(
        recipients_of_alice(&mut mgr, CHAN_YELL).await,
        vec![1, 2, 3],
        "yell: witnesses AND the speaker's own echo"
    );
}
