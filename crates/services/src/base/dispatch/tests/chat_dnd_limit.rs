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
async fn dnd_rejects_129_unicode_scalars_preserving_active_and_inactive_state() {
    for previous in [None, Some("do not disturb".to_string())] {
        for scalar in ["x", "界", "🚀"] {
            let capture = LogCapture::install();
            assert_eq!(
                update_dnd(previous.clone(), &scalar.repeat(129)).await,
                previous,
                "overlong input must neither enable DND nor replace existing text",
            );
            assert!(capture
                .find_event(
                    Level::DEBUG,
                    "chatSetDNDMessage: message exceeds limit",
                    "dnd_message_too_long",
                )
                .is_some());
        }
    }
}
