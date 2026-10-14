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

use super::letter_turn_in_route::{lowercase_hex, route_letter_turn_in, LetterTurnInPlugin};
use super::letter_turn_in_tests::{
    accept_and_advance, cleanup, mission_snapshot, player, put_item, receipt_count,
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
                // A relog may reuse the same address, entity and player IDs.
                // Even a copied generation/key cannot reuse the old Arc.
                let mut replacement = test_default_connected_client_state();
                replacement.key = [9; 32];
                replacement.connected_at = request.generation;
                replacement.player_entity_id = Some(ENTITY);
                replacement.active_player_id = Some(id);
                replacement.world_name = Some("Harset_CmdCenter".to_owned());
                replacement
                    .extensions
                    .insert(LetterTurnInEntryEpoch::new(1));
                rig.connected.lock().unwrap().insert(rig.addr, replacement);
            }
        }
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
