//! Live-DB integration tests (skip when DATABASE_URL is unset).
//!
//! These exercise the SQL paths (`enqueue_in_tx`, `mark_delivered`,
//! `record_failure`, `drain_undelivered`) end-to-end against a real
//! Postgres. Setup + rationale: docs/architecture/integration-test-infra.md.
//!
//! Each test scopes its writes by a sentinel `entity_id` and cleans up
//! at the end so tests don't poison each other.

use tokio::sync::mpsc;

use super::super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for entity_ids used by live-DB outbox tests. Each test
/// picks a unique offset so concurrent runs don't collide on the same
/// `cell_event_outbox` rows.
///
/// Stays well below `i32::MAX` because `cell_event_outbox.entity_id`
/// is `INTEGER` and the `enqueue_*` helpers cast `as i32`. A `u32`
/// value above `i32::MAX` would wrap to negative and silently land
/// in row space that's hard to recognize as "test fixture" during
/// debugging.
const TEST_ENTITY_BASE: u32 = 0x7000_0000;

async fn cleanup(pool: &sqlx::PgPool, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id as i32)
        .execute(pool)
        .await;
}

/// Clear ALL undelivered rows ahead of a failure-injection test. The
/// drainer scans the first 64 undelivered rows globally (`ORDER BY id
/// LIMIT 64`) and breaks on the first send error, so any older
/// undelivered backlog (e.g. left behind by a prior test that panicked)
/// would be processed BEFORE our sentinel row and prevent us from
/// reaching it. Tests run under `--test-threads=1` so a global wipe is
/// safe — no other live-DB test is in-flight.
async fn purge_undelivered_backlog(pool: &sqlx::PgPool) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE delivered_at IS NULL")
        .execute(pool)
        .await;
}

#[tokio::test]
async fn enqueue_in_tx_writes_row_atomic_with_caller_commit() {
    // Pin the atomicity contract: a row enqueued inside a tx is INVISIBLE
    // to other connections until the caller commits. Without the row
    // becoming visible only on commit, a concurrent drainer pass could
    // see (and dispatch) a row whose accompanying inventory mutation
    // ultimately rolled back.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 1;
    cleanup(&pool, entity_id).await;

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 19,
        target_id: 0,
    };

    // Open a tx, enqueue, but DON'T commit. A separate connection must
    // not see the row.
    let mut tx = pool.begin().await.unwrap();
    let id = enqueue_in_tx(&mut tx, entity_id, &payload).await.unwrap();
    assert!(id > 0, "RETURNING id should produce a real BIGSERIAL value");

    // Separate-connection visibility check — must be invisible pre-commit.
    let visible_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id as i32)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        visible_count, 0,
        "row enqueued in uncommitted tx must not be visible to other connections"
    );

    // Roll back. Row should never appear.
    tx.rollback().await.unwrap();
    let post_rollback_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id as i32)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        post_rollback_count, 0,
        "row enqueued in rolled-back tx must not persist"
    );
}

#[tokio::test]
async fn enqueue_then_drain_round_trips_message_and_marks_delivered() {
    // Full round-trip: enqueue (committing), drain, observe the
    // dispatched message, then verify mark_delivered cleared the row
    // from the undelivered set.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 2;
    cleanup(&pool, entity_id).await;

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 42,
        target_id: 7,
    };
    let id = enqueue(&pool, entity_id, &payload).await.unwrap();
    assert!(id > 0);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(8);
    let stats = drain_undelivered(&pool, &tx).await.unwrap();
    assert_eq!(stats.delivered, 1, "the row we just enqueued should drain");
    assert_eq!(stats.skipped_bad, 0);
    assert_eq!(stats.send_failed, 0);

    // The dispatched message reflects the row contents.
    let msg = rx
        .try_recv()
        .expect("drainer should have sent the BaseToCellMsg");
    match msg {
        BaseToCellMsg::ItemUsed {
            entity_id: e,
            instance_id: _,
            type_id,
            target_id,
        } => {
            assert_eq!(e, entity_id);
            assert_eq!(type_id, 42);
            assert_eq!(target_id, 7);
        }
        _ => panic!("unexpected message variant"),
    }

    // mark_delivered (called by the drainer's success path) clears the
    // row from the undelivered partial-index.
    let undelivered: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cell_event_outbox \
         WHERE entity_id = $1 AND delivered_at IS NULL",
    )
    .bind(entity_id as i32)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        undelivered, 0,
        "drainer's mark_delivered should have cleared the row"
    );

    cleanup(&pool, entity_id).await;
}

#[tokio::test]
async fn drain_stops_at_first_send_failure_and_records_attempt() {
    // The drainer breaks the batch on a closed cell channel — subsequent
    // rows for this peer should NOT be dispatched, and the failed row
    // should have its attempts column bumped + last_error populated for
    // operator triage.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 3;
    cleanup(&pool, entity_id).await;

    let payload_a = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 1,
        target_id: 0,
    };
    let payload_b = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 2,
        target_id: 0,
    };
    enqueue(&pool, entity_id, &payload_a).await.unwrap();
    enqueue(&pool, entity_id, &payload_b).await.unwrap();

    // Drop the receiver immediately so the first send fails.
    let (tx, rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(8);
    drop(rx);
    let stats = drain_undelivered(&pool, &tx).await.unwrap();

    assert_eq!(stats.delivered, 0);
    assert_eq!(
        stats.send_failed, 1,
        "first row's send must fail (and be the only attempt this pass)"
    );

    // Both rows still undelivered (drainer broke the batch on the first
    // failure), and the first row's attempts column is now > 0.
    let still_undelivered: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cell_event_outbox \
         WHERE entity_id = $1 AND delivered_at IS NULL",
    )
    .bind(entity_id as i32)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        still_undelivered, 2,
        "both rows must remain undelivered after the batch break"
    );

    let failed_attempts: i32 =
        sqlx::query_scalar("SELECT MAX(attempts) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id as i32)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        failed_attempts >= 1,
        "record_failure should have bumped the attempts counter on the failed row"
    );

    cleanup(&pool, entity_id).await;
}

/// Drain mpsc messages addressed to `entity_id` and return matching ItemUsed
/// payloads. The outbox is a shared table — concurrent live-DB tests can leave
/// undelivered rows that drain alongside ours, so global stats counts are not
/// safe to assert; per-entity filtering is.
fn drain_item_used_for(rx: &mut mpsc::Receiver<BaseToCellMsg>, entity_id: u32) -> Vec<(i32, i32)> {
    let mut hits = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::ItemUsed {
            entity_id: e,
            instance_id: _,
            type_id,
            target_id,
        } = msg
        {
            if e == entity_id {
                hits.push((type_id, target_id));
            }
        }
    }
    hits
}

#[tokio::test]
async fn injected_send_failure_replays_on_next_drain_with_payload_intact() {
    // Durability contract: when a cell→base dispatch fails (receiver gone /
    // task panic / shutdown race), the row stays in the outbox and is
    // replayed verbatim on the next drain pass. The closed-channel test
    // above covers the "row stays" half; this covers the "row replays
    // intact" half — the operator sees the message once on the *retry*
    // with the same payload bytes.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 4;
    cleanup(&pool, entity_id).await;
    // Wipe any leftover backlog from a prior crashed test — the drainer
    // limits to 64 rows globally and breaks on first send error, so
    // backlog ahead of our row would prevent us from being reached.
    purge_undelivered_backlog(&pool).await;

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 4242,
        target_id: 1337,
    };
    let row_id = enqueue(&pool, entity_id, &payload).await.unwrap();

    // Pass 1: dropped receiver. The drainer reaches our row and fails to
    // send. We can't assert global `send_failed == 1` because the shared
    // outbox may carry undelivered rows from concurrent tests; instead
    // assert ROW state: still undelivered + attempts column bumped.
    let (closed_tx, closed_rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(8);
    drop(closed_rx);
    drain_undelivered(&pool, &closed_tx).await.unwrap();

    let (still_undelivered, attempts): (bool, i32) = sqlx::query_as(
        "SELECT delivered_at IS NULL, attempts FROM cell_event_outbox WHERE id = $1",
    )
    .bind(row_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        still_undelivered,
        "row must remain undelivered after pass-1 send failure"
    );
    // `cleanup` zeros our row's attempts at test start; one failed pass
    // calls record_failure exactly once. Equality (not >=) catches a
    // regression that double-bumps on a single failed pass.
    assert_eq!(
        attempts, 1,
        "exactly one failed pass must have recorded exactly one attempt (got {attempts})"
    );

    // Pass 2: fresh channel. Our row drains and dispatches the original
    // payload bytes verbatim; mark_delivered fires on success.
    let (live_tx, mut live_rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(64);
    drain_undelivered(&pool, &live_tx).await.unwrap();

    let hits = drain_item_used_for(&mut live_rx, entity_id);
    assert_eq!(
        hits,
        vec![(4242, 1337)],
        "exactly one ItemUsed with the original payload bytes must arrive on the retry"
    );

    // Row is now marked delivered.
    let final_delivered: bool =
        sqlx::query_scalar("SELECT delivered_at IS NOT NULL FROM cell_event_outbox WHERE id = $1")
            .bind(row_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        final_delivered,
        "successful pass-2 dispatch must set delivered_at"
    );

    // Pass 3 with a fresh channel: our row must NOT redeliver. Drainer
    // may still emit other tests' rows; filter again by entity_id.
    let (third_tx, mut third_rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(64);
    drain_undelivered(&pool, &third_tx).await.unwrap();
    assert!(
        drain_item_used_for(&mut third_rx, entity_id).is_empty(),
        "delivered row must not redeliver"
    );

    cleanup(&pool, entity_id).await;
}

#[tokio::test]
async fn try_dispatch_now_failure_leaves_row_for_drainer_replay() {
    // The hot-path companion to the drainer-replay case above: when the
    // post-enqueue `try_dispatch_now` lands on a closed channel (cell
    // task gone between enqueue and send), the durability guarantee
    // says the row stays put and the periodic drainer picks it up
    // later. Without this, a single channel-closed window between
    // enqueue and immediate-dispatch would silently drop the event
    // even though the row is in the database.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 5;
    cleanup(&pool, entity_id).await;
    purge_undelivered_backlog(&pool).await;

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 99,
        target_id: 11,
    };
    let row_id = enqueue(&pool, entity_id, &payload).await.unwrap();

    // Inject the failure: try_dispatch_now sees a closed receiver.
    let (closed_tx, closed_rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(8);
    drop(closed_rx);
    try_dispatch_now(&pool, &closed_tx, row_id, entity_id, payload.clone()).await;

    // Row must still be undelivered (try_dispatch_now's failure path is
    // log-and-leave, not log-and-mark-delivered).
    let undelivered_after_try: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cell_event_outbox \
         WHERE id = $1 AND delivered_at IS NULL",
    )
    .bind(row_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        undelivered_after_try, 1,
        "failed try_dispatch_now must leave the row for the drainer"
    );

    // Drainer with a working channel finishes the job. Filter the
    // dispatched messages to our sentinel — concurrent tests' rows in
    // the same shared outbox may also drain on this pass.
    let (live_tx, mut live_rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(64);
    drain_undelivered(&pool, &live_tx).await.unwrap();
    let hits = drain_item_used_for(&mut live_rx, entity_id);
    assert_eq!(
        hits,
        vec![(99, 11)],
        "drainer must replay the row try_dispatch_now skipped, with the original payload"
    );

    cleanup(&pool, entity_id).await;
}

#[tokio::test]
async fn poison_row_does_not_block_following_rows_in_same_batch() {
    // Adversarial mix: a poison row (event_type/payload mismatch) at the
    // head of the batch must skip cleanly without blocking the next
    // legitimate row. `row_to_message` returns None on the poison row,
    // the drainer logs it once, bumps `attempts`, and `continue`s — so
    // a single bad row from a half-rolled deployment doesn't strand
    // every subsequent valid event for the same entity.
    let pool = require_db_or_skip!();
    let entity_id = TEST_ENTITY_BASE + 6;
    cleanup(&pool, entity_id).await;
    purge_undelivered_backlog(&pool).await;

    // Hand-craft a poison row: event_type says "inventory_item_granted"
    // but the payload JSON shape is ItemUsed. row_to_message rejects
    // the (event_type, payload) pair.
    let poison_payload = serde_json::json!({
        "kind": "item_used",
        "type_id": 1,
        "target_id": 0,
    });
    let poison_id: i64 = sqlx::query_scalar(
        "INSERT INTO cell_event_outbox (entity_id, event_type, payload) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(entity_id as i32)
    .bind("inventory_item_granted")
    .bind(sqlx::types::Json(&poison_payload))
    .fetch_one(&pool)
    .await
    .unwrap();

    // Legitimate row enqueued *after* the poison row, so global id order
    // puts the poison row first in the batch.
    let good_payload = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 7,
        target_id: 3,
    };
    let good_id = enqueue(&pool, entity_id, &good_payload).await.unwrap();
    assert!(
        good_id > poison_id,
        "BIGSERIAL must place the good row after the poison row"
    );

    // Drain. Stats counts span the whole table (other tests' undelivered
    // rows would be drained alongside ours), so we assert per-row state
    // rather than global stats.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<BaseToCellMsg>(64);
    drain_undelivered(&pool, &tx).await.unwrap();

    // Poison row was bumped via record_failure but NOT marked delivered —
    // it's left undelivered for operator triage on the next pass.
    let (poison_attempts, poison_undelivered): (i32, bool) = sqlx::query_as(
        "SELECT attempts, delivered_at IS NULL FROM cell_event_outbox WHERE id = $1",
    )
    .bind(poison_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        poison_attempts, 1,
        "poison row must be flagged once per pass"
    );
    assert!(
        poison_undelivered,
        "poison row must remain undelivered (skipped, not dispatched)"
    );

    // Good row IS marked delivered — the poison row didn't block it.
    let good_delivered: bool =
        sqlx::query_scalar("SELECT delivered_at IS NOT NULL FROM cell_event_outbox WHERE id = $1")
            .bind(good_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        good_delivered,
        "legitimate row behind the poison row must drain successfully"
    );

    // Filter dispatched messages by sentinel entity_id: exactly the good
    // payload, no phantom message from the poison row.
    let hits = drain_item_used_for(&mut rx, entity_id);
    assert_eq!(
        hits,
        vec![(7, 3)],
        "exactly one ItemUsed for the good payload — poison row must not produce a message"
    );

    cleanup(&pool, entity_id).await;
}
