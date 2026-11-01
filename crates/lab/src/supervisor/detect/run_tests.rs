//! Tests of [`run_all`] and of the `lab_detectors_read` lease policy.

use super::*;

/// A heartbeat history with the same tick twice, 300 ms apart: the client
/// main thread stalled, and no watch is running.
fn stalled_beats() -> Vec<HeartbeatSample> {
    vec![
        HeartbeatSample {
            tick_count: 5,
            observed_ms: 0,
        },
        HeartbeatSample {
            tick_count: 5,
            observed_ms: 300,
        },
    ]
}

#[test]
fn no_watch_runs_two_detectors() {
    let beats = stalled_beats();
    let errors: Vec<CallRow> = (0..3)
        .map(|k| CallRow {
            ms: 1000 * k,
            instance: "default".into(),
            tool: "client_move_to".into(),
            outcome: "error",
        })
        .collect();
    let fired = run_all(&Inputs {
        samples: &[],
        heartbeats: &beats,
        holds: &[],
        calls: &errors,
        now_ms: 5000,
    });
    let names: Vec<&str> = fired.iter().map(|f| f.name).collect();
    assert_eq!(names, ["hitch", "log_error"]);
}

#[test]
fn firings_are_sorted_by_time() {
    let beats = stalled_beats();
    let errors: Vec<CallRow> = (0..3)
        .map(|k| CallRow {
            ms: 10 + k,
            instance: "default".into(),
            tool: "client_move_to".into(),
            outcome: "error",
        })
        .collect();
    let fired = run_all(&Inputs {
        samples: &[],
        heartbeats: &beats,
        holds: &[],
        calls: &errors,
        now_ms: 5000,
    });
    assert!(fired.windows(2).all(|w| w[0].at_ms <= w[1].at_ms));
    assert_eq!(fired.len(), 2);
}

#[test]
fn detectors_tool_is_open() {
    assert!(crate::lease::policy::OPEN.contains(&"lab_detectors_read"));
    assert!(!crate::proof::claims::NO_CLAIM.contains(&"lab_detectors_read"));
}
