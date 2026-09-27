//! Stored DND text is bounded at the public base-method dispatch boundary.

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

async fn update_dnd(previous: Option<String>, message: &str) -> Option<String> {
    let addr: SocketAddr = "127.0.0.1:54404".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.dnd_message = previous;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let mut payload = Vec::new();
    crate::mercury::write_wstring(&mut payload, message);

    dispatch_sgw_player_base_method(
        sgw_player_base::CHAT_SET_DND,
        &payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        [0; 32],
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
    )
    .await
    .expect("DND update must finish without a dispatch error");

    let result = connected
        .lock()
        .unwrap()
        .get(&addr)
        .unwrap()
        .dnd_message
        .clone();
    result
}

#[tokio::test]
async fn dnd_accepts_128_unicode_scalars_without_truncation() {
    for scalar in ["x", "界", "🚀"] {
        let message = scalar.repeat(128);
        assert_eq!(
            update_dnd(Some("old status".into()), &message).await,
            Some(message),
            "the bound counts Unicode scalars, not UTF-8 bytes or UTF-16 units",
        );
    }
}

#[tokio::test]
async fn dnd_truncates_129_unicode_scalars_and_still_activates() {
    for previous in [None, Some("do not disturb".to_string())] {
        for scalar in ["x", "界", "🚀"] {
            let capture = LogCapture::install();
            assert_eq!(
                update_dnd(previous.clone(), &scalar.repeat(129)).await,
                Some(scalar.repeat(128)),
                "overlong input must set DND with the text cut to 128 scalars",
            );
            assert!(capture
                .find_event(
                    Level::DEBUG,
                    "chatSetDNDMessage: message exceeds limit",
                    "dnd_message_truncated",
                )
                .is_some());
        }
    }
}

#[tokio::test]
async fn dnd_bound_holds_for_a_very_long_message() {
    let stored = update_dnd(None, &"y".repeat(10_000)).await;
    assert_eq!(stored.map(|m| m.chars().count()), Some(128));
}

/// SS-C1: `chatSetAFKMessage` stores the away message a tell answers with,
/// clears it on an empty or 1-char message, bounds it like DND, and leaves
/// it alone on a malformed payload.
#[tokio::test]
async fn chat_set_afk_stores_clears_and_bounds_the_away_message() {
    let addr: SocketAddr = "127.0.0.1:54406".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let afk = |payload: Vec<u8>| {
        let (connected, transport, entity_manager, entity_to_addr) = (
            connected.clone(),
            transport.clone(),
            entity_manager.clone(),
            entity_to_addr.clone(),
        );
        async move {
            dispatch_sgw_player_base_method(
                sgw_player_base::CHAT_SET_AFK,
                &payload,
                &Some("Tester".to_string()),
                addr,
                &transport,
                [0; 32],
                &connected,
                &entity_manager,
                &None,
                &entity_to_addr,
                &None,
            )
            .await
            .expect("AFK update must not error");
            connected.lock().unwrap()[&addr].afk_message.clone()
        }
    };
    let wstr = |s: &str| {
        let mut p = Vec::new();
        crate::mercury::write_wstring(&mut p, s);
        p
    };

    assert_eq!(afk(wstr("at lunch")).await.as_deref(), Some("at lunch"));
    assert_eq!(
        afk(vec![0xFF, 0xFF]).await.as_deref(),
        Some("at lunch"),
        "a malformed payload keeps the message"
    );
    assert_eq!(afk(wstr("x")).await, None, "a 1-char message clears it");
    let long = "z".repeat(300);
    assert_eq!(
        afk(wstr(&long)).await.map(|m| m.chars().count()),
        Some(128),
        "stored bounded like DND"
    );
}

/// An away message is read by other players in the tell auto-reply, so the
/// D-SS12 character rules apply: a control or bidi character refuses the
/// AFK or DND message with a feedback line and keeps the previous one.
/// Fails when `away_message_allowed` is not called.
#[tokio::test]
async fn away_messages_follow_the_chat_character_rules() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:54407".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(5150);
    state.afk_message = Some("old afk".to_string());
    state.dnd_message = Some("old dnd".to_string());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let test_transport = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    for method in [sgw_player_base::CHAT_SET_AFK, sgw_player_base::CHAT_SET_DND] {
        let mut payload = Vec::new();
        crate::mercury::write_wstring(&mut payload, "brb\u{202E}lol");
        dispatch_sgw_player_base_method(
            method,
            &payload,
            &Some("Tester".to_string()),
            addr,
            &transport,
            [0; 32],
            &connected,
            &entity_manager,
            &None,
            &entity_to_addr,
            &None,
        )
        .await
        .expect("away update must not error");
    }
    {
        let g = connected.lock().unwrap();
        assert_eq!(g[&addr].afk_message.as_deref(), Some("old afk"));
        assert_eq!(g[&addr].dnd_message.as_deref(), Some("old dnd"));
    }
    assert_eq!(
        test_transport.filter_to(addr).len(),
        2,
        "one feedback line per refused away message"
    );
    let rejected: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "chat.away_rejected"))
        .collect();
    assert_eq!(rejected.len(), 2);
    assert!(rejected
        .iter()
        .all(|c| c.has_field("reason", "bidi_control")));
}

/// PR #893 review: the character rules run on the whole decoded message,
/// before the 128-scalar cut. A bidi override at scalar 130 refuses the
/// message; checked after the cut, it would be dropped and the rest stored.
#[tokio::test]
async fn away_message_with_a_forbidden_character_past_128_is_refused() {
    let addr: SocketAddr = "127.0.0.1:54408".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(5151);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let test_transport = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let text = format!("{}\u{202E}tail", "a".repeat(130));
    for method in [sgw_player_base::CHAT_SET_AFK, sgw_player_base::CHAT_SET_DND] {
        let mut payload = Vec::new();
        crate::mercury::write_wstring(&mut payload, &text);
        dispatch_sgw_player_base_method(
            method,
            &payload,
            &Some("Tester".to_string()),
            addr,
            &transport,
            [0; 32],
            &connected,
            &entity_manager,
            &None,
            &entity_to_addr,
            &None,
        )
        .await
        .expect("away update must not error");
    }
    {
        let g = connected.lock().unwrap();
        assert_eq!(g[&addr].afk_message, None, "AFK refused, not truncated");
        assert_eq!(g[&addr].dnd_message, None, "DND refused, not truncated");
    }
    assert_eq!(
        test_transport.filter_to(addr).len(),
        2,
        "two feedback lines"
    );
}
