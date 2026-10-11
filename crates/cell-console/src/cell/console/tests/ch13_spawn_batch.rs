//! CH-13 (D-CH9): `.spawnrandom` batches. The table's own rules (counting,
//! the summary format the lab runner parses, timeouts, parking a lab reply)
//! and the command's request shape. The round trip through the cell's
//! message handlers and the lab console tee is tested in `cimmeria-cell`
//! (`base_messages/tests/spawnrandom_batch.rs`).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::console::spawn_batch::{
    batches, LabWaiter, Recorded, SpawnBatches, SPAWN_BATCH_TIMEOUT,
};
use crate::cell::messages::CellToBaseMsg;

fn summary(r: Recorded) -> String {
    match r {
        Recorded::Completed(c) => c.summary,
        other => panic!("expected the batch to close, got {other:?}"),
    }
}

/// Every copy spawned: the summary lists every id, in arrival order, in the
/// exact format the lab runner's `burst` parser reads.
#[test]
fn summary_lists_every_spawned_id() {
    let mut t = SpawnBatches::default();
    let b = t.open(1, 4321, 3, Instant::now());
    assert!(matches!(t.spawned(b, 101), Recorded::Pending));
    assert!(matches!(t.spawned(b, 102), Recorded::Pending));
    assert_eq!(
        summary(t.spawned(b, 103)),
        "spawnrandom: spawned 3/3 x template 4321: ids 101,102,103"
    );
    assert_eq!(t.open_count(), 0, "a reported batch is closed");
}

/// Failures and timeouts are reported with their counts; a repeated reason
/// is named once.
#[test]
fn summary_reports_failures_and_timeouts_honestly() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(1, 4321, 5, now);
    t.spawned(b, 101);
    t.failed(b, "template 4321 not found");
    t.failed(b, "template 4321 not found");

    assert!(
        t.expire(now).is_empty(),
        "nothing is due before the timeout"
    );
    let done = t.expire(now + SPAWN_BATCH_TIMEOUT);
    assert_eq!(done.len(), 1);
    assert_eq!(
        done[0].summary,
        "spawnrandom: spawned 1/5 x template 4321: ids 101; \
         2 failed (template 4321 not found); 2 timed out"
    );
}

/// Nothing spawned: `ids none`, so the parser never sees an empty list.
#[test]
fn summary_with_no_ids_says_none() {
    let mut t = SpawnBatches::default();
    let b = t.open(1, 7, 1, Instant::now());
    assert_eq!(
        summary(t.failed(b, "no database to resolve the template")),
        "spawnrandom: spawned 0/1 x template 7: ids none; \
         1 failed (no database to resolve the template)"
    );
}

/// Copies that never reached the base count as failed at once, so the batch
/// closes on the copies that did.
#[test]
fn undelivered_copies_fail_without_waiting() {
    let mut t = SpawnBatches::default();
    let b = t.open(1, 7, 3, Instant::now());
    t.undelivered(b, 2, "cell→base channel error");
    assert_eq!(
        summary(t.spawned(b, 9)),
        "spawnrandom: spawned 1/3 x template 7: ids 9; 2 failed (cell→base channel error)"
    );
}

/// A result for a closed or unknown batch is handed back, never counted
/// (the caller reports that copy on its own line).
#[test]
fn a_late_or_duplicate_result_is_unknown() {
    let mut t = SpawnBatches::default();
    let b = t.open(1, 7, 1, Instant::now());
    summary(t.spawned(b, 9));
    assert!(matches!(t.spawned(b, 10), Recorded::UnknownBatch));
    assert!(matches!(t.failed(42, "x"), Recorded::UnknownBatch));
}

/// The lab tee parks its reply only on a batch the caller opened during the
/// command: not another GM's, and not one opened before the mark.
#[test]
fn attach_lab_finds_only_the_callers_new_batch() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let old = t.open(1, 7, 1, now);
    let mark = t.next_id();
    let _other_gm = t.open(2, 7, 1, now);

    let (reply_tx, _rx) = oneshot::channel();
    let waiter = LabWaiter {
        lines: vec![],
        reply_tx,
    };
    let waiter = t
        .attach_lab(1, mark, waiter)
        .expect_err("no batch of caller 1 opened after the mark");

    let mine = t.open(1, 7, 1, now);
    assert!(t.attach_lab(1, mark, waiter).is_ok());
    let done = summary(t.spawned(old, 5));
    assert!(
        done.contains("ids 5"),
        "the old batch closes without a waiter"
    );
    match t.spawned(mine, 6) {
        Recorded::Completed(c) => assert!(c.lab.is_some(), "the waiter rode on the new batch"),
        other => panic!("expected the batch to close, got {other:?}"),
    }
}

/// `.spawnrandom` tags every copy with one batch id and opens that batch;
/// the immediate line is the acknowledgement, with no per-copy line yet.
#[tokio::test]
async fn spawnrandom_tags_every_copy_with_one_batch() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);

    handle_console_command(
        gm,
        ".spawnrandom 4321 3 3 4",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let tags: Vec<Option<u32>> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmSpawnNpc { batch, .. } => Some(*batch),
            _ => None,
        })
        .collect();
    assert_eq!(tags.len(), 4, "one request per copy");
    assert!(tags[0].is_some(), "a .spawnrandom copy is batched");
    assert!(
        tags.iter().all(|t| *t == tags[0]),
        "all copies share the batch"
    );
    assert_eq!(batches(&mut mgr).open_count(), 1);

    let fb: Vec<String> = msgs.iter().filter_map(decode_feedback).collect();
    assert_eq!(
        fb,
        vec!["spawnrandom: requested 4 x template 4321".to_string()]
    );
}

/// A closed channel delivers nothing, so the command must not leave a batch
/// open that would only ever time out.
#[tokio::test]
async fn spawnrandom_with_nothing_delivered_opens_no_batch() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, rx) = mpsc::channel::<CellToBaseMsg>(64);
    drop(rx);

    handle_console_command(
        gm,
        ".spawnrandom 4321 3 3 2",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    assert_eq!(batches(&mut mgr).open_count(), 0);
}
