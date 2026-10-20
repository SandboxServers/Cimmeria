use super::*;
use std::future::{poll_fn, Future};
use tokio::sync::oneshot;

fn queue_behind_lock<F>(future: F) -> (oneshot::Receiver<()>, tokio::task::JoinHandle<F::Output>)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (queued_tx, queued_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let mut future = Box::pin(future);
        let mut queued_tx = Some(queued_tx);
        poll_fn(move |cx| {
            let result = future.as_mut().poll(cx);
            if result.is_pending() {
                if let Some(tx) = queued_tx.take() {
                    let _ = tx.send(());
                }
            }
            result
        })
        .await
    });
    (queued_rx, task)
}

fn board(entity_id: u32) -> BoardSnapshot {
    BoardSnapshot {
        entity_id,
        game_name: "Hack".into(),
        state: "ready",
        outcome: None,
        difficulty: 1,
        game_started: false,
        timer_state: 0,
        time_remaining: 20.0,
        goal_total: 1,
        goal_cut: 0,
        wires: vec![],
        events: vec![],
        events_dropped: 0,
    }
}

async fn claimed(registry: &SessionRegistry, entity: u32) -> MinigameSession {
    let ticket = registry
        .register(
            entity,
            entity as i32,
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
            9,
        )
        .await
        .unwrap();
    registry
        .authenticate_and_claim(entity, &ticket, "Hack")
        .await
        .unwrap()
}

#[tokio::test]
async fn session_result_one_use_survives_normal_teardown_and_rejects_second_deposit() {
    let registry = SessionRegistry::new();
    let session = claimed(&registry, 1).await;
    assert!(registry.deposit_result(&session, 1, true).await);
    assert!(!registry.deposit_result(&session, 2, true).await);
    assert!(registry.remove_if_ticket(1, &session.ticket).await);
    let receipt = registry.redeem_result(&session.ticket).await.unwrap();
    assert_eq!(
        (
            receipt.entity_id,
            receipt.player_id,
            receipt.session_instance_id
        ),
        (1, 1, 9)
    );
    assert_eq!(
        (
            receipt.result_code,
            receipt.on_victory_chains,
            receipt.notify_client
        ),
        (1, vec![100], true)
    );
    assert!(registry.redeem_result(&session.ticket).await.is_none());
}

#[tokio::test]
async fn session_result_explicit_teardown_revokes_orphan_receipts() {
    for path in 0..3 {
        let registry = SessionRegistry::new();
        let session = claimed(&registry, 1).await;
        assert!(registry.deposit_result(&session, 0, true).await);
        registry.remove_if_ticket(1, &session.ticket).await;
        match path {
            0 => registry.remove(1).await,
            1 => {
                registry.cancel(1).await;
            }
            _ => {
                registry.cancel_if_ticket(1, &session.ticket).await;
            }
        }
        assert!(
            registry.redeem_result(&session.ticket).await.is_none(),
            "path {path}"
        );
    }
}

#[tokio::test]
async fn session_result_stale_cancel_cannot_erase_replacement() {
    let registry = SessionRegistry::new();
    let old = claimed(&registry, 1).await;
    assert!(registry.deposit_result(&old, 1, true).await);
    registry.remove_if_ticket(1, &old.ticket).await;
    let new = claimed(&registry, 1).await;
    registry.publish_board(1, &new.ticket, board(1)).await;
    assert!(registry.board_snapshot(1).await.is_some());
    assert!(registry.redeem_result(&old.ticket).await.is_none());
    assert!(registry.deposit_result(&new, 1, true).await);
    assert!(!registry.cancel_if_ticket(1, &old.ticket).await);
    let retained = registry.board_snapshot(1).await.unwrap();
    assert!(retained
        .events
        .iter()
        .any(|event| event.event == "board_started"));
    assert_eq!(
        registry.redeem_result(&new.ticket).await.unwrap().ticket,
        new.ticket
    );
}

#[tokio::test]
async fn session_result_cancel_and_deposit_obey_both_lock_orders() {
    for cancel_first in [true, false] {
        let registry = SessionRegistry::new();
        let session = claimed(&registry, 1).await;
        registry.publish_board(1, &session.ticket, board(1)).await;
        let guard = registry.hold_registry_lock_for_test().await;
        let reg = registry.clone();
        let claim = session.clone();
        let deposit_op = async move { reg.deposit_result_decision(&claim, 1, true).await };
        let reg = registry.clone();
        let cancel_op = async move { reg.cancel(1).await };
        let (cancel, deposit) = if cancel_first {
            let (cancel_queued, cancel) = queue_behind_lock(cancel_op);
            tokio::time::timeout(Duration::from_secs(1), cancel_queued)
                .await
                .unwrap()
                .unwrap();
            let (deposit_queued, deposit) = queue_behind_lock(deposit_op);
            tokio::time::timeout(Duration::from_secs(1), deposit_queued)
                .await
                .unwrap()
                .unwrap();
            (cancel, deposit)
        } else {
            let (deposit_queued, deposit) = queue_behind_lock(deposit_op);
            tokio::time::timeout(Duration::from_secs(1), deposit_queued)
                .await
                .unwrap()
                .unwrap();
            let (cancel_queued, cancel) = queue_behind_lock(cancel_op);
            tokio::time::timeout(Duration::from_secs(1), cancel_queued)
                .await
                .unwrap()
                .unwrap();
            (cancel, deposit)
        };
        drop(guard);
        assert!(tokio::time::timeout(Duration::from_secs(1), cancel)
            .await
            .unwrap()
            .unwrap());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), deposit)
                .await
                .unwrap()
                .unwrap(),
            if cancel_first {
                DepositOutcome::Refused {
                    player_canceled: true,
                }
            } else {
                DepositOutcome::Deposited
            }
        );
        assert!(registry.redeem_result(&session.ticket).await.is_none());
        assert!(registry.board_snapshot(1).await.is_none());
    }
}

#[tokio::test]
async fn session_result_cancel_and_redeem_obey_both_lock_orders() {
    for cancel_first in [true, false] {
        let registry = SessionRegistry::new();
        let session = claimed(&registry, 1).await;
        registry.publish_board(1, &session.ticket, board(1)).await;
        assert!(registry.deposit_result(&session, 1, true).await);
        let guard = registry.hold_registry_lock_for_test().await;
        let reg = registry.clone();
        let cancel_op = async move { reg.cancel(1).await };
        let reg = registry.clone();
        let ticket = session.ticket.clone();
        let redeem_op = async move { reg.redeem_result(&ticket).await };
        let (cancel, redeem) = if cancel_first {
            let (cancel_queued, cancel) = queue_behind_lock(cancel_op);
            tokio::time::timeout(Duration::from_secs(1), cancel_queued)
                .await
                .unwrap()
                .unwrap();
            let (redeem_queued, redeem) = queue_behind_lock(redeem_op);
            tokio::time::timeout(Duration::from_secs(1), redeem_queued)
                .await
                .unwrap()
                .unwrap();
            (cancel, redeem)
        } else {
            let (redeem_queued, redeem) = queue_behind_lock(redeem_op);
            tokio::time::timeout(Duration::from_secs(1), redeem_queued)
                .await
                .unwrap()
                .unwrap();
            let (cancel_queued, cancel) = queue_behind_lock(cancel_op);
            tokio::time::timeout(Duration::from_secs(1), cancel_queued)
                .await
                .unwrap()
                .unwrap();
            (cancel, redeem)
        };
        drop(guard);
        assert!(tokio::time::timeout(Duration::from_secs(1), cancel)
            .await
            .unwrap()
            .unwrap());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), redeem)
                .await
                .unwrap()
                .unwrap()
                .is_some(),
            !cancel_first
        );
        assert_eq!(registry.receipt_count().await, 0);
        assert!(registry.board_snapshot(1).await.is_none());
    }
}

#[tokio::test(start_paused = true)]
async fn session_result_exact_ttl_and_terminal_marker_outlive_receipt() {
    let capture = crate::test_support::LogCapture::install();
    let registry = SessionRegistry::new();
    let session = claimed(&registry, 1).await;
    assert!(registry.deposit_result(&session, 1, true).await);
    tokio::time::advance(session_result::RESULT_RECEIPT_TTL).await;
    assert!(registry.redeem_result(&session.ticket).await.is_none());
    assert_eq!(registry.receipt_count().await, 0);
    assert!(!registry.deposit_result(&session, 1, true).await);
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "Minigame result receipt expired",
            "receipt_expired",
        )
        .unwrap();
    assert!(row.has_field("entity_id", "1"), "{row:#?}");
    assert!(!format!("{:#?}", capture.all()).contains(&session.ticket));
}

#[tokio::test(start_paused = true)]
async fn session_result_sweep_and_fresh_deposit_expire_old_receipts() {
    let registry = SessionRegistry::new();
    let old = claimed(&registry, 1).await;
    assert!(registry.deposit_result(&old, 1, true).await);
    tokio::time::advance(session_result::RESULT_RECEIPT_TTL).await;
    registry.expire_pending(PENDING_SESSION_TTL).await;
    assert_eq!(registry.receipt_count().await, 0);
    let fresh = claimed(&registry, 2).await;
    assert!(registry.deposit_result(&fresh, 2, true).await);
    assert_eq!(registry.receipt_count().await, 1);
    assert!(registry.redeem_result(&old.ticket).await.is_none());
    assert_eq!(
        registry
            .redeem_result(&fresh.ticket)
            .await
            .unwrap()
            .result_code,
        2
    );

    let old_again = claimed(&registry, 3).await;
    assert!(registry.deposit_result(&old_again, 1, true).await);
    tokio::time::advance(session_result::RESULT_RECEIPT_TTL).await;
    let next = claimed(&registry, 4).await;
    assert!(registry.deposit_result(&next, 2, true).await);
    assert_eq!(registry.receipt_count().await, 1);
    assert!(registry.redeem_result(&old_again.ticket).await.is_none());
}

#[tokio::test]
async fn session_result_refuses_wrong_unclaimed_and_invalid_codes() {
    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            1,
            1,
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
            9,
        )
        .await
        .unwrap();
    let pending = registry.authenticate(1, &ticket, "Hack").await.unwrap();
    assert!(!registry.deposit_result(&pending, 1, true).await);
    let claimed = registry
        .authenticate_and_claim(1, &ticket, "Hack")
        .await
        .unwrap();
    let mut forged = claimed.clone();
    forged.ticket = "forged".into();
    assert!(!registry.deposit_result(&forged, 1, true).await);
    assert!(!registry.deposit_result(&claimed, 3, true).await);
    assert!(registry.deposit_result(&claimed, 0, true).await);
    assert!(registry
        .redeem_result(&ticket)
        .await
        .unwrap()
        .on_victory_chains
        .is_empty());
}

#[tokio::test]
async fn session_result_capacity_evicts_oldest_without_reissuing() {
    let capture = crate::test_support::LogCapture::install();
    assert_eq!(session_result::RESULT_RECEIPT_CAP, 4096);
    let registry = SessionRegistry::new();
    let first = claimed(&registry, 1).await;
    assert!(registry.deposit_result(&first, 1, true).await);
    for entity in 2..=(session_result::RESULT_RECEIPT_CAP as u32 + 1) {
        let session = claimed(&registry, entity).await;
        assert!(registry.deposit_result(&session, 2, true).await);
    }
    assert_eq!(
        registry.receipt_count().await,
        session_result::RESULT_RECEIPT_CAP
    );
    assert!(registry.redeem_result(&first.ticket).await.is_none());
    assert!(!registry.deposit_result(&first, 1, true).await);
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "Minigame result receipt evicted",
            "receipt_capacity",
        )
        .unwrap();
    assert!(row.has_field("entity_id", "1"), "{row:#?}");
    assert!(!format!("{:#?}", capture.all()).contains(&first.ticket));
}
