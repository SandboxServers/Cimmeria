//! TG-NET-01: `logOff(0)` from an in-world session writes one `session.end`
//! row. Return to character select keeps the session alive, so no teardown
//! writes it for this path; `handle_log_off` must.

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

#[tokio::test]
async fn logoff_to_character_select_ends_the_session() {
    let addr: SocketAddr = "127.0.0.1:54800".parse().unwrap();
    let entity_id: u32 = 4343;

    let mut state = test_default_connected_client_state();
    state.account_id = 0x5EED_0801;
    state.account_name = Some("sgc_login".into());
    state.player_entity_id = Some(entity_id);
    state.player_name = Some("Lomiada".to_string());
    state.active_player_id = Some(7);
    state.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[0],
        &Some("Lomiada".to_string()),
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &Some(tx),
        &entity_to_addr,
        &None,
    )
    .await
    .expect("logOff must not fail");

    let ends: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "session.end")
        .collect();
    assert_eq!(ends.len(), 1, "one session.end row: {:#?}", capture.all());
    let end = &ends[0];
    assert!(end.has_field("disconnect_reason", "logoff_character_select"));
    assert!(end.has_field("entity_id", "4343"), "{end:#?}");
    assert!(end.has_field("entity_name", "Lomiada"), "{end:#?}");
    assert!(end.has_field("player_id", "7"), "{end:#?}");
    assert!(end.has_field("player_name", "Lomiada"), "{end:#?}");
    assert!(end.has_field("account_name", "sgc_login"), "{end:#?}");
    assert_eq!(
        connected.lock().unwrap().len(),
        1,
        "the session survives a return to character select"
    );
}
