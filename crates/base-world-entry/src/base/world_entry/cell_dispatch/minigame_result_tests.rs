//! Ticket-only result dispatch guards.

use super::*;
use crate::test_support::{LogCapture, TestTransport};
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CloseTakeoverTransport {
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    sends: AtomicUsize,
}

impl Transport for CloseTakeoverTransport {
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
            self.sends.fetch_add(1, Ordering::SeqCst);
            let mut replacement = crate::test_support::test_default_connected_client_state();
            replacement.player_entity_id = Some(10);
            replacement.active_player_id = Some(1);
            self.connected
                .lock()
                .unwrap()
                .insert(self.addr, replacement);
            Ok(bytes.len())
        })
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

async fn result_fixture(
    code: u8,
    chains: Vec<i64>,
    notify: bool,
) -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    crate::minigame::SessionRegistry,
    String,
) {
    use crate::test_support::test_default_connected_client_state;
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(10);
    state.active_player_id = Some(1);
    let instance = state.session_instance_id;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(10, addr)])));
    let registry = crate::minigame::SessionRegistry::new();
    let ticket = registry
        .register(
            10,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            chains,
            None,
            Some(addr.ip()),
            instance,
        )
        .await
        .unwrap();
    let claimed = registry
        .authenticate_and_claim(10, &ticket, "Livewire")
        .await
        .unwrap();
    assert!(registry.deposit_result(&claimed, code, notify).await);
    (connected, entity_to_addr, registry, ticket)
}

#[tokio::test]
async fn minigame_result_forwards_receipt_to_cell_once() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr, registry, ticket) =
        result_fixture(1, vec![100, 200], false).await;
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    for _ in 0..2 {
        handle_cell_message(
            CellToBaseMsg::MinigameResult {
                ticket: ticket.clone(),
            },
            &transport,
            &connected,
            &entity_to_addr,
            &Some(cell_tx.clone()),
            &None,
            &Some(registry.clone()),
            "127.0.0.1",
            7777,
        )
        .await;
    }
    assert!(
        matches!(cell_rx.try_recv(), Ok(BaseToCellMsg::MinigameResult {
        entity_id: 10, player_id: 1, result_code: 1, on_victory_chains,
    }) if on_victory_chains == vec![100, 200])
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "duplicate ticket must not forward"
    );
    assert!(registry.redeem_result(&ticket).await.is_none());
}

#[tokio::test]
async fn minigame_result_with_dropped_cell_receiver_logs_error() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr, registry, ticket) =
        result_fixture(1, vec![100, 200], false).await;
    let (cell_tx, cell_rx) = mpsc::channel(1);
    drop(cell_rx);
    handle_cell_message(
        CellToBaseMsg::MinigameResult { ticket },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &Some(registry),
        "127.0.0.1",
        7777,
    )
    .await;
    let event = capture
        .find_event(
            tracing::Level::ERROR,
            "Minigame result lost",
            "cell_channel_closed",
        )
        .expect("dropped receiver must log an ERROR");
    assert!(event.has_field("chain_count", "2"), "{event:#?}");
    assert!(event.has_field("entity_id", "10"), "{event:#?}");
}

#[tokio::test]
async fn minigame_result_without_cell_channel_logs_warn() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr, registry, ticket) = result_fixture(1, vec![100], false).await;
    handle_cell_message(
        CellToBaseMsg::MinigameResult { ticket },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &Some(registry),
        "127.0.0.1",
        7777,
    )
    .await;
    let event = capture
        .find_event(
            tracing::Level::WARN,
            "Minigame result lost",
            "no_cell_channel",
        )
        .expect("missing cell channel must log a WARN");
    assert!(event.has_field("chain_count", "1"), "{event:#?}");
}

#[tokio::test]
async fn forged_ticket_and_original_owner_takeover_emit_no_outputs() {
    let transport = Arc::new(TestTransport::new());
    let (connected, entity_to_addr, registry, ticket) = result_fixture(1, vec![100], true).await;
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    minigame::minigame_result(
        "forged-ticket",
        &Some(registry.clone()),
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx.clone()),
    )
    .await;
    assert!(transport.is_empty());
    assert!(cell_rx.try_recv().is_err());
    let addr = entity_to_addr.lock().unwrap()[&10];
    let mut replacement = crate::test_support::test_default_connected_client_state();
    replacement.player_entity_id = Some(10);
    replacement.active_player_id = Some(1);
    connected.lock().unwrap().insert(addr, replacement);
    minigame::minigame_result(
        &ticket,
        &Some(registry),
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;
    assert!(
        transport.is_empty(),
        "new Base connection must receive no old close"
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "new Base connection must get no Cell credit"
    );
}

#[tokio::test]
async fn result_notification_with_disabled_registry_emits_no_outputs() {
    let capture = LogCapture::install();
    let transport = Arc::new(TestTransport::new());
    let (connected, entity_to_addr, _registry, ticket) = result_fixture(1, vec![100], true).await;
    let (cell_tx, mut cell_rx) = mpsc::channel(1);
    minigame::minigame_result(
        &ticket,
        &None,
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;
    assert!(transport.is_empty());
    assert!(cell_rx.try_recv().is_err());
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "Minigame result notification dropped",
            "result_registry_disabled",
        )
        .unwrap();
    assert!(!format!("{row:#?}").contains(&ticket));
}

#[tokio::test]
async fn result_close_rechecks_original_owner_before_cell_forward() {
    let (connected, entity_to_addr, registry, ticket) = result_fixture(1, vec![100], true).await;
    let addr = entity_to_addr.lock().unwrap()[&10];
    let transport = Arc::new(CloseTakeoverTransport {
        connected: connected.clone(),
        addr,
        sends: AtomicUsize::new(0),
    });
    let (cell_tx, mut cell_rx) = mpsc::channel(1);
    minigame::minigame_result(
        &ticket,
        &Some(registry),
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
    assert!(
        cell_rx.try_recv().is_err(),
        "owner changed during awaited close"
    );
    let state = &connected.lock().unwrap()[&addr];
    assert_eq!(state.next_seq.load(Ordering::SeqCst), 0);
    assert!(state.channel.lock().unwrap().tx_window.is_empty());
}
