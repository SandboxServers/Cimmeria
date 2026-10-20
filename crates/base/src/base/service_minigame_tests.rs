//! Serial CellToBase minigame routing through the production receive helper.

use super::*;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::Notify;

struct PauseCloseTransport {
    first: AtomicBool,
    sent: AtomicUsize,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

impl Transport for PauseCloseTransport {
    fn send_to<'a, 'b, 'f>(
        &'a self,
        bytes: &'b [u8],
        _addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        Box::pin(async move {
            self.sent.fetch_add(1, Ordering::SeqCst);
            if self.first.swap(false, Ordering::SeqCst) {
                self.entered.notify_one();
                self.release.notified().await;
            }
            Ok(bytes.len())
        })
    }
    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

fn owner_maps() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    u64,
) {
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let mut state = crate::test_support::test_default_connected_client_state();
    state.player_entity_id = Some(1);
    state.active_player_id = Some(1);
    let instance = state.session_instance_id;
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(1, addr)]))),
        instance,
    )
}

async fn deposited(registry: &SessionRegistry, instance: u64) -> String {
    deposited_entity(registry, instance, 1).await
}

async fn deposited_entity(registry: &SessionRegistry, instance: u64, entity_id: u32) -> String {
    let ticket = registry
        .register(
            entity_id,
            entity_id as i32,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![100],
            None,
            None,
            instance,
        )
        .await
        .unwrap();
    let claimed = registry
        .authenticate_and_claim(entity_id, &ticket, "Hack")
        .await
        .unwrap();
    assert!(registry.deposit_result(&claimed, 1, true).await);
    ticket
}

async fn route_stale_ticket_without_outputs(
    registry: SessionRegistry,
    ticket: String,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let transport = Arc::new(crate::test_support::TestTransport::new());
    let route_transport = transport.clone() as Arc<dyn Transport>;
    let (tx, mut rx) = mpsc::channel(1);
    let (cell_tx, mut cell_rx) = mpsc::channel(1);
    tx.send(CellToBaseMsg::MinigameResult { ticket })
        .await
        .unwrap();
    drop(tx);
    let cell_channel = Some(cell_tx);
    let no_db = None;
    let route_registry = Some(registry);
    let plugins = BasePlugins::empty();
    receive_cell_messages(&mut rx, |msg| {
        route_cell_message(
            msg,
            &route_transport,
            &connected,
            &entity_to_addr,
            &cell_channel,
            &no_db,
            &route_registry,
            "127.0.0.1",
            9339,
            &plugins,
        )
    })
    .await;
    assert!(
        transport.is_empty(),
        "stale result must send no client packet"
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "stale result must not reach Cell"
    );
}

#[tokio::test(start_paused = true)]
async fn minigame_route_queued_result_expired_before_base_consumes_it() {
    let (connected, entity_to_addr, instance) = owner_maps();
    let registry = SessionRegistry::new();
    let ticket = deposited(&registry, instance).await;
    tokio::time::advance(std::time::Duration::from_secs(180)).await;
    route_stale_ticket_without_outputs(registry, ticket, connected, entity_to_addr).await;
}

#[tokio::test]
async fn minigame_route_queued_result_evicted_before_base_consumes_it() {
    let (connected, entity_to_addr, instance) = owner_maps();
    let registry = SessionRegistry::new();
    let ticket = deposited(&registry, instance).await;
    for entity_id in 2..=4097 {
        deposited_entity(&registry, instance, entity_id).await;
    }
    route_stale_ticket_without_outputs(registry, ticket, connected, entity_to_addr).await;
}

#[tokio::test]
async fn minigame_route_order_waits_for_real_result_close_before_start() {
    let (connected, entity_to_addr, instance) = owner_maps();
    let registry = SessionRegistry::new();
    let ticket = deposited(&registry, instance).await;
    assert!(registry.remove_if_ticket(1, &ticket).await);
    let transport = Arc::new(PauseCloseTransport {
        first: AtomicBool::new(true),
        sent: AtomicUsize::new(0),
        entered: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    });
    let (tx, mut rx) = mpsc::channel(2);
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    let route_transport = transport.clone() as Arc<dyn Transport>;
    let route_registry = Some(registry.clone());
    let task = tokio::spawn(async move {
        let cell_channel = Some(cell_tx);
        let no_db = None;
        let plugins = BasePlugins::empty();
        receive_cell_messages(&mut rx, |msg| {
            route_cell_message(
                msg,
                &route_transport,
                &connected,
                &entity_to_addr,
                &cell_channel,
                &no_db,
                &route_registry,
                "127.0.0.1",
                9339,
                &plugins,
            )
        })
        .await;
    });
    tx.send(CellToBaseMsg::MinigameResult { ticket })
        .await
        .unwrap();
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        transport.entered.notified(),
    )
    .await
    .unwrap();
    tx.send(CellToBaseMsg::StartMinigame {
        entity_id: 1,
        player_id: 1,
        game_name: "Hack".into(),
        difficulty: 1,
        on_victory_chains: vec![],
    })
    .await
    .unwrap();
    tokio::task::yield_now().await;
    assert_eq!(
        registry.session_count().await,
        0,
        "Start waits behind result close"
    );
    transport.release.notify_one();
    drop(tx);
    tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(registry.session_count().await, 1);
    assert_eq!(
        transport.sent.load(Ordering::SeqCst),
        2,
        "close A then launch B"
    );
    assert!(matches!(
        cell_rx.try_recv(),
        Ok(BaseToCellMsg::MinigameResult { result_code: 1, .. })
    ));
    assert!(cell_rx.try_recv().is_err());
}

#[tokio::test]
async fn minigame_route_order_start_first_invalidates_undrained_result() {
    let (connected, entity_to_addr, instance) = owner_maps();
    let registry = SessionRegistry::new();
    let ticket = deposited(&registry, instance).await;
    assert!(registry.remove_if_ticket(1, &ticket).await);
    let transport = Arc::new(crate::test_support::TestTransport::new());
    let route_transport = transport.clone() as Arc<dyn Transport>;
    let (tx, mut rx) = mpsc::channel(2);
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    tx.send(CellToBaseMsg::StartMinigame {
        entity_id: 1,
        player_id: 1,
        game_name: "Hack".into(),
        difficulty: 1,
        on_victory_chains: vec![],
    })
    .await
    .unwrap();
    tx.send(CellToBaseMsg::MinigameResult {
        ticket: ticket.clone(),
    })
    .await
    .unwrap();
    drop(tx);
    let cell_channel = Some(cell_tx);
    let no_db = None;
    let route_registry = Some(registry.clone());
    let plugins = BasePlugins::empty();
    receive_cell_messages(&mut rx, |msg| {
        route_cell_message(
            msg,
            &route_transport,
            &connected,
            &entity_to_addr,
            &cell_channel,
            &no_db,
            &route_registry,
            "127.0.0.1",
            9339,
            &plugins,
        )
    })
    .await;
    assert_eq!(registry.session_count().await, 1);
    assert!(registry.redeem_result(&ticket).await.is_none());
    assert!(
        cell_rx.try_recv().is_err(),
        "A must not receive Cell credit"
    );
    assert_eq!(
        transport.send_count_to("127.0.0.1:5000".parse().unwrap()),
        1,
        "only B URL, no A close"
    );
}

#[tokio::test]
async fn minigame_route_order_duplicate_start_preserves_live_ticket() {
    let (connected, entity_to_addr, instance) = owner_maps();
    let registry = SessionRegistry::new();
    let ticket = deposited(&registry, instance).await;
    let transport = Arc::new(crate::test_support::TestTransport::new());
    let route_transport = transport.clone() as Arc<dyn Transport>;
    let (tx, mut rx) = mpsc::channel(1);
    let (cell_tx, mut cell_rx) = mpsc::channel(1);
    tx.send(CellToBaseMsg::StartMinigame {
        entity_id: 1,
        player_id: 1,
        game_name: "Hack".into(),
        difficulty: 1,
        on_victory_chains: vec![],
    })
    .await
    .unwrap();
    drop(tx);
    let cell_channel = Some(cell_tx);
    let no_db = None;
    let route_registry = Some(registry.clone());
    let plugins = BasePlugins::empty();
    receive_cell_messages(&mut rx, |msg| {
        route_cell_message(
            msg,
            &route_transport,
            &connected,
            &entity_to_addr,
            &cell_channel,
            &no_db,
            &route_registry,
            "127.0.0.1",
            9339,
            &plugins,
        )
    })
    .await;
    assert_eq!(registry.session_count().await, 1);
    assert!(registry.authenticate(1, &ticket, "Hack").await.is_some());
    assert_eq!(
        registry.redeem_result(&ticket).await.unwrap().result_code,
        1
    );
    assert!(cell_rx.try_recv().is_err());
    assert_eq!(
        transport.send_count_to("127.0.0.1:5000".parse().unwrap()),
        1,
        "duplicate Start sends feedback without a second launch"
    );
}
