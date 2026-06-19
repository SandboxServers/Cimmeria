use super::*;

#[test]
fn decode_session_key_valid() {
    let hex = "AABBCCDD".repeat(8); // 64 chars
    let key = decode_session_key(&hex).unwrap();
    assert_eq!(key[0], 0xAA);
    assert_eq!(key[1], 0xBB);
    assert_eq!(key[2], 0xCC);
    assert_eq!(key[3], 0xDD);
}

#[test]
fn decode_session_key_too_short() {
    let result = decode_session_key("AABB");
    assert!(result.is_err());
}

/// Decode rejects non-hex characters with a clear error rather than
/// silently producing zero bytes. Auth feeds this raw from the
/// upstream POST body, so a malformed key must surface as a Phase 3
/// reject, not a silent connection with a wrong key.
#[test]
fn decode_session_key_rejects_invalid_hex() {
    // 64 chars, all non-hex characters (Z is outside [0-9A-Fa-f]).
    let s = "ZZ".repeat(32);
    assert!(decode_session_key(&s).is_err());
}

/// Mixed lowercase hex must round-trip identically to uppercase —
/// the auth service may emit either depending on upstream casing.
#[test]
fn decode_session_key_accepts_lowercase_hex() {
    let upper = "AABBCCDD".repeat(8);
    let lower = "aabbccdd".repeat(8);
    assert_eq!(
        decode_session_key(&upper).unwrap(),
        decode_session_key(&lower).unwrap(),
        "uppercase and lowercase hex must decode to the same key bytes"
    );
}

#[test]
fn parse_baseapp_login_valid() {
    let mut raw = Vec::new();
    raw.push(0x41u8); // flags
    raw.push(0x00u8); // msg_id
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.push(20u8);
    raw.extend_from_slice(b"ABCDEF1234567890ABCD");
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&3u32.to_le_bytes());

    assert_eq!(raw.len(), 41);

    let (req_id, ticket) = parse_baseapp_login(&raw).unwrap();
    assert_eq!(req_id, 0xCAFEBABE);
    assert_eq!(ticket, "ABCDEF1234567890ABCD");
}

/// Truncated body (less than the 34-byte fixed header for a valid
/// `baseAppLogin`) must reject — the auth → base seam is the first
/// validation point a malformed Phase 3 packet hits, and silently
/// continuing past truncation would index OOB on the ticket bytes.
///
/// The packet IS well-formed at the `parse_incoming` layer (valid
/// flags + footers); only the body is short. The error must come
/// from `parse_baseapp_login`'s own `body.len() < 34` check, not
/// from underflow in the upstream packet parser.
#[test]
fn parse_baseapp_login_rejects_truncated_body() {
    // FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE = 0x41. With these flags the
    // upstream parser strips two trailing footers from the buffer
    // before yielding the body (innermost → outermost):
    //   * first_req_offset (u16) — where requests begin in the body
    //   * seq_id           (u32)
    // Layout:
    //   raw = [flags][..body..][first_req_offset u16][seq_id u32]
    // We give a 13-byte body (msg_id 0x00 + word_len 25u16 + 10
    // zero filler). 13 < 34, so parse_baseapp_login's own length
    // guard fires AFTER the upstream parser succeeds.
    let mut raw = vec![0x41u8]; // flags
    raw.push(0x00u8); // msg_id
    raw.extend_from_slice(&25u16.to_le_bytes()); // word_len (well-formed)
    raw.extend_from_slice(&[0u8; 10]); // zero filler — body total = 13 bytes
    raw.extend_from_slice(&0u16.to_le_bytes()); // first_req_offset footer
    raw.extend_from_slice(&7u32.to_le_bytes()); // seq_id footer

    let err =
        parse_baseapp_login(&raw).expect_err("13-byte body must trigger the 34-byte length guard");
    let msg = err.to_string();
    assert!(
        msg.contains("body too short") || msg.contains("34"),
        "error must come from parse_baseapp_login's body-length check, got: {msg}"
    );
}

/// An unexpected msg_id must reject — the parser is keyed on 0x00
/// (`baseAppLogin`). Without this guard a misrouted packet of a
/// different shape would surface as a confusing "ticketLen wrong"
/// error instead of the clear msg_id mismatch.
#[test]
fn parse_baseapp_login_rejects_wrong_msg_id() {
    let mut raw = vec![0x41u8, 0x99u8]; // msg_id = 0x99 (not 0x00)
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.push(20u8);
    raw.extend_from_slice(b"ABCDEF1234567890ABCD");
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&3u32.to_le_bytes());
    let err = parse_baseapp_login(&raw).unwrap_err().to_string();
    assert!(
        err.contains("msg_id"),
        "error message should mention msg_id, got: {err}"
    );
}

// ── Auth → base login handoff seam ─────────────────────────────────────
//
// The Phase 3 handoff is the auth↔base seam: auth created a
// PendingLogin during shard select, and base consumes it when the
// client connects via Mercury UDP. Without these tests, a regression
// in the handoff (forgotten ticket consume, wrong key copied into
// ConnectedClientState, missing duplicate-login evict) silently
// corrupts every subsequent encrypted packet.
//
// These tests use a recording `TestTransport` so the `transport.send_to`
// calls inside handle_login complete successfully — most assertions are
// about state after the handoff, not packet bytes. The
// `login_phases_emit_ordered_byte_sequence` test below is the exception:
// it inspects the recorded fan-out to pin the phase 1→4 wire sequence.

use crate::auth::PendingLogin;
use crate::test_support::TestTransport;
use cimmeria_entity::manager::EntityManager;

fn make_pending_login(account_id: u32, key_byte: u8) -> PendingLogin {
    PendingLogin {
        account_id,
        access_level: 0,
        ticket: "TICKET00000000000001".to_string(),
        // 32-byte key encoded as 64 hex chars, all the same byte.
        session_key: format!("{:02X}", key_byte).repeat(32),
        created: Instant::now(),
    }
}

/// Helper: a recording transport so the `transport.send_to` calls inside
/// handle_login are captured rather than hitting a real socket. Tests that
/// only assert post-handoff state ignore the recorded bytes; the phase
/// sequence test drains them.
fn make_transport() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
}

/// Stop the tick-sync loop spawned by handle_login. Without this,
/// the spawned task leaks across tests (it sleeps + sends every
/// 100ms forever). Setting cancelled=true on the loop's flag ends
/// it on the next tick.
fn cancel_session(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) {
    if let Ok(map) = connected.lock() {
        if let Some(c) = map.get(&addr) {
            c.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Phase 3 happy path: a valid ticket gets consumed, the connected
/// map gets a fully-populated ConnectedClientState, and the account
/// entity is created in the entity manager. Pins the auth → base
/// seam end-to-end at the state level.
#[tokio::test]
async fn login_consumes_ticket_and_registers_connected_client_state() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55555".parse().unwrap();

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    let pending = make_pending_login(0xDEAD_BEEF, 0xAB);
    let ticket = pending.ticket.clone();
    let expected_account_id = pending.account_id;
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    // Pre-seed sanity: ticket present, no connected entries.
    assert_eq!(pending_logins.lock().unwrap().len(), 1);
    assert!(connected.lock().unwrap().is_empty());

    handle_login(
        &transport,
        addr,
        0xCAFE_BABE,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("Phase 3 handoff");

    // Ticket consumed — Phase 3 is single-shot.
    assert!(
        pending_logins.lock().unwrap().is_empty(),
        "ticket must be removed from pending_logins after consumption"
    );

    // Connected client state populated with the right account_id and
    // a key matching the session_key bytes. Any drift here breaks
    // the entire encrypted channel.
    let state = connected
        .lock()
        .unwrap()
        .get(&addr)
        .map(|c| {
            (
                c.account_id,
                c.access_level,
                c.key,
                c.world_entry_sent,
                c.char_list_sent,
            )
        })
        .expect("connected entry present");
    assert_eq!(state.0, expected_account_id, "account_id from PendingLogin");
    assert_eq!(state.1, 0, "access_level from PendingLogin");
    assert_eq!(state.2, [0xABu8; 32], "key matches the decoded session_key");
    assert!(
        !state.3,
        "world_entry_sent starts false (no playCharacter yet)"
    );
    assert!(!state.4, "char_list_sent starts false");

    cancel_session(&connected, addr);
}

/// Regression guard for the Discord auth-channel wiring: a successful
/// `handle_login` must push a `player_login` event into the Discord pipeline.
/// Inits the global runtime with a *disabled* config so no webhook/HTTP is
/// involved — `try_send`'s pre-filter short-circuits a disabled config into
/// the `filtered` counter rather than `enqueued`, and both mean "the emit
/// fired and the event reached the pipeline", so we sum them. Reverting the
/// `emit_player_login` call in `handle_login` leaves the sum flat and trips
/// this.
///
/// Reliable under nextest (process-per-test isolates the global counters).
/// The strict-`>` on the summed delta still can't false-fail under parallel
/// `cargo test` — a concurrent login sharing the process-global only adds to
/// the count.
#[tokio::test]
async fn login_pushes_discord_player_login_event() {
    let rt = cimmeria_discord::init_with_config(cimmeria_discord::Config::disabled());
    // "Entered the pipeline" = enqueued (would post) + filtered (toggle/
    // disabled short-circuit). Summing keeps the guard config-agnostic.
    let pipelined = |rt: &cimmeria_discord::DiscordRuntime| {
        let s = rt.stats();
        s.enqueued + s.filtered
    };
    let before = pipelined(rt);

    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55557".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    let pending = make_pending_login(0x0000_1234, 0xCD);
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    handle_login(
        &transport,
        addr,
        0x1234_5678,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("Phase 3 handoff");

    let after = pipelined(rt);
    assert!(
        after > before,
        "successful handle_login must push a player_login Discord event into \
         the pipeline (enqueued+filtered {before} -> {after})"
    );

    cancel_session(&connected, addr);
}

/// Domain F (fan-out byte test): with no duplicate session, `handle_login`
/// emits exactly two packets, in order — the Phase 3 connect-reply (seq 1)
/// then the initial time-sync bundle (seq 2) — both to the connecting
/// addr, byte-exact. This pins the auth→base handshake *wire* sequence the
/// state-level tests above can't see (no old-addr cleanup fires because
/// `connected` starts empty). The spawned tick-sync loop sleeps 100 ms
/// before its first send, and we cancel it immediately, so a synchronous
/// drain captures only the handshake.
#[tokio::test]
async fn login_emits_ordered_connect_reply_then_time_sync_bytes() {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55556".parse().unwrap();

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    let key = [0xABu8; 32];
    let request_id = 0xCAFE_BABEu32;
    let pending = make_pending_login(0xDEAD_BEEF, 0xAB);
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    handle_login(
        &dyn_transport,
        addr,
        request_id,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("Phase 3 handoff");

    // Stop the tick loop before it can append a third packet.
    cancel_session(&connected, addr);

    let sent = transport.drain();
    assert_eq!(
        sent.len(),
        2,
        "no duplicate session ⇒ exactly connect_reply + time_sync (no old-addr cleanup)"
    );
    assert_eq!(sent[0].0, addr, "connect_reply goes to the connecting addr");
    assert_eq!(sent[1].0, addr, "time_sync goes to the connecting addr");
    assert_eq!(
        sent[0].1,
        build_connect_reply(request_id, ticket.as_bytes(), &key, 1),
        "phase-3 connect_reply bytes (seq 1)"
    );
    assert_eq!(
        sent[1].1,
        build_time_sync(&key, 2),
        "initial time_sync bytes (seq 2)"
    );
}

/// Phase 3 with an unknown ticket must NOT register a connected
/// state — the function logs and returns Ok. Without this, a
/// replayed-ticket packet from a stale client could create a
/// half-initialized ConnectedClientState (no key, no account).
#[tokio::test]
async fn login_with_unknown_ticket_does_not_register_state() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55556".parse().unwrap();

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    // pending_logins is empty — any ticket lookup must miss.
    handle_login(
        &transport,
        addr,
        1,
        "DOES_NOT_EXIST_00000",
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("Phase 3 with unknown ticket returns Ok and logs");

    assert!(
        connected.lock().unwrap().is_empty(),
        "unknown ticket must not produce a ConnectedClientState"
    );
}

/// Duplicate-login eviction (KI-7): when a second Phase 3 lands for
/// the same account_id from a different SocketAddr, the prior
/// session is evicted — its addr removed from `connected` and
/// LOGGED_OFF dispatched to the old client. Without this, two
/// active sessions per account corrupt entity state on both sides.
#[tokio::test]
async fn second_login_for_same_account_evicts_first_session() {
    let transport = make_transport();
    let addr_a: SocketAddr = "127.0.0.1:55557".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:55558".parse().unwrap();

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    const ACCOUNT_ID: u32 = 0xC0FF_EE42;

    // Session A: original login.
    let mut p_a = make_pending_login(ACCOUNT_ID, 0x01);
    p_a.ticket = "TICKETA000000000001A".to_string();
    let ticket_a = p_a.ticket.clone();
    pending_logins.lock().unwrap().insert(ticket_a.clone(), p_a);
    handle_login(
        &transport,
        addr_a,
        1,
        &ticket_a,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("first login");
    assert!(
        connected.lock().unwrap().contains_key(&addr_a),
        "session A registered after first login"
    );

    // Session B: same account_id, different addr — must evict A.
    let mut p_b = make_pending_login(ACCOUNT_ID, 0x02);
    p_b.ticket = "TICKETB000000000002B".to_string();
    let ticket_b = p_b.ticket.clone();
    pending_logins.lock().unwrap().insert(ticket_b.clone(), p_b);
    handle_login(
        &transport,
        addr_b,
        2,
        &ticket_b,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
    )
    .await
    .expect("second login");

    let map = connected.lock().unwrap();
    assert!(
        !map.contains_key(&addr_a),
        "addr_a (session A) must be evicted when account_id collides on a new addr",
    );
    assert!(
        map.contains_key(&addr_b),
        "addr_b (session B) takes the slot"
    );
    assert_eq!(
        map.get(&addr_b).unwrap().key,
        [0x02u8; 32],
        "session B's key must come from its OWN PendingLogin, not A's stale one"
    );
    drop(map);

    cancel_session(&connected, addr_b);
}
