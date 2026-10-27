use super::*;

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

use crate::test_support::TestTransport;
use cimmeria_entity::manager::EntityManager;

mod fixtures;
use fixtures::make_pending_login;

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
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
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

/// TG-NET-16: Phase 3 turns the ticket's `test_account` flag into the
/// session's `session_kind` ("lab" for a test account, "player" otherwise).
/// Reverting the copy in `handle_login` leaves `session_kind` unset and
/// trips this.
#[tokio::test]
async fn login_resolves_session_kind_from_test_account_flag() {
    let transport = make_transport();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    for (port, account_id, test_account, expected) in [
        (55561u16, 0x1616_0001u32, true, "lab"),
        (55562u16, 0x1616_0002u32, false, "player"),
    ] {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let mut pending = make_pending_login(account_id, 0x16);
        pending.test_account = test_account;
        pending.ticket = format!("TICKETTGNET16{port:07}");
        let ticket = pending.ticket.clone();
        pending_logins
            .lock()
            .unwrap()
            .insert(ticket.clone(), pending);

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
            &None,
            cimmeria_mercury::encryption::EncryptionVersion::V1,
            &cimmeria_base_session::base::plugin::BasePlugins::empty(),
        )
        .await
        .expect("Phase 3 handoff");

        let kind = connected
            .lock()
            .unwrap()
            .get(&addr)
            .map(|c| c.session_kind)
            .expect("connected entry present");
        assert_eq!(
            kind,
            Some(expected),
            "test_account={test_account} must resolve to session_kind={expected}"
        );
        cancel_session(&connected, addr);
    }
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
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
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
///
/// The time-sync bundle carries the server's game clock at login, not 0.
/// The test waits until the clock has passed tick 0, then checks
/// the bytes against the ticks read back from the packet, bracketed by the
/// clock before and after the call.
#[tokio::test]
async fn login_emits_ordered_connect_reply_then_time_sync_bytes() {
    use crate::mercury::game_clock;
    use cimmeria_mercury::encryption::MercuryEncryption;

    game_clock::init();
    while game_clock::game_ticks() == 0 {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
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

    let ticks_before = game_clock::game_ticks();
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
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("Phase 3 handoff");
    let ticks_after = game_clock::game_ticks();

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
        build_connect_reply(
            request_id,
            ticket.as_bytes(),
            &key,
            1,
            cimmeria_mercury::encryption::EncryptionVersion::V1,
        ),
        "phase-3 connect_reply bytes (seq 1)"
    );
    // Plaintext: [flags][0x02 hz][0x0D ticks:u32 rate:u32][0x03 ticks:u32][seq].
    let plaintext = MercuryEncryption::from_session_key(key)
        .decrypt(&sent[1].1)
        .expect("time_sync decrypts under the session key");
    let sent_ticks = u32::from_le_bytes(plaintext[4..8].try_into().unwrap());
    assert!(
        (ticks_before..=ticks_after).contains(&sent_ticks),
        "TICK_SYNC carries the game clock at login: {sent_ticks} not in          [{ticks_before}, {ticks_after}]"
    );
    assert_eq!(
        sent[1].1,
        build_time_sync(
            &key,
            2,
            sent_ticks,
            cimmeria_mercury::encryption::EncryptionVersion::V1
        ),
        "initial time_sync bytes (seq 2), SET_GAME_TIME equal to TICK_SYNC"
    );
}

/// Phase 3 with an unknown ticket must NOT register a connected
/// state — the function logs and returns Ok. Without this, a
/// replayed-ticket packet from a stale client could create a
/// half-initialized ConnectedClientState (no key, no account).
///
/// The miss must be logged with a redacted `ticket_prefix`, never the
/// full client-supplied ticket.
#[tokio::test]
async fn login_with_unknown_ticket_does_not_register_state() {
    const TICKET: &str = "DOES_NOT_EXIST_00000";
    let capture = crate::test_support::LogCapture::install();
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
        TICKET,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("Phase 3 with unknown ticket returns Ok and logs");

    assert!(
        connected.lock().unwrap().is_empty(),
        "unknown ticket must not produce a ConnectedClientState"
    );

    let miss = capture
        .find_message(tracing::Level::WARN, "Unknown or already-consumed ticket")
        .expect("unknown ticket must be logged at WARN");
    assert_eq!(
        miss.fields.get("ticket_prefix").map(String::as_str),
        Some("DOES_N…"),
        "the miss must carry a redacted ticket_prefix"
    );
    for event in capture.all() {
        assert!(
            !event.message.as_deref().unwrap_or("").contains(TICKET)
                && !event.fields.values().any(|v| v.contains(TICKET)),
            "full ticket leaked into a log event: {event:?}"
        );
    }
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
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
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
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
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

/// **Cross-IP ticket guard (#442).** A Phase-2 ticket issued to IP A and
/// consumed from IP B at Phase 3 is the replay signature of a harvested
/// ticket and must log a WARN with `reason = "ticket_ip_mismatch"`. The
/// ticket is still consumed (warn-only first — see the issue's NAT / IP
/// roaming caveat), so the assertion is the log event. Reverting the check
/// in `handle_login` removes the event and trips this guard.
#[tokio::test]
async fn login_from_different_ip_logs_ticket_ip_mismatch() {
    let capture = crate::test_support::LogCapture::install();
    let transport = make_transport();
    let addr: SocketAddr = "203.0.113.20:56789".parse().unwrap();

    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    // Ticket issued to a different IP (198.51.100.10) than the connecting
    // Mercury addr above.
    let mut pending = make_pending_login(0x4420_0002, 0x42);
    pending.client_ip = "198.51.100.10".parse().unwrap();
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    handle_login(
        &transport,
        addr,
        1,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("Phase 3 with a valid-but-foreign-IP ticket returns Ok and logs");

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "consumed from a different IP",
            "ticket_ip_mismatch",
        )
        .expect("cross-IP ticket consumption must log reason=ticket_ip_mismatch at WARN");
    assert_eq!(
        warn.fields.get("ticket_ip").map(String::as_str),
        Some("198.51.100.10")
    );
    assert_eq!(
        warn.fields.get("client_ip").map(String::as_str),
        Some("203.0.113.20")
    );
}

mod duplicate_liveness;
mod game_clock;
mod handshake_retransmit;
mod named_fields;
mod parse;
mod ticket_age;
mod ticket_revoked;
mod ticket_unknown;
