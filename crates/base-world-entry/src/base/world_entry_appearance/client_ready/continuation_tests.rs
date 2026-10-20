//! Stage-boundary tests for the world-entry continuation: the stages a current
//! session reaches, and where a replaced or missing session stops it.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::session_token_tests::{arm, enter, start_with_transport, state};
use crate::base::ConnectedClientState;
use crate::test_support::TestTransport;

/// Stage names in the order the continuation reaches them.
const STAGES: [&str; 10] = [
    "burst",
    "gm_redirect",
    "contact_lists",
    "org_restore",
    "plugin_hook",
    "ignore_resync",
    "mail_sweep",
    "login_fanout",
    "cinematic",
    "aoi_flush",
];

static REACHED: LazyLock<Mutex<HashMap<SocketAddr, Vec<&'static str>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Records that the continuation for `addr` passed the guard of `stage`.
pub(super) fn mark(addr: SocketAddr, stage: &'static str) {
    REACHED.lock().unwrap().entry(addr).or_default().push(stage);
}

/// Returns and clears the stages the continuation for `addr` reached.
fn reached(addr: SocketAddr) -> Vec<&'static str> {
    REACHED.lock().unwrap().remove(&addr).unwrap_or_default()
}

/// A session whose world entry is a first login, so the cinematic runs.
fn first_login_state(entity_id: u32, key: [u8; 32], generation: Instant) -> ConnectedClientState {
    let mut session = state(entity_id, 42, key, generation);
    if let Some(pending) = session.pending_client_ready.as_mut() {
        pending.first_login = 1;
    }
    session
}

#[tokio::test]
async fn continuation_runs_every_stage_in_order_for_a_current_session() {
    let addr: SocketAddr = "127.0.0.1:56010".parse().unwrap();
    let entity_id = 56010;
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        first_login_state(entity_id, [1; 32], Instant::now()),
    )])));
    let (tx, _rx) = mpsc::channel(32);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    start_with_transport(addr, connected, tx, entity_id, transport)
        .await
        .unwrap();
    assert_eq!(reached(addr), STAGES);
}

#[tokio::test]
async fn continuation_stops_at_each_stage_boundary_when_session_replaced() {
    for (index, stage) in STAGES.into_iter().enumerate() {
        let port = 56020 + index as u16;
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let entity_id = u32::from(port);
        let generation = Instant::now();
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            first_login_state(entity_id, [1; 32], generation),
        )])));
        let (tx, _rx) = mpsc::channel(32);
        let typed = Arc::new(TestTransport::default());
        let transport: Arc<dyn Transport> = typed.clone();
        let (entered, release) = arm(addr, stage);
        let task = start_with_transport(addr, connected.clone(), tx, entity_id, transport);
        enter(entered).await;

        // Same address, entity and player; new key and generation.
        connected.lock().unwrap().insert(
            addr,
            first_login_state(entity_id, [2; 32], generation + Duration::from_nanos(1)),
        );
        let sent_before = typed.filter_to(addr).len();
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(reached(addr), &STAGES[..index], "replaced at stage {stage}");
        assert_eq!(
            typed.filter_to(addr).len(),
            sent_before,
            "packet sent after replacement at stage {stage}"
        );
    }
}

/// A replacement that lands inside a callee's window, after the cinematic
/// stage guard passed and before `send_cinematic`, must not put the old
/// entry's cinematic packet on the wire under the new key.
#[tokio::test]
async fn replacement_inside_a_callee_window_drops_the_old_entry_send() {
    let addr: SocketAddr = "127.0.0.1:56040".parse().unwrap();
    let entity_id = 56040;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        first_login_state(entity_id, [1; 32], generation),
    )])));
    let (tx, _rx) = mpsc::channel(32);
    let typed = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = typed.clone();
    let (entered, release) = arm(addr, "cinematic_late");
    let task = start_with_transport(addr, connected.clone(), tx, entity_id, transport);
    enter(entered).await;

    // Same address, entity and player; new key and generation.
    connected.lock().unwrap().insert(
        addr,
        first_login_state(entity_id, [2; 32], generation + Duration::from_nanos(1)),
    );
    let sent_before = typed.filter_to(addr).len();
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();

    assert!(
        reached(addr).contains(&"cinematic"),
        "the cinematic stage guard should have passed before the replacement"
    );
    assert_eq!(
        typed.filter_to(addr).len(),
        sent_before,
        "the old entry's cinematic send reached the replaced session"
    );
}

#[tokio::test]
async fn continuation_replacement_with_no_session_for_addr_stops_quietly() {
    let addr: SocketAddr = "127.0.0.1:56030".parse().unwrap();
    let entity_id = 56030;
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        first_login_state(entity_id, [1; 32], Instant::now()),
    )])));
    let (tx, _rx) = mpsc::channel(32);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let (entered, release) = arm(addr, "org_restore");
    let task = start_with_transport(addr, connected.clone(), tx, entity_id, transport);
    enter(entered).await;

    connected.lock().unwrap().remove(&addr);
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(reached(addr), &STAGES[..3]);
}
