//! Placing client rows on the server clock and in a space (NT-40), before
//! the cell is asked anything.

use std::time::{Duration, SystemTime};

use serde_json::json;

use cimmeria_services::cell::messages::ENTITY_LABEL_QUERY_CAP;

use super::dto::{ClientNativeEvent, TelemetryEvent};
use super::entity_labels::{resolve, NamingPlan, SessionClock};

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

/// Each chunk's observed offset is the true offset plus that chunk's
/// upload delay; the session keeps the smallest.
#[test]
fn the_session_keeps_the_smallest_offset_seen() {
    let mut clock = SessionClock::default();
    // Chunk 1 sat in the queue for 5 s: observed offset 1_000_000 + 5_000.
    NamingPlan::build(&[native(10_000, json!({}))], at(1_015_000), &mut clock);
    assert_eq!(clock.offset_ms, Some(1_005_000));
    // Chunk 2 went out at once: the true offset.
    NamingPlan::build(&[native(20_000, json!({}))], at(1_020_000), &mut clock);
    assert_eq!(clock.offset_ms, Some(1_000_000));
    // Chunk 3 was a slow retry: the offset doesn't grow back.
    NamingPlan::build(&[native(30_000, json!({}))], at(1_090_000), &mut clock);
    assert_eq!(clock.offset_ms, Some(1_000_000));
}

/// A row is placed in the last space the session reported, across chunks;
/// a row before any report is not placed, so nothing is asked for it.
#[test]
fn rows_take_the_last_reported_space_across_chunks() {
    let mut clock = SessionClock::default();
    let before = NamingPlan::build(
        &[native(1_000, json!({ "entity_id": 5 }))],
        at(2_000),
        &mut clock,
    );
    assert!(before.queries().is_empty(), "no space reported yet");

    let first = NamingPlan::build(
        &[
            native(3_000, json!({ "entity_id": 5, "space_id": 7 })),
            native(4_000, json!({ "target_id": 6 })),
        ],
        at(5_000),
        &mut clock,
    );
    let spaces: Vec<_> = first
        .queries()
        .iter()
        .map(|q| (q.entity_id, q.space_id))
        .collect();
    assert_eq!(spaces, [(5, 7), (6, 7)]);

    let next = NamingPlan::build(
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

/// A row's server time is its client time plus the offset, and the newest
/// row of a chunk lands on the receive time.
#[test]
fn a_row_is_placed_at_its_client_time_plus_the_offset() {
    let mut clock = SessionClock {
        space_id: Some(1),
        ..SessionClock::default()
    };
    let plan = NamingPlan::build(
        &[
            native(40_000, json!({ "entity_id": 5 })),
            native(100_000, json!({ "entity_id": 6 })),
        ],
        at(5_100_000),
        &mut clock,
    );
    let times: Vec<_> = plan.queries().iter().map(|q| q.at).collect();
    assert_eq!(times, [at(5_040_000), at(5_100_000)]);
}

/// The same entity at the same moment is asked about once, and a chunk
/// never asks for more than the cell answers.
#[test]
fn queries_are_deduplicated_and_capped() {
    let mut clock = SessionClock {
        space_id: Some(1),
        ..SessionClock::default()
    };
    let mut events = vec![
        native(1_000, json!({ "entity_id": 5, "target_id": 5 })),
        native(1_000, json!({ "source_id": 5 })),
    ];
    let plan = NamingPlan::build(&events, at(1_000), &mut clock);
    assert_eq!(plan.queries().len(), 1);

    events = (0..ENTITY_LABEL_QUERY_CAP as u32 + 10)
        .map(|i| native(1_000, json!({ "entity_id": i + 1 })))
        .collect();
    let plan = NamingPlan::build(&events, at(1_000), &mut clock);
    assert_eq!(plan.queries().len(), ENTITY_LABEL_QUERY_CAP);
}

/// With no cell to ask, every row replays unnamed.
#[tokio::test]
async fn no_cell_names_nothing() {
    let mut clock = SessionClock {
        space_id: Some(1),
        ..SessionClock::default()
    };
    let plan = NamingPlan::build(
        &[native(1_000, json!({ "entity_id": 5 }))],
        at(1_000),
        &mut clock,
    );
    let labels = resolve(plan, None).await;
    assert_eq!(labels.label(0, 5), None);
}
