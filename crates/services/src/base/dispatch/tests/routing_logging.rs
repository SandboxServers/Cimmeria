//! Routing + log-level guards for `dispatch_sgw_player_base_method`:
//! logoff send-failure surfacing, the unhandled-method WARN, and the
//! explicit DEBUG handlers for `perfStats` / `elementDataRequest`.
//!
//! Split out of the monolithic `dispatch/tests.rs`; test bodies and
//! assertions were moved verbatim from the original.

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

/// logOff with a closed cell→base channel must surface
/// the dropped DisconnectEntity / DestroyEntity sends so a "ghost
/// player in space_manager" report can be traced back to the
/// logoff path. Reverting either `if let Err` to `let _ = tx.send`
/// trips this guard.
///
/// Uses `disconnect=0` (return-to-char-select) so the path runs
/// through both cell-tx sends and then the RESET_ENTITIES wire
/// emit. TestTransport captures the wire send; the cell-tx sends
/// fail because the receiver was dropped.
#[tokio::test]
async fn logoff_warns_when_cell_to_base_channel_closed_for_both_sends() {
    let capture = LogCapture::install();

    let addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();
    let entity_id: u32 = 4242;
    let key = [0u8; 32];

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

    // CLOSED channel.
    let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
    drop(rx);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

    // payload[0] == 0 => return-to-char-select branch. Skips the
    // loggedOff packet but still runs the RESET_ENTITIES path,
    // which uses transport.send_to (TestTransport swallows it).
    let payload = [0u8];

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &payload,
        &None,
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("logOff dispatch should not propagate Err for closed cell_tx");

    // Both sends fail independently — assert each produces its own
    // WARN so a partial revert (only one of two) is also caught.
    assert!(
        capture
            .find_message(Level::WARN, "logOff: DisconnectEntity send failed")
            .is_some(),
        "DisconnectEntity WARN missing. Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::WARN, "logOff: DestroyEntity send failed")
            .is_some(),
        "DestroyEntity WARN missing. Captured: {:#?}",
        capture.all()
    );

    // entity_to_addr cleanup still runs regardless of cell_tx failure.
    assert!(
        entity_to_addr.lock().unwrap().get(&entity_id).is_none(),
        "logOff must still clean up entity_to_addr even when cell_tx is closed"
    );
}

/// **Regression guard for issue #311** (Tier 4 follow-up to #304).
///
/// An SGWPlayer base-method dispatch with no registered handler must
/// fire `warn!` (not `trace!`) so that an unimplemented method index
/// appears on the standard ops dashboard rather than vanishing below
/// the default filter. Reverting the fall-through arm to `trace!`
/// fails this test on the level check.
///
/// Bug shape: when the client calls a base method the server hasn't
/// implemented, the original `trace!` swallowed the signal — the
/// server silently returned `Ok` and the client's session would
/// behave as if the method had run. This guard pins the level
/// promotion AND the structured `msg_id` / `base_method_index`
/// fields ops queries pivot on.
#[tokio::test]
async fn unhandled_base_method_warns_with_msg_id_and_base_index() {
    let capture = LogCapture::install();

    let addr: SocketAddr = "127.0.0.1:54322".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

    // cell_tx isn't exercised by the unhandled-method fall-through arm;
    // pass None to keep the test minimal.
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // 0xFF is past the last defined SGWPlayer base method (ON_CLIENT_READY
    // = 0xD8). Guaranteed to land in the fall-through arm regardless of
    // future handler additions in the 0xC0–0xD8 range.
    let unhandled_msg_id: u8 = 0xFF;

    dispatch_sgw_player_base_method(
        unhandled_msg_id,
        /* payload */ &[],
        /* player_name */ &None,
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("unhandled method must not propagate Err — just log");

    let event = capture
        .find_message(Level::WARN, "Unhandled SGWPlayer base method")
        .unwrap_or_else(|| {
            panic!(
                "expected WARN for unhandled SGWPlayer base method; \
                 a revert to `trace!` makes this test fail because the \
                 event is captured at a level below WARN. Captured: {:#?}",
                capture.all()
            )
        });

    // Pin the structured field shape so an ops query for
    // `base_method_index` continues to surface the gap. A refactor that
    // drops either field would let the warn fire but break the query.
    assert!(
        event.has_field("base_method_index", "63"),
        "base_method_index must be 0xFF - 0xC0 = 63; got {event:#?}",
    );
}

/// Pins the `perfStats` (0xDD / index 29) explicit DEBUG handler.
/// The client pushes 12 × FLOAT telemetry every ~15 s; without
/// an explicit handler it landed in the unhandled-WARN catch-all
/// and produced ~40 WARNs per session of pure noise. Promoting
/// back to WARN (by removing the match arm) trips this guard
/// because the unhandled-WARN body fires instead of the DEBUG.
#[tokio::test]
async fn perf_stats_logs_at_debug_not_warn() {
    let capture = LogCapture::install();

    let addr: SocketAddr = "127.0.0.1:54323".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // 12 × FLOAT = 48 bytes — the documented wire shape per
    // `docs/protocol/sgwplayer-base-method-dispatch-table.md`.
    let payload = [0u8; 48];

    dispatch_sgw_player_base_method(
        sgw_player_base::PERF_STATS,
        &payload,
        &None,
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("perfStats dispatch must not propagate Err");

    assert!(
        capture
            .find_message(Level::DEBUG, "SGWPlayer.perfStats — telemetry sink")
            .is_some(),
        "perfStats must log at DEBUG; got: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::WARN, "Unhandled SGWPlayer base method")
            .is_none(),
        "perfStats must NOT fall through to the unhandled-WARN catch-all — \
         removing the explicit PERF_STATS arm re-introduces the ~40-WARN/session \
         noise observed pre-fix: {:#?}",
        capture.all()
    );
}

/// Pins the `elementDataRequest` (0xD5 / index 21) explicit DEBUG
/// handler. In-world cache-miss requests are diagnostic only —
/// the catalog + per-key push happens in `cooked_data.rs` before
/// world entry, so the runtime path here serves no live data. The
/// pre-fix unhandled-WARN created a category of operator-alert
/// noise indistinguishable from genuinely missing handlers.
#[tokio::test]
async fn element_data_request_logs_at_debug_not_warn() {
    let capture = LogCapture::install();

    let addr: SocketAddr = "127.0.0.1:54324".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // UINT16 categoryId + UINT32 key = 6 bytes per the dispatch table.
    let mut payload = Vec::with_capacity(6);
    payload.extend_from_slice(&7u16.to_le_bytes()); // category
    payload.extend_from_slice(&12345u32.to_le_bytes()); // key

    dispatch_sgw_player_base_method(
        sgw_player_base::ELEMENT_DATA_REQUEST,
        &payload,
        &None,
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("elementDataRequest dispatch must not propagate Err");

    let event = capture
        .find_message(
            Level::DEBUG,
            "SGWPlayer.elementDataRequest — in-world cache miss",
        )
        .unwrap_or_else(|| {
            panic!(
                "elementDataRequest must log at DEBUG; got: {:#?}",
                capture.all()
            )
        });
    // Pin the parsed fields so a refactor that drops them (which
    // would also break ops queries pivoting on category_id) trips
    // here instead of slipping in silently.
    assert!(
        event.has_field("category_id", "7") && event.has_field("key", "12345"),
        "elementDataRequest must surface category_id + key fields: {event:#?}",
    );
    assert!(
        capture
            .find_message(Level::WARN, "Unhandled SGWPlayer base method")
            .is_none(),
        "elementDataRequest must NOT fall through to the unhandled-WARN catch-all \
         — removing the explicit ELEMENT_DATA_REQUEST arm re-introduces operator-alert \
         noise: {:#?}",
        capture.all()
    );
}
