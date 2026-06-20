//! Integration tests for `dispatch_sgw_player_base_method`.
//!
//! Moved verbatim from the inline `#[cfg(test)] mod tests` in `dispatch.rs`
//! when that file was split (issue #529). Only the module wrapper changed —
//! every test body and assertion is byte-identical to the original.

use super::*;
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

// ── Speaker flags regression guards ────────────────────────────────
//
// The chat dispatch at `SEND_PLAYER_COMMUNICATION` must compute
// `speaker_flags` from per-connection state:
//   - SPEAKER_GM  (0x01) when access_level > 0
//   - SPEAKER_DND (0x04) when dnd_message.is_some()
// matching `python/base/Chat.py::getSpeakerFlags`. The wire
// serializer is byte-exact already (pinned by
// `serialize_on_player_communication_basic` in `cell/chat.rs`); the
// guard surface here is the bit-assembly done before the cell
// forward. Reverting any of these branches to the old hardcoded
// `speaker_flags: 0` trips tests 1–4; reverting the DND handler to
// its stub trips test 5.

/// Build a `sendPlayerCommunication` payload:
/// `[u8 channel][WSTRING target][WSTRING text]`. `target` is the
/// recipient name for tell/private channels — empty for spatial
/// channels (say/emote/yell).
fn build_send_player_communication_payload(channel: u8, target: &str, text: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(channel);
    crate::mercury::write_wstring(&mut buf, target);
    crate::mercury::write_wstring(&mut buf, text);
    buf
}

/// Drive the SEND_PLAYER_COMMUNICATION dispatch arm against a
/// freshly built `ConnectedClientState` (with the requested
/// `access_level` and `dnd_message`) and return the resulting
/// `speaker_flags` value sent to the cell. `player_entity_id` is
/// pre-set so the cell-forward branch is reached.
async fn drive_send_player_communication_and_get_flags(
    access_level: u32,
    dnd_message: Option<String>,
) -> u8 {
    let addr: SocketAddr = "127.0.0.1:54400".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(1234);
    state.access_level = access_level;
    state.dnd_message = dnd_message;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(4);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

    // channel=0 (say), no target, "hi" as text.
    let payload = build_send_player_communication_payload(0, "", "hi");

    dispatch_sgw_player_base_method(
        sgw_player_base::SEND_PLAYER_COMMUNICATION,
        &payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("sendPlayerCommunication dispatch should not propagate Err");

    // Drop cell_tx so rx.recv() returns None instead of hanging if
    // dispatch didn't actually emit a message — gives a clear panic
    // path rather than a 60-second test timeout.
    drop(cell_tx);

    match rx.recv().await {
        Some(BaseToCellMsg::ChatMessage { speaker_flags, .. }) => speaker_flags,
        Some(_) => panic!("expected BaseToCellMsg::ChatMessage; got a different variant"),
        None => panic!("dispatch did not forward a ChatMessage to the cell"),
    }
}

/// Test 1: a default connection (access_level=0, no DND) sends
/// `speaker_flags == 0`. Regression for the hardcoded-zero literal.
#[tokio::test]
async fn send_player_communication_default_state_has_zero_speaker_flags() {
    let flags = drive_send_player_communication_and_get_flags(0, None).await;
    assert_eq!(
        flags, 0,
        "default state must produce speaker_flags == 0 (no GM, no DND)"
    );
}

/// Test 2: access_level=1 (Moderator) sets SPEAKER_GM (0x01).
/// Python parity: `accessLevel > 0`, so Moderators get the bit too.
#[tokio::test]
async fn send_player_communication_access_level_one_sets_gm_flag() {
    let flags = drive_send_player_communication_and_get_flags(1, None).await;
    assert_eq!(
        flags & speaker_flags::GM,
        speaker_flags::GM,
        "access_level > 0 must set SPEAKER_GM (0x01); got {flags:#04x}",
    );
    assert_eq!(
        flags & speaker_flags::DND,
        0,
        "SPEAKER_DND must NOT be set when dnd_message is None; got {flags:#04x}",
    );
}

/// Test 3: dnd_message present sets SPEAKER_DND (0x04) even when
/// access_level == 0 (non-GM user).
#[tokio::test]
async fn send_player_communication_dnd_message_sets_dnd_flag() {
    let flags =
        drive_send_player_communication_and_get_flags(0, Some("busy raiding".to_string()))
            .await;
    assert_eq!(
        flags & speaker_flags::DND,
        speaker_flags::DND,
        "dnd_message.is_some() must set SPEAKER_DND (0x04); got {flags:#04x}",
    );
    assert_eq!(
        flags & speaker_flags::GM,
        0,
        "SPEAKER_GM must NOT be set when access_level == 0; got {flags:#04x}",
    );
}

/// Test 4: GM with DND active sets both bits — verifies bitwise OR
/// composition and that neither branch clobbers the other.
#[tokio::test]
async fn send_player_communication_gm_with_dnd_sets_both_flags() {
    let flags =
        drive_send_player_communication_and_get_flags(2, Some("on duty, dnd".to_string()))
            .await;
    assert_eq!(
        flags,
        speaker_flags::GM | speaker_flags::DND,
        "access_level=2 + dnd_message=Some must produce 0x05; got {flags:#04x}",
    );
}

/// Test 5: `CHAT_SET_DND` handler round-trip: a >1-char payload
/// sets the field; a follow-up 0-char payload clears it. Pins the
/// handler's set/clear semantics (matches the Python `<= 1 char`
/// rule). Reverting `CHAT_SET_DND` to the stub trips this.
#[tokio::test]
async fn chat_set_dnd_handler_sets_then_clears_dnd_message() {
    let addr: SocketAddr = "127.0.0.1:54401".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // ── Set: a >1-char message populates dnd_message ───────────
    let mut set_payload = Vec::new();
    crate::mercury::write_wstring(&mut set_payload, "afk for dinner");
    dispatch_sgw_player_base_method(
        sgw_player_base::CHAT_SET_DND,
        &set_payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("chatSetDNDMessage (set) must not propagate Err");
    // Bind the cloned Option so the MutexGuard's lifetime ends
    // before any assert macro evaluates its arguments.
    let after_set = {
        let g = connected.lock().unwrap();
        g.get(&addr).and_then(|c| c.dnd_message.clone())
    };
    assert_eq!(
        after_set,
        Some("afk for dinner".to_string()),
        "chatSetDNDMessage with a >1-char message must populate dnd_message",
    );

    // ── Clear: an empty message clears dnd_message ─────────────
    let mut clear_payload = Vec::new();
    crate::mercury::write_wstring(&mut clear_payload, "");
    dispatch_sgw_player_base_method(
        sgw_player_base::CHAT_SET_DND,
        &clear_payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("chatSetDNDMessage (clear) must not propagate Err");
    let after_clear = {
        let g = connected.lock().unwrap();
        g.get(&addr).and_then(|c| c.dnd_message.clone())
    };
    assert_eq!(
        after_clear, None,
        "chatSetDNDMessage with an empty payload must clear dnd_message",
    );
}

/// Test 6: a 1-character DND payload clears `dnd_message`, matching
/// Python's `chatSetDNDMessage` which treats `len(message) <= 1`
/// as a clear. Pins the `> 1` threshold so a regression from
/// `> 1` to `> 0` (which would treat "x" as setting DND to "x")
/// trips this test.
///
/// Boundary case: existing tests prove `>1` sets and empty clears.
/// This guards the exact discriminator between the two branches.
#[tokio::test]
async fn chat_set_dnd_handler_one_char_payload_clears_dnd_message() {
    let addr: SocketAddr = "127.0.0.1:54402".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Seed the connection with DND already active so the 1-char
    // payload has something to clear (otherwise None->None looks
    // identical to a no-op handler).
    let mut state = test_default_connected_client_state();
    state.dnd_message = Some("previously busy".to_string());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // 1-char payload.
    let mut payload = Vec::new();
    crate::mercury::write_wstring(&mut payload, "x");

    dispatch_sgw_player_base_method(
        sgw_player_base::CHAT_SET_DND,
        &payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("chatSetDNDMessage (1-char) must not propagate Err");

    let after = {
        let g = connected.lock().unwrap();
        g.get(&addr).and_then(|c| c.dnd_message.clone())
    };
    assert_eq!(
        after, None,
        "1-char DND payload must clear dnd_message (matches Python `len <= 1` clear rule)",
    );
}

/// Test 7: a malformed `CHAT_SET_DND` payload (too short for the
/// WSTRING char_count header) must leave existing `dnd_message`
/// untouched. Reverting the handler to `unwrap_or_default()`
/// (which coerces the decode failure to `""` and then runs the
/// "≤1 char clears" rule) would wipe DND on a garbage packet —
/// this guard trips that revert.
///
/// Also asserts a WARN with `reason = "read_wstring_failed"` so
/// the failure is visible in SigNoz rather than a silent drop.
#[tokio::test]
async fn chat_set_dnd_handler_malformed_payload_preserves_dnd_message() {
    let capture = LogCapture::install();

    let addr: SocketAddr = "127.0.0.1:54403".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let mut state = test_default_connected_client_state();
    state.dnd_message = Some("do not disturb".to_string());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // 2 bytes: shorter than the 4-byte WSTRING char_count header
    // `read_wstring` requires. `read_wstring(payload, 0)` returns
    // Err — the handler must NOT then clear dnd_message.
    let payload = [0u8, 0u8];

    dispatch_sgw_player_base_method(
        sgw_player_base::CHAT_SET_DND,
        &payload,
        &Some("Tester".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("chatSetDNDMessage (malformed) must not propagate Err -- just warn + skip");

    let after = {
        let g = connected.lock().unwrap();
        g.get(&addr).and_then(|c| c.dnd_message.clone())
    };
    assert_eq!(
        after,
        Some("do not disturb".to_string()),
        "malformed CHAT_SET_DND must NOT clear existing dnd_message",
    );

    // Pin the `reason` field so a generic refactor that drops the
    // structured tag (but still warns) is also flagged.
    let event = capture
        .find_message(Level::WARN, "chatSetDNDMessage: WSTRING decode failed")
        .unwrap_or_else(|| {
            panic!(
                "expected WARN for malformed CHAT_SET_DND payload; \
                 a silent unwrap_or_default revert fails this. Captured: {:#?}",
                capture.all()
            )
        });
    assert!(
        event.has_field("reason", "read_wstring_failed"),
        "reason field must be `read_wstring_failed` for SigNoz pivots; got {event:#?}",
    );
}

/// Test 8: the post-character-select reset path clears
/// `dnd_message`. Without this, char A's `/dnd` would leak into
/// char B on the same connection — every subsequent
/// `sendPlayerCommunication` from char B would incorrectly carry
/// `SPEAKER_DND` until char B's user toggled DND explicitly.
///
/// Drives the disconnect=0 (return-to-character-select) branch of
/// `LOG_OFF`, which is the only path that resets per-character
/// state without dropping the connection. Reverting the
/// `c.dnd_message = None;` line in that block trips this test.
#[tokio::test]
async fn logoff_disconnect_zero_clears_dnd_message_for_next_character() {
    let addr: SocketAddr = "127.0.0.1:54404".parse().unwrap();
    let entity_id: u32 = 4243;
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.dnd_message = Some("char A is busy".to_string());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

    // Open cell_tx receiver so the DisconnectEntity / DestroyEntity
    // sends don't warn (we're testing the reset path, not the
    // send-failure path).
    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(8);
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

    // disconnect == 0 => return-to-character-select reset path.
    let payload = [0u8];

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &payload,
        &Some("CharA".to_string()),
        addr,
        &transport,
        key,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("logOff (disconnect=0) must not propagate Err");

    // Drain the cell-tx so nothing leaks (and so the assertion
    // panic message stays clean).
    while rx.try_recv().is_ok() {}

    let after = {
        let g = connected.lock().unwrap();
        g.get(&addr).and_then(|c| c.dnd_message.clone())
    };
    assert_eq!(
        after, None,
        "disconnect=0 (return-to-character-select) must clear dnd_message \
         so char A's DND state does not leak into char B on the same connection",
    );
}

/// Test 9: the `CHAT_SET_AFK` handler is log-only and must not
/// crash or mutate connection state for any payload shape. AFK is
/// intentionally NOT wired into `speaker_flags` (no `SPEAKER_AFK`
/// token exists in `entities/defs/enumerations.xml`). This guard
/// pins both invariants so a future "AFK should set a flag" change
/// trips the test and forces a deliberate review of the enum.
#[tokio::test]
async fn chat_set_afk_handler_is_log_only_and_preserves_state() {
    let addr: SocketAddr = "127.0.0.1:54405".parse().unwrap();
    let key = [0u8; 32];
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

    // Seed non-default state to verify AFK doesn't stomp on it.
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(9999);
    state.dnd_message = Some("on a raid".to_string());
    state.access_level = 1;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    // Drive several payload shapes to exercise both the empty-buf
    // and decoded-WSTRING paths the handler's body never reads.
    let mut valid_wstring_payload = Vec::new();
    crate::mercury::write_wstring(&mut valid_wstring_payload, "afk for lunch");
    let payloads: &[&[u8]] = &[
        &[],                    // empty payload
        &[0u8, 0u8, 0u8, 0u8],  // valid 0-length WSTRING header
        &valid_wstring_payload, // valid non-empty WSTRING
        &[0xFFu8, 0xFFu8],      // malformed (too short for char_count)
    ];

    for (i, payload) in payloads.iter().enumerate() {
        dispatch_sgw_player_base_method(
            sgw_player_base::CHAT_SET_AFK,
            payload,
            &Some("Tester".to_string()),
            addr,
            &transport,
            key,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .unwrap_or_else(|e| {
            panic!("chatSetAFKMessage (payload {i}) must not propagate Err: {e}")
        });
    }

    // Every per-connection field the speaker_flags wiring touches
    // must survive AFK dispatch unchanged.
    let snapshot = {
        let g = connected.lock().unwrap();
        let c = g.get(&addr).expect("client state must still be present");
        (c.player_entity_id, c.dnd_message.clone(), c.access_level)
    };
    assert_eq!(
        snapshot,
        (Some(9999), Some("on a raid".to_string()), 1),
        "chatSetAFKMessage is log-only and must not mutate connection state",
    );
}
