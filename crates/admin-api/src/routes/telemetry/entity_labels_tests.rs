//! Placing client rows on the server clock and in a space (NT-40), and the
//! bounds on what a chunk may ask of the cell.

use std::time::{Duration, Instant, SystemTime};

use serde_json::json;
use tokio::sync::mpsc;

use cimmeria_services::cell::messages::{EntityLabelsRequest, ENTITY_LABEL_QUERY_CAP};

use super::dto::{ClientNativeEvent, TelemetryEvent};
use super::entity_labels::{
    resolve, EntityLabelLink, NamingPlan, SessionClock, SessionClocks, CELL_REPLY_TIMEOUT,
    MAX_SESSIONS_PER_INSTALL, OFFSET_WINDOW,
};

fn native(ts_ms: i64, fields: serde_json::Value) -> TelemetryEvent {
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms,
        seq: 0,
        target: "client.ability.recv".into(),
        level: "debug".into(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    })
}

fn at(ms: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(ms)
}

fn all(events: &[TelemetryEvent]) -> Vec<bool> {
    vec![true; events.len()]
}

fn in_space(space: u32) -> SessionClock {
    SessionClock::in_space(space)
}

/// Build a plan for `events`, every row admitted.
fn plan(events: &[TelemetryEvent], recv: SystemTime, clock: &mut SessionClock) -> NamingPlan {
    NamingPlan::build(events, &all(events), recv, clock)
}

/// Each chunk's observed offset is the true offset plus its upload delay;
/// the session takes the smallest in the window.
#[test]
fn the_offset_is_the_smallest_seen_in_the_window() {
    let mut clock = SessionClock::default();
    // Chunk 1 sat in the queue for 5 s.
    plan(&[native(10_000, json!({}))], at(1_015_000), &mut clock);
    assert_eq!(clock.offset_ms(), Some(1_005_000));
    // Chunk 2 went out at once: the true offset.
    plan(&[native(20_000, json!({}))], at(1_020_000), &mut clock);
    assert_eq!(clock.offset_ms(), Some(1_000_000));
    // Chunk 3 was a slow retry: the offset doesn't grow back.
    plan(&[native(30_000, json!({}))], at(1_090_000), &mut clock);
    assert_eq!(clock.offset_ms(), Some(1_000_000));
}

/// A sample older than the window ages out, so one wrong clock reading (or
/// a clock step) skews the session for at most the window, not for good.
#[test]
fn an_old_small_offset_ages_out_of_the_window() {
    let mut clock = SessionClock::default();
    // One row stamped from a clock a minute fast: offset 60 s too small.
    plan(&[native(70_000, json!({}))], at(1_010_000), &mut clock);
    assert_eq!(clock.offset_ms(), Some(940_000));
    let later = 1_010_000 + OFFSET_WINDOW.as_millis() as u64 + 1_000;
    plan(
        &[native(later as i64 - 1_000_000, json!({}))],
        at(later),
        &mut clock,
    );
    assert_eq!(
        clock.offset_ms(),
        Some(1_000_000),
        "the fast sample aged out"
    );
}

/// A row is placed in the last space the session reported, across chunks;
/// a row before any report is not placed, so nothing is asked for it.
#[test]
fn rows_take_the_last_reported_space_across_chunks() {
    let mut clock = SessionClock::default();
    let before = plan(
        &[native(1_000, json!({ "entity_id": 5 }))],
        at(2_000),
        &mut clock,
    );
    assert!(before.queries().is_empty(), "no space reported yet");

    let first = plan(
        &[
            native(3_000, json!({ "entity_id": 5, "space_id": 7 })),
            native(4_000, json!({ "target_id": 6 })),
        ],
        at(5_000),
        &mut clock,
    );
    let pairs: Vec<_> = first
        .queries()
        .iter()
        .map(|q| (q.entity_id, q.space_id))
        .collect();
    assert_eq!(pairs, [(5, 7), (6, 7)]);

    let next = plan(
        &[native(6_000, json!({ "pet_id": 8 }))],
        at(7_000),
        &mut clock,
    );
    assert_eq!(
        next.queries()[0].space_id,
        7,
        "carried from the previous chunk"
    );
}

/// A row's server time is its client time plus the offset; the newest row
/// of a chunk lands on the receive time.
#[test]
fn a_row_is_placed_at_its_client_time_plus_the_offset() {
    let mut clock = in_space(1);
    let p = plan(
        &[
            native(40_000, json!({ "entity_id": 5 })),
            native(100_000, json!({ "entity_id": 6 })),
        ],
        at(5_100_000),
        &mut clock,
    );
    let times: Vec<_> = p.queries().iter().map(|q| q.at).collect();
    assert_eq!(times, [at(5_040_000), at(5_100_000)]);
}

/// One question per (space, entity) per chunk, at its first row's time,
/// whatever timestamps the client puts on the rest: distinct `ts_ms` can't
/// multiply the cell's work.
#[test]
fn one_question_per_entity_per_chunk() {
    let mut clock = in_space(1);
    let events: Vec<_> = (0..500)
        .map(|i| native(1_000 + i, json!({ "entity_id": 5, "target_id": 5 })))
        .collect();
    let p = plan(&events, at(10_000), &mut clock);
    assert_eq!(p.queries().len(), 1);
    assert_eq!(p.queries()[0].at, at(10_000 - 499));

    let events: Vec<_> = (0..ENTITY_LABEL_QUERY_CAP as u32 + 10)
        .map(|i| native(1_000, json!({ "entity_id": i + 1 })))
        .collect();
    let p = plan(&events, at(1_000), &mut clock);
    assert_eq!(p.queries().len(), ENTITY_LABEL_QUERY_CAP);
}

/// Rows the session budget refuses are never asked about.
#[test]
fn rows_the_budget_refuses_are_not_asked_about() {
    let mut clock = in_space(1);
    let events = [
        native(1_000, json!({ "entity_id": 5 })),
        native(1_000, json!({ "entity_id": 6 })),
    ];
    let p = NamingPlan::build(&events, &[false, true], at(1_000), &mut clock);
    let ids: Vec<_> = p.queries().iter().map(|q| q.entity_id).collect();
    assert_eq!(ids, [6]);
}

/// One install minting many sessions evicts its own oldest clock, never
/// another install's.
#[test]
fn an_install_is_capped_and_evicts_its_own_sessions() {
    let mut clocks = SessionClocks::default();
    clocks.entry("honest", "install-a", 0);
    for i in 0..(MAX_SESSIONS_PER_INSTALL as i64 + 3) {
        clocks.entry(&format!("spam-{i}"), "install-b", 1 + i);
    }
    assert!(clocks.contains("honest"));
    assert!(!clocks.contains("spam-0"), "install-b's oldest went first");
    assert!(clocks.contains(&format!("spam-{}", MAX_SESSIONS_PER_INSTALL + 2)));
}

fn one_question() -> NamingPlan {
    let mut clock = in_space(1);
    plan(
        &[native(1_000, json!({ "entity_id": 5 }))],
        at(1_000),
        &mut clock,
    )
}

/// With no link, every row replays unnamed.
#[tokio::test]
async fn no_link_names_nothing() {
    let labels = resolve(one_question(), None).await;
    assert_eq!(labels.label(0, 5), None);
}

/// A cell that never answers, asked by many chunks at once: at most one
/// request ever reaches the label channel, and every chunk comes back
/// unnamed within the reply timeout instead of queueing. Without the
/// in-flight permit, every chunk would put a request on the channel.
#[tokio::test]
async fn concurrent_chunks_never_queue_behind_a_silent_cell() {
    // Room for all of them, so only the permit can hold the line.
    let (tx, rx) = mpsc::channel::<EntityLabelsRequest>(64);
    let link = EntityLabelLink::new(tx);
    let started = Instant::now();
    let chunks: Vec<_> = (0..8)
        .map(|_| {
            let link = link.clone();
            tokio::spawn(async move { resolve(one_question(), Some(&link)).await })
        })
        .collect();
    for c in chunks {
        assert_eq!(c.await.unwrap().label(0, 5), None);
    }
    assert!(
        rx.len() <= 1,
        "{} requests reached the cell's channel; at most one may be in flight",
        rx.len()
    );
    assert!(
        started.elapsed() < CELL_REPLY_TIMEOUT * 2,
        "a chunk waited its turn"
    );
    drop(rx);
}

/// A full label channel replays the chunk unnamed at once; the request
/// goes nowhere else (the ingest holds no gameplay sender at all).
#[tokio::test]
async fn a_full_label_channel_replays_unnamed_at_once() {
    let (tx, mut rx) = mpsc::channel::<EntityLabelsRequest>(1);
    let (filler_tx, _filler_rx) = tokio::sync::oneshot::channel();
    tx.try_send(EntityLabelsRequest {
        queries: Vec::new(),
        reply_tx: filler_tx,
    })
    .unwrap();
    let link = EntityLabelLink::new(tx);
    let started = Instant::now();
    let labels = resolve(one_question(), Some(&link)).await;
    assert_eq!(labels.label(0, 5), None);
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "it waited for room"
    );
    assert_eq!(rx.len(), 1, "only the filler is queued");
    assert!(rx.recv().await.unwrap().queries.is_empty());
}
