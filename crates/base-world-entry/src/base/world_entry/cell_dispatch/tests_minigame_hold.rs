//! MG-06: base-side hold release paths (split out of `minigame.rs` for the
//! file-size cap).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::minigame::{cancel_minigame, start_minigame, MinigamePlayerParams};
use crate::cell::messages::BaseToCellMsg;
use crate::minigame::SessionRegistry;
use crate::test_support::{test_default_connected_client_state, TestTransport};

const TEST_PARAMS: MinigamePlayerParams = MinigamePlayerParams {
    player_level: 1,
    intelligence: 0,
    tech_competency: 1,
    abilities_mask: 0,
};

/// Register an unconnected Livewire session for entity 77 and return its
/// registry, a cancel-ready fixture and the cell receiver.
async fn pending_session() -> (
    SessionRegistry,
    SocketAddr,
    Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    mpsc::Sender<BaseToCellMsg>,
    mpsc::Receiver<BaseToCellMsg>,
    String,
) {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            77,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(addr.ip()),
        )
        .await
        .unwrap();
    let (tx, rx) = mpsc::channel(4);
    (registry, addr, connected, entity_to_addr, tx, rx, ticket)
}

/// Review N1/B1: closing the window before the SWF connects leaves no task to
/// report, so the base must send result 3 itself. Fails with that send removed:
/// the channel stays empty and the cell keeps the lock until relog.
#[tokio::test]
async fn cancel_before_connect_frees_the_player() {
    let (registry, _addr, connected, entity_to_addr, tx, mut rx, _ticket) = pending_session().await;

    cancel_minigame(
        77,
        &Some(registry.clone()),
        &connected,
        &entity_to_addr,
        &Some(tx),
    )
    .await;

    assert_eq!(registry.session_count().await, 0);
    assert!(matches!(
        rx.try_recv(),
        Ok(BaseToCellMsg::MinigameResult {
            entity_id: 77,
            result_code: 3,
            ..
        })
    ));
}

/// A connected session's own task reports Canceled; a second report here
/// would double-release.
#[tokio::test]
async fn cancel_after_connect_leaves_the_report_to_the_task() {
    let (registry, _addr, connected, entity_to_addr, tx, mut rx, ticket) = pending_session().await;
    registry
        .authenticate_and_claim(77, &ticket, "Livewire")
        .await
        .expect("claim must succeed");

    cancel_minigame(
        77,
        &Some(registry.clone()),
        &connected,
        &entity_to_addr,
        &Some(tx),
    )
    .await;

    assert!(rx.try_recv().is_err(), "the connection task reports it");
}
/// MG-06: the cell took the minigame hold when it sent StartMinigame, so a
/// launch the base cannot carry out must send result 3 back or the player
/// stays locked with BSF_PlayingMinigame set.
#[tokio::test]
async fn launch_failure_with_server_disabled_frees_the_player() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let transport = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
    let (cell_tx, mut cell_rx) = mpsc::channel(2);

    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &None,
        "203.0.113.1",
        9339,
        &Some(cell_tx),
    )
    .await;

    assert_eq!(transport.send_count_to(addr), 1, "feedback line still sent");
    assert!(matches!(
        cell_rx.try_recv(),
        Ok(BaseToCellMsg::MinigameResult {
            entity_id: 77,
            result_code: 3,
            ..
        })
    ));
}

/// No client address: the same free-the-player result.
#[tokio::test]
async fn launch_failure_without_client_addr_frees_the_player() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let (cell_tx, mut cell_rx) = mpsc::channel(2);

    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &transport,
        &connected,
        &entity_to_addr,
        &Some(SessionRegistry::new()),
        "203.0.113.1",
        9339,
        &Some(cell_tx),
    )
    .await;

    assert!(matches!(
        cell_rx.try_recv(),
        Ok(BaseToCellMsg::MinigameResult { result_code: 3, .. })
    ));
}

/// A duplicate refusal must not send result 3: the first board is still
/// running, and the cell would free its hold.
#[tokio::test]
async fn duplicate_refusal_does_not_free_the_running_games_hold() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
    let registry = Some(SessionRegistry::new());
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    for _ in 0..2 {
        start_minigame(
            77,
            1,
            "Livewire".into(),
            1,
            vec![],
            vec![],
            TEST_PARAMS,
            &transport,
            &connected,
            &entity_to_addr,
            &registry,
            "203.0.113.1",
            9339,
            &Some(cell_tx.clone()),
        )
        .await;
    }
    assert!(
        cell_rx.try_recv().is_err(),
        "no result for a refused duplicate"
    );
}
