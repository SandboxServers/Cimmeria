//! CH-13 (D-CH9): `.spawnrandom` batches. The table's own rules (counting,
//! the summary format the lab runner parses, timeouts, parking a lab reply,
//! remembering closed batches) and the command's request shape. The round
//! trip through the cell's message handlers and the lab console tee is
//! tested in `cimmeria-cell` (`base_messages/tests/spawnrandom_*.rs`).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::console::spawn_batch::{
    batches, LabWaiter, Recorded, Requester, SpawnBatches, CHANNEL_ERROR, CLOSED_BATCH_MEMORY,
    SPAWN_BATCH_TIMEOUT,
};
use crate::cell::messages::CellToBaseMsg;

fn gm(entity_id: u32) -> Requester {
    Requester {
        entity_id,
        account_id: Some(1),
        player_id: Some(1),
    }
}

fn summary(r: Recorded) -> String {
    match r {
        Recorded::Completed(c) => c.summary,
        other => panic!("expected the batch to close, got {other:?}"),
    }
}

fn waiter() -> LabWaiter {
    let (reply_tx, _rx) = oneshot::channel();
    LabWaiter {
        lines: vec![],
        reply_tx,
    }
}

/// Every copy spawned: the summary lists every id, in arrival order, in the
/// exact format the lab runner's `burst` parser reads.
#[test]
fn summary_lists_every_spawned_id() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 4321, 3, now);
    assert!(matches!(t.spawned(b, 101, now), Recorded::Pending));
    assert!(matches!(t.spawned(b, 102, now), Recorded::Pending));
    assert_eq!(
        summary(t.spawned(b, 103, now)),
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
    let b = t.open(gm(1), 4321, 5, now);
    t.spawned(b, 101, now);
    t.failed(b, "template 4321 not found", now);
    t.failed(b, "template 4321 not found", now);

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
    assert_eq!((done[0].failed, done[0].timed_out), (2, 2));
}

/// Two distinct reasons are named in first-seen order, joined by `; `
/// inside the parentheses (the contract CH-08's parser relies on).
#[test]
fn summary_joins_distinct_reasons_with_a_semicolon() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 3, now);
    t.failed(b, "template 7 not found", now);
    t.spawned(b, 40, now);
    assert_eq!(
        summary(t.failed(b, "could not place it in the space", now)),
        "spawnrandom: spawned 1/3 x template 7: ids 40; \
         2 failed (template 7 not found; could not place it in the space)"
    );
}

/// Nothing spawned: `ids none`, so the parser never sees an empty list.
#[test]
fn summary_with_no_ids_says_none() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 1, now);
    assert_eq!(
        summary(t.failed(b, "no database to resolve the template", now)),
        "spawnrandom: spawned 0/1 x template 7: ids none; \
         1 failed (no database to resolve the template)"
    );
}

/// Copies that never reached the base count as failed at once, so the batch
/// closes on the copies that did.
#[test]
fn undelivered_copies_fail_without_waiting() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 3, now);
    assert!(t.undelivered(b, 2, CHANNEL_ERROR, now).is_none());
    assert_eq!(
        summary(t.spawned(b, 9, now)),
        "spawnrandom: spawned 1/3 x template 7: ids 9; 2 failed (cell→base channel error)"
    );
}

/// When no copy reached the base, `undelivered` closes the batch itself:
/// nothing else ever would, short of the timeout. Revert proof: return
/// `None` regardless and the lab tee's reply waits five seconds.
#[test]
fn undelivered_closes_a_batch_with_nothing_outstanding() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 2, now);
    let done = t
        .undelivered(b, 2, CHANNEL_ERROR, now)
        .expect("nothing is outstanding");
    assert_eq!(
        done.summary,
        "spawnrandom: spawned 0/2 x template 7: ids none; 2 failed (cell→base channel error)"
    );
    assert_eq!(t.open_count(), 0);
}

/// A result for a batch that already closed is `Late` and never counted
/// again (a duplicate of the last copy cannot reopen or re-report it); a
/// batch id never opened is unknown.
#[test]
fn a_result_for_a_closed_batch_is_late_and_an_unknown_id_is_unknown() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 1, now);
    summary(t.spawned(b, 9, now));
    match t.spawned(b, 10, now) {
        Recorded::Late(late) => {
            assert_eq!(late.requester, gm(1));
            assert!(!late.lab, "no lab caller waited on this batch");
        }
        other => panic!("expected Late, got {other:?}"),
    }
    assert!(matches!(t.failed(42, "x", now), Recorded::UnknownBatch));
}

/// A late copy of a batch a lab caller waited on says so, which is what
/// makes the cell despawn it.
#[test]
fn a_late_copy_of_a_lab_batch_is_marked_lab() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let mark = t.next_id();
    let b = t.open(gm(1), 7, 2, now);
    t.attach_lab(1, mark, waiter()).expect("the caller's batch");
    assert_eq!(t.expire(now + SPAWN_BATCH_TIMEOUT).len(), 1);
    assert!(matches!(
        t.spawned(b, 9, now + SPAWN_BATCH_TIMEOUT),
        Recorded::Late(late) if late.lab
    ));
}

/// Closed batches are forgotten after [`CLOSED_BATCH_MEMORY`], so the set
/// stays small.
#[test]
fn closed_batches_are_forgotten_after_a_minute() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let b = t.open(gm(1), 7, 1, now);
    summary(t.spawned(b, 9, now));
    assert_eq!(t.closed_count(), 1);

    t.expire(now + CLOSED_BATCH_MEMORY - SPAWN_BATCH_TIMEOUT);
    assert_eq!(t.closed_count(), 1, "still remembered within the minute");
    t.expire(now + CLOSED_BATCH_MEMORY);
    assert_eq!(t.closed_count(), 0);
    assert!(matches!(t.spawned(b, 10, now), Recorded::UnknownBatch));
}

/// The lab tee parks its reply only on a batch the caller opened during the
/// command: not another GM's, and not one opened before the mark.
#[test]
fn attach_lab_finds_only_the_callers_new_batch() {
    let mut t = SpawnBatches::default();
    let now = Instant::now();
    let old = t.open(gm(1), 7, 1, now);
    let mark = t.next_id();
    let _other_gm = t.open(gm(2), 7, 1, now);

    let w = t
        .attach_lab(1, mark, waiter())
        .expect_err("no batch of caller 1 opened after the mark");

    let mine = t.open(gm(1), 7, 1, now);
    assert!(t.attach_lab(1, mark, w).is_ok());
    let done = summary(t.spawned(old, 5, now));
    assert!(
        done.contains("ids 5"),
        "the old batch closes without a waiter"
    );
    match t.spawned(mine, 6, now) {
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

/// The base channel closes part-way through the copies: the ones that never
/// left count as failed with the channel-error reason at once, and only the
/// delivered ones are left to answer or time out. A one-slot channel and a
/// reader that takes two requests then hangs up make the send fail
/// mid-loop. Revert proof: drop the `undelivered` call and the summary has
/// no `failed` part, every missing copy reads as timed out.
#[tokio::test(start_paused = true)]
async fn spawnrandom_counts_copies_the_channel_refused_mid_loop() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(1);
    let reader = tokio::spawn(async move {
        for _ in 0..2 {
            rx.recv().await;
        }
        drop(rx);
    });

    handle_console_command(
        gm,
        ".spawnrandom 4321 3 3 5",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    reader.await.unwrap();

    let table = batches(&mut mgr);
    assert_eq!(table.open_count(), 1, "some copies were delivered");
    let done = table.expire(Instant::now() + SPAWN_BATCH_TIMEOUT);
    let s = &done[0].summary;
    assert!(
        s.starts_with("spawnrandom: spawned 0/5 x template 4321: ids none; "),
        "got {s}"
    );
    assert!(
        s.contains(" failed (cell→base channel error)"),
        "copies the channel refused are failed, not timed out; got {s}"
    );
    assert!(
        s.ends_with(" timed out"),
        "delivered copies time out; got {s}"
    );
    assert_eq!(
        done[0].failed + done[0].timed_out,
        5,
        "every copy is accounted for"
    );
}
