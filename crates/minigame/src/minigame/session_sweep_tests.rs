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
        Ok(CellToBaseMsg::MinigameResult {
            entity_id,
            ticket: reported,
            result_code,
            on_victory_chains,
            on_failure_chains,
            ..
        }) => {
            assert_eq!(entity_id, 42);
            assert_eq!(reported, ticket);
            assert_eq!(result_code, 3);
            assert!(on_victory_chains.is_empty() && on_failure_chains.is_empty());
        }
        other => panic!("expected the expiry report, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "one report per expired session");
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
