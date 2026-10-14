use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Barrier;

use super::letter_turn_in::{commit_letter_turn_in, LetterTurnInVerdict};
use super::letter_turn_in_tests::{
    accept_and_advance, cleanup, mission_snapshot, player, put_item, receipt_count, SESSION,
};
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn live_db_letter_turn_in_serializes_races() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 40).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 1, 0, 2).await;

    let barrier = Arc::new(Barrier::new(3));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let gate = barrier.clone();
        let db = pool.clone();
        tasks.push(tokio::spawn(async move {
            gate.wait().await;
            commit_letter_turn_in(&db, id, "racing-attempt", SESSION).await
        }));
    }
    barrier.wait().await;
    let mut committed = 0;
    let mut retried = 0;
    for task in tasks {
        match tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .expect("turn-in race timed out")
            .expect("turn-in task panicked")
            .expect("turn-in SQL error")
        {
            LetterTurnInVerdict::Committed(_) => committed += 1,
            LetterTurnInVerdict::AlreadyCommitted(_) => retried += 1,
            other => panic!("unexpected race outcome: {other:?}"),
        }
    }
    assert_eq!((committed, retried), (1, 1));
    assert_eq!(receipt_count(&pool, id).await, 1);
    assert_eq!(mission_snapshot(&pool, id).await.0, 2);
    let stack: i32 = sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stack, 1);
    cleanup(&pool, id).await;

    // A character deletion can hold the player row before turn-in reaches
    // FOR KEY SHARE. Both must finish, with the deletion winning this order.
    let id = player(&pool, 41).await;
    accept_and_advance(&pool, id).await;
    put_item(&pool, id, 3730, 1, 0, 1).await;
    let mut delete_tx = pool.begin().await.expect("start deletion transaction");
    sqlx::query("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(id)
        .execute(&mut *delete_tx)
        .await
        .expect("lock player for deletion");
    let db = pool.clone();
    let turn_in =
        tokio::spawn(async move { commit_letter_turn_in(&db, id, "delete-race", SESSION).await });
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(id)
        .execute(&mut *delete_tx)
        .await
        .expect("delete player");
    tokio::time::timeout(Duration::from_secs(10), delete_tx.commit())
        .await
        .expect("deletion timed out")
        .expect("deletion failed");
    let outcome = tokio::time::timeout(Duration::from_secs(10), turn_in)
        .await
        .expect("turn-in during deletion timed out")
        .expect("turn-in task panicked")
        .expect("turn-in SQL error");
    assert!(matches!(
        outcome,
        LetterTurnInVerdict::Refused(super::letter_turn_in::LetterTurnInRefusal::NoPlayer)
    ));
    assert_eq!(receipt_count(&pool, id).await, 0);
    cleanup(&pool, id).await;
}
