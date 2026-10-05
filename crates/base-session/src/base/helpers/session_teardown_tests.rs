//! Guards for [`destroy_owned_client_entities`], the tick-sync loop's
//! owner-checked teardown.
//!
//! A client killed and relaunched within the 60 s inactivity window comes
//! back on the same address:port (the SGW client binds a fixed UDP port),
//! and the base replaces the session at that address with the new login
//! (`relaunch_takeover`). The old session's tick-sync loop is still alive
//! until its next tick. If it had already decided to time out, its
//! teardown keyed only on the address would destroy the *new* session.
//! These tests pin that the loop only ever tears down the session it was
//! started for.

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::time::{Duration, Instant};

use super::*;

use crate::test_support::{test_default_connected_client_state, TestTransport};

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

fn transport() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
}

/// The session that took the address over, with its own cancel flag.
fn replacement_session(addr: SocketAddr) -> (Connected, Arc<AtomicBool>) {
    let state = test_default_connected_client_state();
    let flag = Arc::clone(&state.cancelled);
    (Arc::new(Mutex::new(HashMap::from([(addr, state)]))), flag)
}

#[test]
fn owned_teardown_leaves_a_session_it_does_not_own() {
    let addr: SocketAddr = "127.0.0.1:52501".parse().unwrap();
    let (connected, new_flag) = replacement_session(addr);
    let stale_owner = Arc::new(AtomicBool::new(true));

    let torn_down = destroy_owned_client_entities(
        &connected,
        &Arc::new(Mutex::new(EntityManager::new())),
        addr,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &transport(),
        &None,
        "inactivity_timeout",
        &stale_owner,
    );

    assert!(!torn_down, "a stale owner must not tear anything down");
    assert!(
        connected.lock().unwrap().contains_key(&addr),
        "the replacement session must stay registered"
    );
    assert!(
        !new_flag.load(Ordering::Relaxed),
        "the replacement session's tick loop must not be cancelled"
    );
}

#[test]
fn owned_teardown_removes_its_own_session() {
    let addr: SocketAddr = "127.0.0.1:52502".parse().unwrap();
    let (connected, own_flag) = replacement_session(addr);

    let torn_down = destroy_owned_client_entities(
        &connected,
        &Arc::new(Mutex::new(EntityManager::new())),
        addr,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &transport(),
        &None,
        "inactivity_timeout",
        &own_flag,
    );

    assert!(torn_down);
    assert!(!connected.lock().unwrap().contains_key(&addr));
    assert!(own_flag.load(Ordering::Relaxed));
}

/// PR #1246 review, finding 3b: teardown also stops a first-login
/// cinematic's appearance re-send loop. That loop reads whatever session
/// holds the address each round, so after a relaunch takeover it would
/// paint the new session's appearance onto the old entity id. Fails with
/// the `cinematic_spam_cancel` store removed from `teardown_session`.
#[test]
fn teardown_stops_the_cinematic_appearance_loop() {
    let addr: SocketAddr = "127.0.0.1:52504".parse().unwrap();
    let (connected, _) = replacement_session(addr);
    let spam_cancel = Arc::clone(&connected.lock().unwrap()[&addr].cinematic_spam_cancel);
    assert!(!spam_cancel.load(Ordering::Relaxed));

    destroy_client_entities(
        &connected,
        &Arc::new(Mutex::new(EntityManager::new())),
        addr,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &transport(),
        &None,
        "relaunch_takeover",
    );

    assert!(
        spam_cancel.load(Ordering::Relaxed),
        "the cinematic re-send loop must be told to stop with the session"
    );
}

/// End to end through the loop: an old session's tick loop that times out
/// after its address was taken over leaves the new session alone. Before
/// the owner check, the loop's teardown keyed on the address alone and
/// destroyed the relaunched client's session.
#[tokio::test]
async fn timed_out_tick_loop_does_not_destroy_the_session_that_replaced_it() {
    let addr: SocketAddr = "127.0.0.1:52503".parse().unwrap();
    let (connected, new_flag) = replacement_session(addr);
    // The old loop's state: silent past the 60 s reap, its own flag never
    // set (the race where it decided to time out before the takeover).
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(61))
        .expect("host uptime must exceed 61 s to backdate last_recv");
    let old_flag = Arc::new(AtomicBool::new(false));

    crate::base::tick_sync::run_tick_loop(
        transport(),
        addr,
        [0u8; 32],
        Default::default(),
        Arc::new(AtomicU32::new(0)),
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(long_ago)),
        old_flag,
        Arc::clone(&connected),
        Arc::new(Mutex::new(EntityManager::new())),
        None,
        Arc::new(Mutex::new(HashMap::new())),
        None,
    )
    .await;

    assert!(
        connected.lock().unwrap().contains_key(&addr),
        "the old loop's timeout must not remove the relaunched client's session"
    );
    assert!(!new_flag.load(Ordering::Relaxed));
}
