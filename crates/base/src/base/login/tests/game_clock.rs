//! Every session reports the one server-wide game clock.
//!
//! Previously each session's tick-sync loop counted from 0 at its own
//! login, so two clients held two different clocks and the cell could not
//! build an absolute `onTimerUpdate` expiry either of them would read the
//! same way. These tests drive `handle_login` and read the `tickSync`
//! packets its loop actually sends.

use std::time::Duration;

use cimmeria_mercury::encryption::MercuryEncryption;

use super::*;
use crate::mercury::game_clock;

/// Ticks the clock must have passed before the login, so a per-session
/// counter (0, 1, 2, ...) cannot land inside the window by accident.
const MIN_TICKS_BEFORE_LOGIN: u32 = 5;

/// `BASEMSG_TICK_SYNC` (`pub(crate)` in `cimmeria-wire`).
const TICK_SYNC_MSG_ID: u8 = 0x0D;

/// `tickSync` gameTime from a decrypted unreliable heartbeat:
/// `[flags][0x0D][gameTime:u32][tickRate:u32][seq:u32]`.
fn tick_sync_game_time(plaintext: &[u8]) -> Option<u32> {
    (plaintext.get(1) == Some(&TICK_SYNC_MSG_ID))
        .then(|| u32::from_le_bytes(plaintext[2..6].try_into().unwrap()))
}

/// The heartbeat carries the server-wide game clock, not a count of this
/// session's sends: every `tickSync` the loop sends reads a tick between
/// the clock before the login and the clock after the drain.
#[tokio::test]
async fn tick_sync_heartbeat_carries_the_server_wide_game_clock() {
    game_clock::init();
    while game_clock::game_ticks() < MIN_TICKS_BEFORE_LOGIN {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55571".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let key = [0xC2u8; 32];
    let pending = make_pending_login(0x7000_C201, 0xC2);
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    let ticks_before = game_clock::game_ticks();
    handle_login(
        &dyn_transport,
        addr,
        1,
        &ticket,
        &pending_logins,
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("Phase 3 handoff");

    // Three or so heartbeats at 100 ms.
    tokio::time::sleep(game_clock::TICK_SYNC_INTERVAL * 3 + Duration::from_millis(50)).await;
    cancel_session(&connected, addr);
    let ticks_after = game_clock::game_ticks();

    let crypto = MercuryEncryption::from_session_key(key);
    let heartbeats: Vec<u32> = transport
        .drain()
        .into_iter()
        .skip(2) // connect_reply, time_sync
        .filter_map(|(_, bytes)| crypto.decrypt(&bytes).ok())
        .filter_map(|p| tick_sync_game_time(&p))
        .collect();

    assert!(
        !heartbeats.is_empty(),
        "the loop sent no tickSync in {}ms",
        (game_clock::TICK_SYNC_INTERVAL * 3).as_millis()
    );
    for tick in &heartbeats {
        assert!(
            (ticks_before..=ticks_after).contains(tick),
            "tickSync gameTime {tick} is not the server clock [{ticks_before},              {ticks_after}] (all heartbeats: {heartbeats:?})"
        );
    }
}
