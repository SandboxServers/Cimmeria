//! Outbox tests that don't need a live database.
//!
//! The DB-touching paths (`enqueue`, `enqueue_in_tx`, `mark_delivered`,
//! `record_failure`, `drain_undelivered`) are tested via the integration
//! suite gated on `sqlx::test` infra (issue #79). This file covers the
//! pure logic: payload serialization shape and row→message conversion.

use super::*;

#[test]
fn payload_serializes_with_kind_tag() {
    let p = CellOutboxPayload::ItemUsed {
        type_id: 42,
        target_id: 17,
    };
    let json = serde_json::to_value(&p).unwrap();
    // kind tag matters: it's how a future variant gets disambiguated on
    // deserialize, and an accidental rename would silently break in-flight
    // rows on production databases.
    assert_eq!(json["kind"], "item_used");
    assert_eq!(json["type_id"], 42);
    assert_eq!(json["target_id"], 17);
}

#[test]
fn payload_roundtrips_through_json() {
    let p = CellOutboxPayload::ItemUsed {
        type_id: -7,
        target_id: 0,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn event_type_string_is_stable() {
    // This string is persisted in the `event_type` column. Changing it
    // breaks any in-flight rows on existing databases — the test exists
    // to make that breakage loud at refactor time.
    assert_eq!(
        CellOutboxPayload::ItemUsed {
            type_id: 0,
            target_id: 0
        }
        .event_type(),
        "item_used",
    );
}

#[test]
fn row_to_message_builds_item_used() {
    let row = OutboxRow {
        id: 99,
        entity_id: 1234,
        event_type: "item_used".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            type_id: 19,
            target_id: 5,
        }),
        attempts: 0,
    };
    let msg = row_to_message(&row).expect("known event_type should produce message");
    match msg {
        BaseToCellMsg::ItemUsed {
            entity_id,
            type_id,
            target_id,
        } => {
            assert_eq!(entity_id, 1234);
            assert_eq!(type_id, 19);
            assert_eq!(target_id, 5);
        }
        _ => panic!("expected ItemUsed"),
    }
}

#[test]
fn row_to_message_returns_none_for_unknown_event_type() {
    // Forward-compat: a future base might enqueue an event_type this
    // build doesn't know. Drainer must skip it (and rate-limit logging
    // off `attempts`), not panic / mis-dispatch.
    let row = OutboxRow {
        id: 100,
        entity_id: 1,
        event_type: "future_event_v2".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            type_id: 0,
            target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}

#[test]
fn persistence_contract_strings_are_stable_for_all_variants() {
    // Each variant has TWO persistence contracts that must agree:
    //   - `event_type()` returns the string written to the
    //     `cell_event_outbox.event_type` VARCHAR column.
    //   - serde's `tag = "kind"` writes the same string into the
    //     payload JSONB body.
    // `row_to_message` matches on (event_type, payload) so both must
    // line up; pin them together here. Changing either string breaks
    // every in-flight row on existing databases.
    let cases: &[(CellOutboxPayload, &str)] = &[
        (
            CellOutboxPayload::ItemUsed {
                type_id: 0,
                target_id: 0,
            },
            "item_used",
        ),
        (
            CellOutboxPayload::InventoryItemGranted {
                item_id: 0,
                container_id: 0,
                slot_id: 0,
                quantity: 0,
            },
            "inventory_item_granted",
        ),
        (
            CellOutboxPayload::InventoryItemRemoved {
                item_id: 0,
                source_container_id: 0,
            },
            "inventory_item_removed",
        ),
    ];

    for (variant, expected) in cases {
        assert_eq!(
            variant.event_type(),
            *expected,
            "event_type() drift for {variant:?}",
        );
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(
            json["kind"], *expected,
            "JSON `kind` tag drift for {variant:?}",
        );
    }
}

#[test]
fn inventory_item_granted_roundtrips() {
    let p = CellOutboxPayload::InventoryItemGranted {
        item_id: 100_001,
        container_id: 1,
        slot_id: 5,
        quantity: 3,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn inventory_item_removed_roundtrips() {
    let p = CellOutboxPayload::InventoryItemRemoved {
        item_id: 100_002,
        source_container_id: 3,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn row_to_message_builds_inventory_item_granted() {
    let row = OutboxRow {
        id: 200,
        entity_id: 42,
        event_type: "inventory_item_granted".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::InventoryItemGranted {
            item_id: 100_007,
            container_id: 1,
            slot_id: 2,
            quantity: 1,
        }),
        attempts: 0,
    };
    match row_to_message(&row).expect("known event_type") {
        BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id,
            container_id,
            slot_id,
            quantity,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(item_id, 100_007);
            assert_eq!(container_id, 1);
            assert_eq!(slot_id, 2);
            assert_eq!(quantity, 1);
        }
        _ => panic!("expected InventoryItemGranted"),
    }
}

#[test]
fn row_to_message_builds_inventory_item_removed() {
    let row = OutboxRow {
        id: 201,
        entity_id: 42,
        event_type: "inventory_item_removed".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::InventoryItemRemoved {
            item_id: 100_008,
            source_container_id: 3,
        }),
        attempts: 0,
    };
    match row_to_message(&row).expect("known event_type") {
        BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id,
            source_container_id,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(item_id, 100_008);
            assert_eq!(source_container_id, 3);
        }
        _ => panic!("expected InventoryItemRemoved"),
    }
}

#[test]
fn row_to_message_returns_none_on_event_type_payload_mismatch() {
    // Defensive: an event_type that says "granted" but a payload that
    // shape-matches a different variant (legacy data, hand-edited row,
    // future variant rename half-rolled) must not silently dispatch the
    // wrong message — drainer should skip and log.
    let row = OutboxRow {
        id: 202,
        entity_id: 1,
        event_type: "inventory_item_granted".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            type_id: 0,
            target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}

#[test]
fn drain_stats_default_is_all_zero() {
    // The drainer's "no work" path returns `Default` and the periodic-log
    // gate suppresses on all-zero. Pin the default values so a future
    // refactor doesn't accidentally turn an empty drain into spam.
    let s = DrainStats::default();
    assert_eq!(s.delivered, 0);
    assert_eq!(s.skipped_bad, 0);
    assert_eq!(s.send_failed, 0);
}

// ── Live-DB integration tests (skip when DATABASE_URL is unset) ─────────
//
// These exercise the SQL paths (`enqueue_in_tx`, `mark_delivered`,
// `record_failure`, `drain_undelivered`) end-to-end against a real
// Postgres. Setup + rationale: docs/architecture/integration-test-infra.md.
//
// Each test scopes its writes by a sentinel `entity_id` and cleans up
// at the end so tests don't poison each other.

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
        type_id: 1,
        target_id: 0,
    };
    let payload_b = CellOutboxPayload::ItemUsed {
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

    let payload = CellOutboxPayload::ItemUsed {
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

    let payload = CellOutboxPayload::ItemUsed {
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
