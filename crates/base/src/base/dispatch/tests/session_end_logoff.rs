//! TG-NET-01: `logOff(0)` writes exactly one `session.end` row for the
//! character that leaves the world. Return to character select keeps the
//! session alive, so no teardown writes it for this path; `handle_log_off`
//! must, and a teardown racing it must not write a second.

use super::super::*;
use crate::test_support::{
    test_default_connected_client_state, Captured, LogCapture, LogCaptureGuard, TestTransport,
};

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

const ENTITY_ID: u32 = 4343;

/// A session with a character in the world. `listed_online` is set by
/// `onClientReady`, so `false` is a character between `session.start` and
/// client ready.
fn in_world_session(addr: SocketAddr, listed_online: bool) -> Connected {
    let mut state = test_default_connected_client_state();
    state.account_id = 0x5EED_0801;
    state.account_name = Some("sgc_login".into());
    state.player_entity_id = Some(ENTITY_ID);
    state.player_name = Some("Lomiada".to_string());
    state.active_player_id = Some(7);
    state.listed_online = listed_online;
    Arc::new(Mutex::new(HashMap::from([(addr, state)])))
}

fn session_ends(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "session.end")
        .collect()
}

async fn log_off(
    disconnect: u8,
    addr: SocketAddr,
    connected: &Connected,
    cell_tx: mpsc::Sender<BaseToCellMsg>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[disconnect],
        &Some("Lomiada".to_string()),
        addr,
        &transport,
        [0u8; 32],
        connected,
        &entity_manager,
        &Some(cell_tx),
        &entity_to_addr,
        &None,
    )
    .await
    .expect("logOff must not fail");
}

#[tokio::test]
async fn logoff_to_character_select_ends_the_session() {
    let addr: SocketAddr = "127.0.0.1:54800".parse().unwrap();
    let connected = in_world_session(addr, true);
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    log_off(0, addr, &connected, tx).await;

    let ends = session_ends(&capture);
    assert_eq!(ends.len(), 1, "one session.end row: {:#?}", capture.all());
    let end = &ends[0];
    assert!(end.has_field("disconnect_reason", "logoff_character_select"));
    assert!(end.has_field("entity_id", "4343"), "{end:#?}");
    assert!(end.has_field("entity_name", "Lomiada"), "{end:#?}");
    assert!(end.has_field("player_id", "7"), "{end:#?}");
    assert!(end.has_field("player_name", "Lomiada"), "{end:#?}");
    assert!(end.has_field("account_name", "sgc_login"), "{end:#?}");
    let secs = end
        .fields
        .get("session_secs")
        .unwrap_or_else(|| panic!("session_secs missing: {end:#?}"));
    secs.parse::<u64>()
        .unwrap_or_else(|e| panic!("session_secs {secs:?} is not a number: {e}"));
    assert_eq!(
        connected.lock().unwrap().len(),
        1,
        "the session survives a return to character select"
    );
}

/// `listed_online` is set only in `onClientReady`; a logOff(0) before it
/// still ends a session the cell's `player_init` started.
#[tokio::test]
async fn logoff_before_client_ready_still_ends_the_session() {
    let addr: SocketAddr = "127.0.0.1:54801".parse().unwrap();
    let connected = in_world_session(addr, false);
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    log_off(0, addr, &connected, tx).await;

    let ends = session_ends(&capture);
    assert_eq!(ends.len(), 1, "one session.end row: {:#?}", capture.all());
    assert!(ends[0].has_field("disconnect_reason", "logoff_character_select"));
    assert!(ends[0].has_field("entity_id", "4343"), "{:#?}", ends[0]);
}

/// A full exit leaves the row to the teardown that follows the client's
/// disconnect; logOff(1) itself writes none.
#[tokio::test]
async fn full_exit_logoff_writes_no_character_select_row() {
    let addr: SocketAddr = "127.0.0.1:54802".parse().unwrap();
    let connected = in_world_session(addr, true);
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    log_off(1, addr, &connected, tx).await;

    let ends = session_ends(&capture);
    assert!(ends.is_empty(), "no session.end on logOff(1): {ends:#?}");
    assert_eq!(
        connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .player_entity_id,
        Some(ENTITY_ID),
        "the teardown after a full exit still owns the session end"
    );
}

/// logOff(0), then the session's teardown: one row in total.
#[tokio::test]
async fn logoff_then_teardown_writes_one_row() {
    let addr: SocketAddr = "127.0.0.1:54803".parse().unwrap();
    let connected = in_world_session(addr, true);
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    log_off(0, addr, &connected, tx.clone()).await;
    teardown(addr, &connected, tx);

    let ends = session_ends(&capture);
    assert_eq!(ends.len(), 1, "one session.end row: {:#?}", capture.all());
    assert!(ends[0].has_field("disconnect_reason", "logoff_character_select"));
}

/// A teardown that lands while logOff(0) is parked on an await (here, a
/// full Base→Cell channel) must not write a second row.
#[tokio::test]
async fn teardown_during_logoff_writes_one_row() {
    let addr: SocketAddr = "127.0.0.1:54804".parse().unwrap();
    let connected = in_world_session(addr, true);
    // Capacity 1, already full: logOff's DisconnectEntity send parks.
    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(1);
    tx.try_send(BaseToCellMsg::DestroyEntity { entity_id: 0 })
        .unwrap();
    let capture = LogCapture::install();

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx = Some(tx.clone());
    let player_name = Some("Lomiada".to_string());
    let fut = dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[0],
        &player_name,
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
        &None,
    );
    tokio::pin!(fut);
    // Drive logOff up to the parked send.
    tokio::select! {
        biased;
        _ = &mut fut => panic!("logOff must park on the full cell channel"),
        _ = tokio::task::yield_now() => {}
    }

    // The client's DISCONNECT arrives in that window.
    teardown(addr, &connected, tx);

    // Unpark logOff by draining the cell channel until it finishes. The
    // session is gone, so its later sends may fail; the result is ignored.
    let drain = async {
        loop {
            if rx.recv().await.is_none() {
                break;
            }
        }
    };
    tokio::select! {
        _ = &mut fut => {}
        _ = drain => {}
    }

    let ends = session_ends(&capture);
    assert_eq!(ends.len(), 1, "one session.end row: {:#?}", capture.all());
    assert!(ends[0].has_field("disconnect_reason", "logoff_character_select"));
}

fn teardown(addr: SocketAddr, connected: &Connected, tx: mpsc::Sender<BaseToCellMsg>) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    cimmeria_base_session::base::helpers::destroy_client_entities(
        connected,
        &entity_manager,
        addr,
        &Some(tx),
        &entity_to_addr,
        &transport,
        &None,
        "client_disconnect",
    );
}
