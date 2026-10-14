use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_base_session::base::helpers::derive_letter_turn_in_token;
use cimmeria_base_session::base::plugin::{BaseCtx, PluginMsg};
use cimmeria_base_session::base::ConnectedClientState;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{
    BaseToCellMsg, LetterTurnInAttempt, LetterTurnInEntryEpoch, LetterTurnInOutcome,
    LetterTurnInRequest,
};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use super::letter_turn_in_route::{
    accepted_entry, lowercase_hex, route_letter_turn_in, LetterTurnInPlugin,
};
use super::letter_turn_in_tests::{
    accept_and_advance, cleanup, mission_snapshot, player, put_item, receipt_count,
};
use crate::base::world_entry::methods::inventory::{
    handle_move_inventory_item, sync_committed_letter,
};
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const ENTITY: u32 = 0x7360;

struct Rig {
    addr: SocketAddr,
    request: LetterTurnInRequest,
    pool: Option<Arc<PgPool>>,
    tx: Option<mpsc::Sender<BaseToCellMsg>>,
    rx: mpsc::Receiver<BaseToCellMsg>,
    transport: Arc<dyn Transport>,
    recorder: Arc<TestTransport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    mapping: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Rig {
    fn new(player_id: i32, pool: Option<Arc<PgPool>>) -> Self {
        let addr = "127.0.0.1:57360".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.key = [9; 32];
        state.enc = MercuryEncryption::from_session_key(state.key);
        state.player_entity_id = Some(ENTITY);
        state.active_player_id = Some(player_id);
        state.world_name = Some("Harset_CmdCenter".to_owned());
        state.extensions.insert(LetterTurnInEntryEpoch::new(1));
        let request = LetterTurnInRequest {
            entity_id: ENTITY,
            player_id,
            attempt: LetterTurnInAttempt::from_bytes([0xab; 16]),
            token: derive_letter_turn_in_token(&state.key),
            generation: state.connected_at,
            epoch: LetterTurnInEntryEpoch::new(1),
        };
        let (tx, rx) = mpsc::channel(8);
        let recorder = Arc::new(TestTransport::default());
        Self {
            addr,
            request,
            pool,
            tx: Some(tx),
            rx,
            transport: recorder.clone(),
            recorder,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            mapping: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
        }
    }

    fn ctx(&self) -> BaseCtx<'_> {
        BaseCtx {
            db_pool: &self.pool,
            cell_tx: &self.tx,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.mapping,
        }
    }

    fn expected(&self) -> cimmeria_base_session::base::helpers::ExpectedLetterTurnInEntry {
        let clients = self.connected.lock().unwrap();
        cimmeria_base_session::base::helpers::ExpectedLetterTurnInEntry::capture(
            self.addr,
            &clients[&self.addr],
            &self.request,
        )
        .unwrap()
    }

    async fn dispatch(&self, request: LetterTurnInRequest) {
        route_letter_turn_in(PluginMsg::new(request), self.ctx()).await;
    }

    async fn result(&mut self) -> LetterTurnInOutcome {
        let msg = tokio::time::timeout(Duration::from_secs(8), self.rx.recv())
            .await
            .expect("turn-in result timeout")
            .expect("turn-in result channel closed");
        let BaseToCellMsg::LetterTurnInResult(result) = msg else {
            panic!("unexpected base-to-cell message");
        };
        assert_eq!(result.request.entity_id, self.request.entity_id);
        assert_eq!(result.request.player_id, self.request.player_id);
        assert_eq!(result.request.attempt, self.request.attempt);
        result.outcome
    }
}

#[test]
fn plugin_envelope_and_hex_boundary_are_exact() {
    use cimmeria_base_session::base::plugin::BasePlugins;
    use cimmeria_wire::cell::messages::CellToBaseMsg;
    let plugins = BasePlugins::build(&[&LetterTurnInPlugin]).unwrap();
    assert!(plugins
        .cell_message_types()
        .any(|name| name.ends_with("LetterTurnInRequest")));
    let rig = Rig::new(42, None);
    let msg = CellToBaseMsg::Plugin(PluginMsg::new(rig.request));
    assert_eq!(msg.kind(), "plugin");
    assert!(!format!("{:?}", rig.request).contains("abababababababababababababababab"));
    assert_eq!(
        lowercase_hex([0xab; 16]),
        "abababababababababababababababab"
    );
}

#[tokio::test]
async fn missing_pool_reports_retryable_without_client_effect() {
    let mut rig = Rig::new(42, None);
    rig.dispatch(rig.request).await;
    assert!(matches!(
        rig.result().await,
        LetterTurnInOutcome::UnknownRetryable
    ));
    assert!(rig.recorder.filter_to(rig.addr).is_empty());
}

struct AsyncOnlyTransport;

impl Transport for AsyncOnlyTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { panic!("fenced item sync must not await an async transport") })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

#[tokio::test]
async fn live_db_unsupported_sync_transport_returns_retryable_without_committed_result() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 74).await;
    accept_and_advance(&pool, id).await;
    put_item(&pool, id, 3730, 2, 0, 1).await;
    let mut rig = Rig::new(id, Some(Arc::new(pool.clone())));
    rig.transport = Arc::new(AsyncOnlyTransport);
    rig.dispatch(rig.request).await;
    assert!(matches!(
        rig.result().await,
        LetterTurnInOutcome::UnknownRetryable
    ));
    assert_eq!(
        receipt_count(&pool, id).await,
        1,
        "durable commit remains, so retry reconciles receipt"
    );
    assert!(rig.recorder.is_empty());
    assert!(rig.connected.lock().unwrap()[&rig.addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
    cleanup(&pool, id).await;
}

struct HeldAdmissionTransport {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
    recorder: Arc<TestTransport>,
}

impl Transport for HeldAdmissionTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { panic!("fenced item sync must not await an async transport") })
    }

    fn try_send_to(&self, bytes: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        if let Some(entered) = self.entered.lock().unwrap().take() {
            let _ = entered.send(());
        }
        self.release.lock().unwrap().recv().unwrap();
        self.recorder.try_send_to(bytes, addr)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_db_partial_sync_row_lock_orders_actual_move_item_after_admission() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 75).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 2, 0, 2).await;
    let rig = Rig::new(id, Some(Arc::new(pool.clone())));
    let (entered_tx, entered_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let held: Arc<dyn Transport> = Arc::new(HeldAdmissionTransport {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
        recorder: rig.recorder.clone(),
    });
    let sync_pool = pool.clone();
    let expected = rig.expected();
    let connected = rig.connected.clone();
    let mapping = rig.mapping.clone();
    let sync_task = tokio::spawn(async move {
        let db_pool = Some(Arc::new(sync_pool.clone()));
        let cell_tx = None;
        sync_committed_letter(
            &sync_pool,
            &expected,
            item_id,
            1,
            BaseCtx {
                db_pool: &db_pool,
                cell_tx: &cell_tx,
                transport: &held,
                connected: &connected,
                entity_to_addr: &mapping,
            },
        )
        .await
        .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(5), entered_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(
        rig.recorder.is_empty(),
        "admission is paused after the locked row read"
    );

    let move_rig = Rig::new(id, Some(Arc::new(pool.clone())));
    let move_pool = move_rig.pool.clone();
    let move_tx = move_rig.tx.clone();
    let move_transport = move_rig.transport.clone();
    let move_connected = move_rig.connected.clone();
    let move_mapping = move_rig.mapping.clone();
    let move_task = tokio::spawn(async move {
        handle_move_inventory_item(
            ENTITY,
            id,
            item_id,
            2,
            1,
            -1,
            &move_pool,
            &move_tx,
            &move_transport,
            &move_connected,
            &move_mapping,
        )
        .await;
    });
    let mut observed_wait = false;
    for _ in 0..250 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity WHERE datname = current_database() \
             AND wait_event_type = 'Lock' AND query LIKE '%sgw_inventory%' AND query LIKE '%FOR UPDATE%'"
        ).fetch_one(&pool).await.unwrap();
        if waiting > 0 {
            observed_wait = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        observed_wait,
        "actual moveItem must reach and wait on the inventory row lock"
    );
    let before: (i32, i32) =
        sqlx::query_as("SELECT container_id, slot_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        before,
        (2, 0),
        "move cannot commit while letter update is paused"
    );
    release_tx.send(()).unwrap();
    assert_eq!(
        sync_task.await.unwrap(),
        cimmeria_base_session::base::helpers::LetterTurnInSendOutcome::Sent
    );
    move_task.await.unwrap();
    let after: (i32, i32) =
        sqlx::query_as("SELECT container_id, slot_id FROM sgw_inventory WHERE item_id = $1")
            .bind(item_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after, (2, 1));
    assert_eq!(
        rig.recorder.len(),
        1,
        "one old-slot update admitted before move commit"
    );
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_route_commits_and_retries_with_targeted_removal() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 70).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 2, 0, 1).await;
    let mut rig = Rig::new(id, Some(Arc::new(pool.clone())));
    rig.dispatch(rig.request).await;
    let first = rig.result().await;
    let LetterTurnInOutcome::Committed(first) = first else {
        panic!("expected committed, got {first:?}");
    };
    assert_eq!(
        (first.item_id, first.stack_before, first.stack_after),
        (item_id, 1, 0)
    );
    assert_eq!(first.mission.status, 2);
    assert_eq!(receipt_count(&pool, id).await, 1);
    assert_eq!(mission_snapshot(&pool, id).await.0, 2);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let sent = rig.recorder.filter_to(rig.addr);
    assert_eq!(sent.len(), 1, "one targeted onRemoveItem precedes result");
    let mut args = Vec::from(1u32.to_le_bytes());
    args.extend_from_slice(&item_id.to_le_bytes());
    let expected = cimmeria_wire::mercury::build_player_entity_method_packet(
        &[9; 32],
        0,
        &[],
        ENTITY,
        71,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(sent[0], expected);

    rig.dispatch(rig.request).await;
    assert!(matches!(
        rig.result().await,
        LetterTurnInOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(
        rig.recorder.filter_to(rig.addr).len(),
        2,
        "retry repeats targeted sync"
    );
    assert_eq!(receipt_count(&pool, id).await, 1);
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_route_partial_stack_sends_one_update_row() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 72).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 2, 0, 2).await;
    let mut rig = Rig::new(id, Some(Arc::new(pool.clone())));
    rig.dispatch(rig.request).await;
    let LetterTurnInOutcome::Committed(commit) = rig.result().await else {
        panic!("partial stack did not commit");
    };
    assert_eq!(
        (commit.item_id, commit.stack_before, commit.stack_after),
        (item_id, 2, 1)
    );
    let sent = rig.recorder.filter_to(rig.addr);
    assert_eq!(sent.len(), 1);
    let packet = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([9; 32])
        .decrypt(&sent[0])
        .unwrap();
    let decoded = cimmeria_mercury::packet::parse_incoming(&packet).unwrap();
    assert_eq!(decoded.body[0], 0xbd, "extended entity method marker");
    assert_eq!(decoded.body[7], 72 - 61, "method 72 subslot");
    assert_eq!(&decoded.body[8..12], &1u32.to_le_bytes(), "one row");
    let stack: i32 = sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stack, 1);
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_route_refuses_stale_identity_before_mutation() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 71).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 2, 0, 1).await;
    for case in 0..9 {
        let rig = Rig::new(id, Some(Arc::new(pool.clone())));
        let mut request = rig.request;
        match case {
            0 => request.player_id += 1,
            1 => request.token = derive_letter_turn_in_token(&[8; 32]),
            2 => request.generation += Duration::from_nanos(1),
            3 => request.epoch = LetterTurnInEntryEpoch::new(2),
            4 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .cancelled
                    .store(true, std::sync::atomic::Ordering::Release);
            }
            5 => {
                rig.mapping.lock().unwrap().remove(&ENTITY);
            }
            6 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .world_name = Some("Castle".to_owned());
            }
            7 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .extensions
                    .insert(LetterTurnInEntryEpoch::new(2));
            }
            _ => {
                // A relog may reuse the address, entity and player IDs, but
                // must have a new entry incarnation (and session key).
                let mut replacement = test_default_connected_client_state();
                replacement.key = [8; 32];
                replacement.enc = MercuryEncryption::from_session_key(replacement.key);
                replacement.player_entity_id = Some(ENTITY);
                replacement.active_player_id = Some(id);
                replacement.world_name = Some("Harset_CmdCenter".to_owned());
                replacement
                    .extensions
                    .insert(LetterTurnInEntryEpoch::new(2));
                rig.connected.lock().unwrap().insert(rig.addr, replacement);
            }
        }
        assert!(
            accepted_entry(&request, rig.ctx()).is_none(),
            "case {case} must never spawn a DB task"
        );
        rig.dispatch(request).await;
        assert!(rig.recorder.filter_to(rig.addr).is_empty(), "case {case}");
        assert_eq!(receipt_count(&pool, id).await, 0, "case {case}");
        assert_eq!(mission_snapshot(&pool, id).await.0, 1, "case {case}");
        let stack: i32 =
            sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
                .bind(item_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stack, 1, "case {case}");
    }
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_route_does_not_block_base_dispatch_on_db_lock() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 73).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 2, 0, 1).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT item_id FROM sgw_inventory WHERE item_id = $1 FOR UPDATE")
        .bind(item_id)
        .fetch_one(&mut *held)
        .await
        .unwrap();

    let mut rig = Rig::new(id, Some(Arc::new(pool.clone())));
    tokio::time::timeout(Duration::from_secs(1), rig.dispatch(rig.request))
        .await
        .expect("plugin dispatch must return before A1 waits for the item lock");
    assert!(
        rig.rx.try_recv().is_err(),
        "DB transaction is still blocked"
    );
    held.rollback().await.unwrap();
    assert!(matches!(
        rig.result().await,
        LetterTurnInOutcome::Committed(_)
    ));
    assert_eq!(receipt_count(&pool, id).await, 1);
    cleanup(&pool, id).await;
}
