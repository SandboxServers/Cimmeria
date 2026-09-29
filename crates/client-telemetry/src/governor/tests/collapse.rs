//! Duplicate collapse: one row plus a repeat count, per target.

use serde_json::json;

use super::{admit, ev, governor};
use crate::governor::ExternalDrops;

const T: &str = "client.ui.cegui_log";

#[test]
fn identical_consecutive_events_become_one_row_and_a_repeat_event() {
    let mut g = governor();
    let line = [("text", json!("Could not find image 'X'"))];
    let first = admit(&mut g, ev(T, "info", 100, &line));
    assert_eq!(first.len(), 1, "the first of a run is forwarded");
    for ts in 101..=150 {
        assert!(admit(&mut g, ev(T, "info", ts, &line)).is_empty());
    }
    // A different event on the target ends the run.
    let out = admit(&mut g, ev(T, "info", 200, &[("text", json!("other"))]));
    assert_eq!(out.len(), 2);
    let rep = &out[0];
    assert_eq!(rep.target, T);
    assert_eq!(rep.fields["text"], json!("Could not find image 'X'"));
    assert_eq!(rep.fields["repeat_count"], json!(50));
    assert_eq!(rep.fields["repeat_first_ts_ms"], json!(101));
    assert_eq!(rep.fields["repeat_last_ts_ms"], json!(150));
    assert_eq!(out[1].fields["text"], json!("other"));
    assert_eq!(g.stats().collapsed, 50);
}

/// Consecutive means "previous on the same target": another target's
/// events in between do not break the run.
#[test]
fn another_target_in_between_does_not_break_a_run() {
    let mut g = governor();
    let line = [("text", json!("same"))];
    admit(&mut g, ev(T, "info", 0, &line));
    admit(
        &mut g,
        ev("client.anim.notify", "info", 1, &[("n", json!(1))]),
    );
    assert!(admit(&mut g, ev(T, "info", 2, &line)).is_empty());
}

#[test]
fn a_different_field_or_level_is_not_a_duplicate() {
    let mut g = governor();
    admit(&mut g, ev(T, "info", 0, &[("text", json!("a"))]));
    assert_eq!(
        admit(&mut g, ev(T, "info", 1, &[("text", json!("b"))])).len(),
        1
    );
    assert_eq!(
        admit(&mut g, ev(T, "debug", 2, &[("text", json!("b"))])).len(),
        1
    );
}

/// The window flush reports a steady repeat without ending it: one repeat
/// event per window, and the next identical event is still held.
#[test]
fn the_window_reports_an_open_run_and_keeps_it() {
    let mut g = governor();
    let line = [("text", json!("spam"))];
    admit(&mut g, ev(T, "info", 0, &line));
    for ts in 1..=9 {
        admit(&mut g, ev(T, "info", ts, &line));
    }
    let mut out = Vec::new();
    g.tick(10_000, ExternalDrops::default(), &mut out);
    let rep: Vec<_> = out.iter().filter(|e| e.target == T).collect();
    assert_eq!(rep.len(), 1);
    assert_eq!(rep[0].fields["repeat_count"], json!(9));
    assert!(admit(&mut g, ev(T, "info", 10_001, &line)).is_empty());
}

/// Must-keep events are not collapsed: every warn is its own row.
#[test]
fn must_keep_events_are_never_collapsed() {
    let mut g = governor();
    for ts in 0..20 {
        let out = admit(&mut g, ev(T, "warn", ts, &[("text", json!("same"))]));
        assert_eq!(out.len(), 1);
    }
}
