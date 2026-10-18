//! Deterministic session-replacement and ordered-handoff tests.

use super::*;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

type Pause = (oneshot::Sender<()>, oneshot::Receiver<()>);
static PAUSES: LazyLock<Mutex<HashMap<(SocketAddr, &'static str), Pause>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) async fn pause_after_pending(addr: SocketAddr) {
    pause_after(addr, "after_pending").await;
}

pub(super) async fn pause_after(addr: SocketAddr, stage: &'static str) {
    let pause = PAUSES.lock().unwrap().remove(&(addr, stage));
    if let Some((entered, release)) = pause {
        let _ = entered.send(());
        let _ = release.await;
    }
}

pub(super) fn arm(
    addr: SocketAddr,
    stage: &'static str,
) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    assert!(PAUSES
        .lock()
        .unwrap()
        .insert((addr, stage), (entered_tx, release_rx))
        .is_none());
    (entered_rx, release_tx)
}

pub(super) fn state(
    entity_id: u32,
    player_id: i32,
    key: [u8; 32],
    generation: Instant,
) -> ConnectedClientState {
    let mut state = test_default_connected_client_state();
    state.key = key;
    state.connected_at = generation;
    state.player_entity_id = Some(entity_id);
    state.active_player_id = Some(player_id);
    state.player_name = Some("Tester".to_string());
    state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
        entity_id,
        player_id,
        world_name: "Harset_CmdCenter".into(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0,
    });
    state
}

type Clients = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

fn start(
    addr: SocketAddr,
    connected: Clients,
    tx: mpsc::Sender<BaseToCellMsg>,
    entity_id: u32,
) -> tokio::task::JoinHandle<()> {
    start_with_transport(
        addr,
        connected,
        tx,
        entity_id,
        Arc::new(TestTransport::default()),
    )
}

pub(super) fn start_with_transport(
    addr: SocketAddr,
    connected: Clients,
    tx: mpsc::Sender<BaseToCellMsg>,
    entity_id: u32,
    transport: Arc<dyn Transport>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        handle_on_client_ready(
            addr,
            [0; 32],
            &connected,
            &Some(tx),
            &transport,
            &entity_to_addr,
            &None,
        )
        .await
        .unwrap();
    })
}

#[tokio::test]
async fn on_client_ready_stops_old_continuation_after_ring_permit_replacement() {
    let addr: SocketAddr = "127.0.0.1:55778".parse().unwrap();
    let entity_id = 55778;
    let generation = Instant::now();
    let mut old = state(entity_id, 42, [1; 32], generation);
    old.pending_destination_ring_id = Some(17);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, old)])));
    let (tx, mut rx) = mpsc::channel(4);
    let typed = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = typed.clone();
    let (entered, release) = arm(addr, "ring_permit_acquired");
    let task = start_with_transport(addr, connected.clone(), tx, entity_id, transport);
    enter(entered).await;

    // Replacement reuses all routing IDs while the old handler owns capacity.
    connected.lock().unwrap().insert(
        addr,
        state(entity_id, 42, [2; 32], generation + Duration::from_nanos(1)),
    );
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();

    let mut kinds = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        kinds.push(match msg {
            BaseToCellMsg::ConnectEntity { .. } => "connect",
            BaseToCellMsg::LetterTurnInSession { .. } => "token",
            BaseToCellMsg::InitPlayerState { .. } => "init",
            BaseToCellMsg::AdvanceRingDestination { .. } => "ring",
            _ => "other",
        });
    }
    assert_eq!(kinds, ["connect", "token", "init"]);
    assert!(
        typed.filter_to(addr).is_empty(),
        "old handler sent post-handoff packets"
    );
}

pub(super) async fn enter(pause: oneshot::Receiver<()>) {
    tokio::time::timeout(Duration::from_secs(3), pause)
        .await
        .expect("pause not reached")
        .expect("pause sender dropped");
}

#[tokio::test]
async fn on_client_ready_queues_session_token_before_init() {
    let addr: SocketAddr = "127.0.0.1:55770".parse().unwrap();
    let entity_id = 55770;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        state(entity_id, 42, [9; 32], generation),
    )])));
    let (tx, mut rx) = mpsc::channel(32);
    start(addr, connected.clone(), tx.clone(), entity_id)
        .await
        .unwrap();
    let mut order = Vec::new();
    let mut token = None;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            BaseToCellMsg::ConnectEntity { .. } => order.push("connect"),
            BaseToCellMsg::LetterTurnInSession { session, .. } => {
                order.push("token");
                token = Some(session);
            }
            BaseToCellMsg::InitPlayerState { .. } => order.push("init"),
            _ => {}
        }
    }
    assert_eq!(order, ["connect", "token", "init"]);
    let token = token.unwrap();
    assert_eq!(token.generation, generation);
    assert_eq!(token.player_id, 42);
    assert_eq!(
        token.token.bytes(),
        [
            0xc7, 0xd4, 0xa0, 0x1c, 0x23, 0x52, 0x49, 0x2e, 0x4b, 0xe6, 0xb9, 0x8e, 0xa0, 0xa0,
            0x89, 0xec
        ]
    );
    assert!(!format!("{token:?}").contains(&format!("{:x?}", token.token.bytes())));

    // Gate travel takes a fresh pending finalization on the same session.
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .pending_client_ready = state(entity_id, 42, [9; 32], generation).pending_client_ready;
    start(addr, connected, tx, entity_id).await.unwrap();
    let mut travel = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::LetterTurnInSession { session, .. } = msg {
            travel = Some(session);
        }
    }
    let travel = travel.unwrap();
    assert_eq!(travel.token, token.token);
    assert_eq!(travel.generation, generation);
    assert!(travel.epoch > token.epoch);
}

#[tokio::test]
async fn on_client_ready_relog_changes_token_for_reused_player_and_entity() {
    let addr: SocketAddr = "127.0.0.1:55777".parse().unwrap();
    let entity_id = 55777;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        state(entity_id, 42, [1; 32], generation),
    )])));
    let (tx, mut rx) = mpsc::channel(32);
    start(addr, connected.clone(), tx.clone(), entity_id)
        .await
        .unwrap();
    let mut old = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::LetterTurnInSession { session, .. } = msg {
            old = Some(session);
        }
    }
    connected.lock().unwrap().insert(
        addr,
        state(entity_id, 42, [2; 32], generation + Duration::from_nanos(1)),
    );
    start(addr, connected, tx, entity_id).await.unwrap();
    let mut new = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::LetterTurnInSession { session, .. } = msg {
            new = Some(session);
        }
    }
    assert_ne!(new.unwrap().token, old.unwrap().token);
}

#[tokio::test]
async fn on_client_ready_drops_replaced_session_after_db_wait() {
    let addr: SocketAddr = "127.0.0.1:55771".parse().unwrap();
    let entity_id = 55771;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        state(entity_id, 42, [1; 32], generation),
    )])));
    let (tx, mut rx) = mpsc::channel(32);
    let (entered, release) = arm(addr, "after_pending");
    let task = start(addr, connected.clone(), tx, entity_id);
    enter(entered).await;
    connected.lock().unwrap().insert(
        addr,
        state(entity_id, 42, [2; 32], generation + Duration::from_nanos(1)),
    );
    release.send(()).unwrap();
    task.await.unwrap();
    while let Ok(msg) = rx.try_recv() {
        assert!(!matches!(
            msg,
            BaseToCellMsg::ConnectEntity { .. }
                | BaseToCellMsg::LetterTurnInSession { .. }
                | BaseToCellMsg::InitPlayerState { .. }
        ));
    }
}

#[tokio::test]
async fn on_client_ready_drops_old_permit_after_replacement() {
    let addr: SocketAddr = "127.0.0.1:55772".parse().unwrap();
    let entity_id = 55772;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        state(entity_id, 42, [1; 32], generation),
    )])));
    let (tx, mut rx) = mpsc::channel(1);
    let (entered, release) = arm(addr, "after_pending");
    let (permit_entered, permit_release) = arm(addr, "connect_permit_acquired");
    let task = start(addr, connected.clone(), tx.clone(), entity_id);
    enter(entered).await;
    tx.send(BaseToCellMsg::ConnectEntity { entity_id: 0 })
        .await
        .unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        rx.recv().await,
        Some(BaseToCellMsg::ConnectEntity { entity_id: 0 })
    ));
    enter(permit_entered).await;
    // The old handler has reserved capacity. Reuse the same IDs before send.
    connected.lock().unwrap().insert(
        addr,
        state(entity_id, 42, [2; 32], generation + Duration::from_nanos(1)),
    );
    permit_release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        rx.try_recv().is_err(),
        "old session enqueued after replacement"
    );
}

#[tokio::test]
async fn on_client_ready_drops_old_harset_entry_with_same_key_and_generation() {
    let addr: SocketAddr = "127.0.0.1:55776".parse().unwrap();
    let entity_id = 55776;
    let generation = Instant::now();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        state(entity_id, 42, [1; 32], generation),
    )])));
    let (tx, mut rx) = mpsc::channel(32);
    let (entered, release) = arm(addr, "connect_permit_acquired");
    let old_task = start(addr, connected.clone(), tx.clone(), entity_id);
    enter(entered).await;
    let old_epoch = *connected.lock().unwrap()[&addr]
        .extensions
        .get::<cimmeria_wire::cell::messages::LetterTurnInEntryEpoch>()
        .unwrap();
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .pending_client_ready = state(entity_id, 42, [1; 32], generation).pending_client_ready;
    start(addr, connected.clone(), tx, entity_id).await.unwrap();
    release.send(()).unwrap();
    old_task.await.unwrap();
    let mut token_epochs = Vec::new();
    let mut connects = 0;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            BaseToCellMsg::ConnectEntity { entity_id: id } if id == entity_id => connects += 1,
            BaseToCellMsg::LetterTurnInSession { session, .. } => token_epochs.push(session.epoch),
            _ => {}
        }
    }
    assert_eq!(connects, 1, "old entry must not enqueue after permit");
    assert_eq!(token_epochs.len(), 1);
    assert!(token_epochs[0] > old_epoch);
}

#[tokio::test]
async fn on_client_ready_logs_each_later_handoff_failure() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    for (port, stage, expected) in [
        (
            55773,
            "after_connect",
            "LetterTurnInSession: base→cell send failed",
        ),
        (
            55774,
            "after_token",
            "InitPlayerState: base→cell send failed",
        ),
        (
            55775,
            "after_init",
            "AdvanceRingDestination: base→cell send failed",
        ),
    ] {
        let capture = LogCapture::install();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let entity_id = port as u32;
        let mut client = state(entity_id, 42, [1; 32], Instant::now());
        client.pending_destination_ring_id = Some(17);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, client)])));
        let (tx, rx) = mpsc::channel(8);
        let (entered, release) = arm(addr, stage);
        let task = start(addr, connected, tx, entity_id);
        enter(entered).await;
        drop(rx);
        release.send(()).unwrap();
        task.await.unwrap();
        assert!(
            capture.find_message(Level::ERROR, expected).is_some(),
            "missing {expected}"
        );
        if stage == "after_connect" {
            assert!(capture
                .find_message(Level::ERROR, "InitPlayerState: base→cell send failed")
                .is_none());
        }
    }
}
