//! `handle_on_client_ready` regression guards: the base→cell handoff
//! negative logs and the `first_login` UPDATE `rows_affected == 0` branch.

use super::*;

// ──────────────────────────────────────────────────────────────────
// Negative-logging regression guards: onClientReady
// cell_tx sends. ConnectEntity / InitPlayerState / AdvanceRing-
// Destination are critical handoffs from base to cell — dropping
// any of them strands the player in a half-loaded state. The
// guards drive `handle_on_client_ready` with a closed cell→base
// channel and assert each of the three ERROR logs fires
// independently so a partial revert (only one of three) is also
// caught.
// ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn on_client_ready_errors_each_cell_tx_send_independently_when_closed() {
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55600".parse().unwrap();
    let entity_id: u32 = 8888;
    let key = [0u8; 32];

    // Pre-stage pending_client_ready + pending_destination_ring_id
    // so the function reaches all three cell_tx send sites.
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id: 42,
        world_name: "Agnos".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0, // skip the cinematic + DB UPDATE branch
    });
    // Non-None forces the AdvanceRingDestination send to fire.
    state.pending_destination_ring_id = Some(17);
    state.player_name = Some("Tester".to_string());

    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, state);
        m
    }));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Closed cell→base channel: all three sends will SendError.
    let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
    drop(rx);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

    // db_pool=None so DB-querying branches return empty/default
    // without needing a live Postgres.
    let _ = handle_on_client_ready(
        addr,
        key,
        &connected,
        &cell_tx,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await;

    // All three ERRORs must fire — assert each independently so a
    // partial revert (e.g. only one of three reverted to `let _`)
    // is also caught.
    assert!(
        capture
            .find_message(Level::ERROR, "ConnectEntity: base→cell send failed")
            .is_some(),
        "negative-logging convention: ConnectEntity ERROR missing. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::ERROR, "InitPlayerState: base→cell send failed")
            .is_some(),
        "negative-logging convention: InitPlayerState ERROR missing"
    );
    assert!(
        capture
            .find_message(
                Level::ERROR,
                "AdvanceRingDestination: base→cell send failed"
            )
            .is_some(),
        "negative-logging convention: AdvanceRingDestination ERROR missing (requires \
         pending_destination_ring_id to be Some; check test staging)"
    );
}

/// Live-DB regression guard for the `first_login` UPDATE's
/// `rows_affected == 0` branch. Stages `pending.first_login = 1`
/// with `pending.player_id` pointing at a `sgw_player` row that
/// does NOT exist; the UPDATE succeeds (no SQL error) but
/// touches zero rows. Pre-#304 this was silent — the flag stayed
/// set in some other table or the cinematic just re-fired every
/// login with no signal. The handler now emits an ERROR naming
/// `rows_affected=0` + `expected=1` so a single ops query
/// catches it.
///
/// Sentinel ID is well below `i32::MAX` and outside the live-DB
/// fixture base ranges; nothing to clean up because the test
/// does NOT insert the row.
#[tokio::test]
async fn first_login_update_errors_when_player_row_missing() {
    use crate::test_support::{
        require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
    };
    use tracing::Level;

    let pool = require_db_or_skip!();
    let capture = LogCapture::install();

    // Sentinel — must NOT exist in sgw_player. Picked outside
    // every other test base (TEST_BASE families peak around
    // 0x7000_0FFF).
    const MISSING_PLAYER_ID: i32 = 0x7FFE_FF99;

    let addr: SocketAddr = "127.0.0.1:55700".parse().unwrap();
    let entity_id: u32 = 9999;
    let key = [0u8; 32];

    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id: MISSING_PLAYER_ID,
        world_name: "Agnos".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 1, // forces the cinematic + UPDATE branch
    });
    state.player_name = Some("Tester".to_string());

    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(addr, state);
        m
    }));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Open cell→base channel; we don't care about its sends for this test.
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(32);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);
    let db_pool = Some(Arc::new(pool));

    let _ = handle_on_client_ready(
        addr,
        key,
        &connected,
        &cell_tx,
        &transport,
        &entity_to_addr,
        &db_pool,
    )
    .await;

    let event = capture
        .find_message(Level::ERROR, "first_login flag NOT cleared")
        .expect(
            "negative-logging convention: missing player row must emit \
             ERROR with rows_affected=0",
        );
    assert!(
        event.has_field("rows_affected", "0"),
        "rows_affected field must be 0 — pin the structured shape so \
         ops can query (rows_affected != expected): {event:#?}"
    );
    assert!(
        event.has_field("expected", "1"),
        "expected field must be 1 — pin the paired structured field: {event:#?}"
    );
    assert!(
        event.has_field("player_id", &MISSING_PLAYER_ID.to_string()),
        "player_id field must carry the missing id for ops triage: {event:#?}"
    );
}
