//! MG-06: the TTL sweep's report to the cell (split out of session_tests.rs).

use super::tests::{let_spawned_task_start, register_livewire};
use super::*;
use std::time::Duration;

/// MG-06: a swept session reports result 3 with its own ticket, so the cell
/// drops the minigame hold. Fails with the report removed from the sweep: the
/// channel stays empty and the player stays locked.
#[tokio::test(start_paused = true)]
async fn spawn_sweep_reporting_sends_result_3_for_an_expired_session() {
    use cimmeria_wire::cell::messages::CellToBaseMsg;

    let ttl = Duration::from_secs(30);
    let interval = Duration::from_secs(5);
    let reg = SessionRegistry::new();
    let ticket = register_livewire(&reg, 42).await.unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    reg.spawn_sweep_reporting(ttl, interval, tx);
    let_spawned_task_start().await;

    tokio::time::advance(ttl + interval * 2).await;
    let_spawned_task_start().await;

    match rx.try_recv() {
        Ok(CellToBaseMsg::MinigameResult { ticket: reported }) => {
            assert_eq!(reported, ticket);
        }
        other => panic!("expected the expiry report, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "one report per expired session");
    // The report is ticket-only; the Base redeems the receipt the sweep minted.
    let receipt = reg
        .redeem_result(&ticket)
        .await
        .expect("the sweep must mint a receipt for the expired session");
    assert_eq!(receipt.entity_id, 42);
    assert_eq!(receipt.result_code, 3);
    assert!(receipt.notify_client);
    assert!(receipt.on_victory_chains.is_empty() && receipt.on_failure_chains.is_empty());
    assert!(
        reg.redeem_result(&ticket).await.is_none(),
        "the expiry receipt is one-use"
    );
}

/// A connected session is never reported: its connection task owns the end.
#[tokio::test(start_paused = true)]
async fn spawn_sweep_reporting_ignores_a_connected_session() {
    let ttl = Duration::from_secs(30);
    let interval = Duration::from_secs(5);
    let reg = SessionRegistry::new();
    let ticket = register_livewire(&reg, 42).await.unwrap();
    reg.authenticate_and_claim(42, &ticket, "Livewire")
        .await
        .expect("claim must succeed");
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    reg.spawn_sweep_reporting(ttl, interval, tx);
    let_spawned_task_start().await;

    tokio::time::advance(ttl + interval * 2).await;
    let_spawned_task_start().await;

    assert!(rx.try_recv().is_err());
}
