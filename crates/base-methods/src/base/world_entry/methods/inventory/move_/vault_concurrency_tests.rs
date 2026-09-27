//! BV-03 concurrency guards: two moves of one stack into the vault, racing
//! on a multi-thread runtime behind a barrier, can never duplicate it.
//!
//! The per-player move lock and the source row's `FOR UPDATE` serialize
//! the two, and each write checks the rows it touched, so the second move
//! finds its source gone (or smaller) and changes nothing. Without them,
//! both merges would read the same source stack and each add it to a
//! different vault stack.
//!
//! Sentinels: players `0x7000_B5C0..=0x7000_B5D1` (beside the
//! `vault_move_tests` range), entities `0x7000_B5EC`/`0x7000_B5ED`.

use std::time::Duration;

use tokio::sync::Barrier;

use super::tests::insert_item;
use super::vault_move_tests::{in_world, rows, setup, teardown, AT_BANKER, BANKABLE};
use super::*;
use crate::test_support::require_db_or_skip;

/// Run `a` and `b` together behind a barrier, each capped at 5 s so a
/// deadlock fails the test instead of wedging the suite.
async fn race(
    pool: &PgPool,
    entity_id: u32,
    player_id: i32,
    a: (i32, i32, i32, i32),
    b: (i32, i32, i32, i32),
) {
    let client = Arc::new(in_world(entity_id, 40845));
    let barrier = Arc::new(Barrier::new(2));
    let spawn = |(item, container, slot, quantity): (i32, i32, i32, i32)| {
        let pool = pool.clone();
        let client = client.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_move_inventory_item_with_vault(
                entity_id,
                player_id,
                item,
                container,
                slot,
                quantity,
                AT_BANKER,
                &Some(Arc::new(pool)),
                &None,
                &client.dyn_transport,
                &client.conn,
                &client.e2a,
            )
            .await;
        })
    };
    let (first, second) = (spawn(a), spawn(b));
    tokio::time::timeout(Duration::from_secs(5), async {
        first.await.expect("first move panicked");
        second.await.expect("second move panicked");
    })
    .await
    .expect("the two moves deadlocked or hung past 5 s");
}

/// One carried stack of 5 merged, at the same moment, onto two different
/// same-type vault stacks of 1: exactly one merge happens. The total stays
/// 7 and the source row is gone. A duplicate would show as 12.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_concurrent_merges_of_one_stack_cannot_duplicate_it() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE_C, BASE_C + 1, 0x7000_B5EC);
    setup(&pool, account_id, player_id).await;
    let source = insert_item(&pool, player_id, BANKABLE, 1, 0, 5).await;
    let a = insert_item(&pool, player_id, BANKABLE, 17, 0, 1).await;
    let b = insert_item(&pool, player_id, BANKABLE, 17, 1, 1).await;

    race(
        &pool,
        entity_id,
        player_id,
        (source, 17, 0, -1),
        (source, 17, 1, -1),
    )
    .await;

    let after = rows(&pool, player_id).await;
    assert_eq!(
        after.iter().map(|r| r.3).sum::<i32>(),
        7,
        "the stack must be counted once: {after:?}"
    );
    assert!(
        after.iter().all(|r| r.0 != source),
        "the merged source row is gone: {after:?}"
    );
    let stacks: Vec<i32> = [a, b]
        .iter()
        .map(|id| after.iter().find(|r| r.0 == *id).expect("vault row").3)
        .collect();
    assert!(
        stacks == vec![6, 1] || stacks == vec![1, 6],
        "exactly one merge: {stacks:?}"
    );

    teardown(&pool, account_id, player_id).await;
}

/// One carried stack of 10 split 6 into two empty vault slots at once: the
/// second split finds only 4 left and is refused. The total stays 10.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_concurrent_splits_of_one_stack_cannot_duplicate_it() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE_C + 0x10, BASE_C + 0x11, 0x7000_B5ED);
    setup(&pool, account_id, player_id).await;
    let source = insert_item(&pool, player_id, BANKABLE, 1, 0, 10).await;

    race(
        &pool,
        entity_id,
        player_id,
        (source, 17, 0, 6),
        (source, 17, 1, 6),
    )
    .await;

    let after = rows(&pool, player_id).await;
    assert_eq!(after.len(), 2, "one split only: {after:?}");
    assert_eq!(after[0], (source, 1, 0, 4));
    assert_eq!(after.iter().map(|r| r.3).sum::<i32>(), 10, "{after:?}");

    teardown(&pool, account_id, player_id).await;
}

const BASE_C: i32 = 0x7000_B5C0;
