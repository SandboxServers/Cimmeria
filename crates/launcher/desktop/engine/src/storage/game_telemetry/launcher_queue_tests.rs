use super::launcher_event::LauncherRecord;
use super::launcher_queue::{is_flush_trigger, LauncherQueue, FLUSH_TARGET, QUEUE_CAP, SEND_MAX};
use super::transport::TelemetryEvent;
use cimmeria_launcher_core::telemetry::events::ClientLogEvent;
use serde_json::{Map, Value};
use std::time::{Duration, Instant};

/// A client log event whose `seq` tells the test which event it is.
fn event(seq: u64) -> TelemetryEvent {
    TelemetryEvent::ClientLog(ClientLogEvent {
        ts_ms: 1,
        seq,
        source_file: "launcher".into(),
        level: "info".into(),
        category: "adoption".into(),
        packet_no: None,
        message: "step".into(),
    })
}

/// `count` events numbered from `first`.
fn events(first: u64, count: u64) -> Vec<TelemetryEvent> {
    (first..first + count).map(event).collect()
}

fn seq_of(event: &TelemetryEvent) -> u64 {
    match event {
        TelemetryEvent::ClientLog(log) => log.seq,
        _ => panic!("the queue test only builds client log events"),
    }
}

fn record(target: &'static str, name: Option<&str>) -> LauncherRecord {
    let mut fields = Map::new();
    if let Some(name) = name {
        fields.insert("event".into(), Value::String(name.to_string()));
    }
    LauncherRecord {
        ts_ms: 1,
        level: "info".into(),
        target,
        fields,
    }
}

#[test]
fn one_event_waits_for_the_coalesce_window() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 1), 0, t0);

    assert!(!queue.due(t0 + Duration::from_millis(1900), None));
    assert!(queue.due(t0 + Duration::from_secs(2), None));
    assert_eq!(queue.next_wake(t0, None), Some(Duration::from_secs(2)));
}

#[test]
fn a_send_is_spaced_seven_seconds_after_the_last() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 1), 0, t0);
    assert_eq!(queue.take(t0 + Duration::from_secs(2)).len(), 1);

    queue.push(events(2, 1), 0, t0 + Duration::from_secs(3));
    // The spacing ends at t0 + 9 s, so the event waits even though its window ended at t0 + 5 s.
    assert!(!queue.due(t0 + Duration::from_secs(5), None));
    assert_eq!(
        queue.next_wake(t0 + Duration::from_secs(3), None),
        Some(Duration::from_secs(6))
    );
    assert!(queue.due(t0 + Duration::from_secs(9), None));
}

#[test]
fn a_full_group_is_due_at_once_and_a_take_is_capped() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 64), 0, t0);
    assert!(queue.due(t0, None), "64 events fill a group");

    queue.push(events(65, 186), 0, t0);
    assert_eq!(queue.len(), 250);
    let sent = queue.take(t0);
    assert_eq!(sent.len(), SEND_MAX, "one take returns at most one chunk");
    assert_eq!(queue.len(), 250 - SEND_MAX);
}

#[test]
fn a_hold_blocks_even_an_urgent_queue() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 1), 0, t0);
    queue.mark_urgent();

    let hold = Some(Duration::from_secs(30));
    assert!(!queue.due(t0, hold));
    assert_eq!(queue.next_wake(t0, hold), Some(Duration::from_secs(30)));
}

#[test]
fn urgent_is_due_now_with_no_wait() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 1), 0, t0);
    queue.mark_urgent();

    assert!(queue.due(t0, None));
    assert_eq!(queue.next_wake(t0, None), Some(Duration::ZERO));
}

#[test]
fn the_cap_drops_the_oldest_and_counts_them() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    for seq in 1..=(QUEUE_CAP as u64 + 10) {
        queue.push(events(seq, 1), 0, t0);
    }

    assert_eq!(queue.len(), QUEUE_CAP);
    assert_eq!(queue.take_lost(), 10);
    assert_eq!(queue.take_lost(), 0, "the count is zeroed after it is read");
    let front = queue.take(t0);
    assert_eq!(
        seq_of(&front[0]),
        11,
        "the first 10 were dropped, so 11 is the oldest"
    );
}

#[test]
fn give_back_goes_ahead_of_newer_events_and_drops_the_oldest_past_the_cap() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 3), 0, t0);
    let first = queue.take(t0);
    queue.push(events(4, 1), 0, t0 + Duration::from_secs(1));

    queue.give_back(first, t0 + Duration::from_secs(2));
    let order: Vec<u64> = queue
        .take(t0 + Duration::from_secs(2))
        .iter()
        .map(seq_of)
        .collect();
    assert_eq!(order, vec![1, 2, 3, 4]);

    let mut full = LauncherQueue::new();
    full.push(events(100, QUEUE_CAP as u64), 0, t0);
    full.give_back(events(1, 3), t0);
    assert_eq!(full.len(), QUEUE_CAP);
    assert_eq!(
        full.take_lost(),
        3,
        "the three given back are the oldest and drop first"
    );
    assert_eq!(seq_of(&full.take(t0)[0]), 100);
}

#[test]
fn a_new_epoch_clears_unsent_events_without_counting_them_as_lost() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 3), 1, t0);
    queue.push(events(4, 1), 2, t0);

    assert_eq!(queue.len(), 1);
    assert_eq!(queue.epoch(), Some(2));
    assert_eq!(queue.take_lost(), 0);
    assert_eq!(seq_of(&queue.take(t0)[0]), 4);
}

#[test]
fn withdraw_returns_the_count_and_clears_the_epoch_and_urgency() {
    let t0 = Instant::now();
    let mut queue = LauncherQueue::new();
    queue.push(events(1, 2), 5, t0);
    queue.mark_urgent();

    assert_eq!(queue.withdraw(), 2);
    assert!(queue.is_empty());
    assert_eq!(queue.epoch(), None);

    queue.push(events(3, 1), 5, t0);
    assert!(
        !queue.due(t0, None),
        "urgency did not survive the withdrawal"
    );
}

#[test]
fn only_the_sentinel_and_launch_finished_are_flush_triggers() {
    assert!(is_flush_trigger(&record(FLUSH_TARGET, None)));
    assert!(is_flush_trigger(&record(
        "client.launcher.launch",
        Some("launch_finished")
    )));
    assert!(!is_flush_trigger(&record(
        "client.launcher.launch",
        Some("launch_started")
    )));
    assert!(!is_flush_trigger(&record(
        "client.launcher.install",
        Some("launch_finished")
    )));
}
