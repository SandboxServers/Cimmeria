use std::sync::{Arc, Barrier};
use std::thread;

use serde_json::json;

use super::*;

fn context(pid: u32, session: &str, monotonic_ms: u64) -> CaptureContext {
    CaptureContext {
        schema_version: 1,
        instance: "default".into(),
        client_pid: pid,
        client_fingerprint: "fixture".into(),
        player_entity_id: Some(10),
        selected_entity_id: None,
        server_session_id: Some(session.into()),
        host_ms: monotonic_ms as i64,
        monotonic_ms,
        server_offset_ms: None,
        source: vec!["client".into()],
        gaps: vec![],
    }
}

fn key(ctx: &CaptureContext) -> CaptureKey {
    CaptureKey::from(ctx)
}

#[test]
fn two_subscribers_receive_independent_cloned_rows() {
    let broker = CaptureBroker::new(4);
    let ctx = context(7, "s1", 100);
    let a = broker.subscribe(key(&ctx), "a".into()).unwrap();
    let b = broker.subscribe(key(&ctx), "b".into()).unwrap();
    broker.push(Source::Client, &ctx, json!({"marker": "original"}));
    broker.push(Source::Server, &ctx, json!({"marker": "server"}));

    let mut first = broker.read(a, 1);
    assert_eq!(
        (first.first_seq, first.last_seq, first.dropped),
        (Some(1), Some(1), 0)
    );
    first.rows[0].row["marker"] = json!("changed");
    let second = broker.read(b, 4);
    assert_eq!((second.first_seq, second.last_seq), (Some(1), Some(2)));
    assert_eq!(second.rows[0].row["marker"], "original");
    assert_eq!(broker.read(a, 4).first_seq, Some(2));
    assert!(broker.read(b, 4).rows.is_empty());
}

#[test]
fn overflow_and_expiration_report_exact_unread_losses() {
    let broker = CaptureBroker::new(2);
    let ctx = context(7, "s1", 0);
    let reader = broker.subscribe(key(&ctx), "slow".into()).unwrap();
    for n in 0..4 {
        broker.push(Source::Client, &ctx, json!(n));
    }
    broker.push(Source::Server, &ctx, json!("server"));
    let first = broker.read(reader, 1);
    assert_eq!(
        (first.first_seq, first.last_seq, first.dropped),
        (Some(3), Some(3), 2)
    );
    assert_eq!(first.rows[0].row, json!(2));

    broker.push(Source::Client, &context(7, "s1", 300_001), json!("new"));
    let next = broker.read(reader, 10);
    assert_eq!(
        (next.first_seq, next.last_seq, next.dropped),
        (Some(6), Some(6), 2)
    );
    assert_eq!(next.rows[0].row, json!("new"));
    assert_eq!(broker.read(reader, 10).dropped, 0);
}

#[test]
fn mismatched_identity_is_busy_and_cannot_mix_rows() {
    let broker = CaptureBroker::new(2);
    let ctx = context(7, "s1", 0);
    let id = broker.subscribe(key(&ctx), "one".into()).unwrap();
    let wrong_session = context(7, "s2", 1);
    assert_eq!(
        broker.subscribe(key(&wrong_session), "two".into()),
        Err(CaptureError::CaptureBusy)
    );
    assert_eq!(CaptureError::CaptureBusy.code(), "capture_busy");
    broker.push(Source::Client, &wrong_session, json!("wrong"));
    broker.push(Source::Client, &context(8, "s1", 1), json!("wrong pid"));
    broker.push(Source::Client, &ctx, json!("right"));
    assert_eq!(broker.read(id, 10).rows[0].row, json!("right"));
    assert_eq!(broker.rejections().identity, 2);
    broker.release(id);
    let replacement = broker.subscribe(key(&wrong_session), "two".into()).unwrap();
    assert!(broker.read(replacement, 10).rows.is_empty());
}

#[test]
fn out_of_order_row_expires_behind_a_newer_front_row() {
    let broker = CaptureBroker::new(4);
    let ctx = context(7, "s1", 100_000);
    let reader = broker.subscribe(key(&ctx), "reader".into()).unwrap();
    broker.push(Source::Client, &ctx, json!("front"));
    broker.push(
        Source::Client,
        &context(7, "s1", 50_000),
        json!("old behind front"),
    );
    broker.push(Source::Client, &context(7, "s1", 350_001), json!("latest"));
    let batch = broker.read(reader, 4);
    assert_eq!(batch.dropped, 1);
    assert_eq!(
        batch.rows.iter().map(|row| &row.row).collect::<Vec<_>>(),
        vec![&json!("front"), &json!("latest")]
    );

    broker.push(
        Source::Client,
        &context(7, "s1", 50_000),
        json!("already stale"),
    );
    assert_eq!(broker.rejections().stale, 1);
    assert!(broker.read(reader, 4).rows.is_empty());
}

#[test]
fn oversized_json_is_rejected_without_consuming_history_capacity() {
    let broker = CaptureBroker::new(1);
    let ctx = context(7, "s1", 0);
    let reader = broker.subscribe(key(&ctx), "reader".into()).unwrap();
    broker.push(Source::Client, &ctx, json!("x".repeat(MAX_ROW_BYTES - 2)));
    broker.push(Source::Client, &ctx, json!("x".repeat(MAX_ROW_BYTES - 1)));
    let batch = broker.read(reader, 2);
    assert_eq!(batch.rows.len(), 1);
    assert_eq!(batch.rows[0].row.as_str().unwrap().len(), MAX_ROW_BYTES - 2);
    assert_eq!(batch.dropped, 0);
    assert_eq!(broker.rejections().oversized, 1);
}

#[test]
fn concurrent_readers_do_not_steal_each_others_history() {
    let broker = Arc::new(CaptureBroker::new(128));
    let ctx = context(7, "s1", 1);
    let left = broker.subscribe(key(&ctx), "left".into()).unwrap();
    let right = broker.subscribe(key(&ctx), "right".into()).unwrap();
    for n in 0..100 {
        broker.push(Source::Client, &ctx, json!(n));
    }
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = [left, right]
        .into_iter()
        .map(|id| {
            let broker = Arc::clone(&broker);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                broker.read(id, 100)
            })
        })
        .collect();
    barrier.wait();
    for handle in handles {
        let batch = handle.join().unwrap();
        assert_eq!(batch.rows.len(), 100);
        assert_eq!(
            (batch.first_seq, batch.last_seq, batch.dropped),
            (Some(1), Some(100), 0)
        );
    }
}
