//! The refusal throttle (negative-logging Pattern D): the burst, the
//! independence and the global ceiling.

use std::time::Instant;

use crate::routes::telemetry::refusal_log::{Decision, RefusalLog, GLOBAL_ROWS_PER_WINDOW, WINDOW};

/// The burst: repeats of one refusal inside the window write nothing, and
/// the next row carries their count.
#[test]
fn repeated_refusals_are_throttled_with_a_count() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    assert_eq!(
        log.decide("busy", "sess-a", t0),
        Decision::Emit { suppressed: 0 }
    );
    for _ in 0..4 {
        assert_eq!(log.decide("busy", "sess-a", t0), Decision::Suppress);
    }
    assert_eq!(
        log.decide("busy", "sess-a", t0 + WINDOW),
        Decision::Emit { suppressed: 4 }
    );
}

/// Independence: another uploader's first refusal, or another reason for
/// the same uploader, is not swallowed by an open window.
#[test]
fn one_uploaders_refusals_do_not_hide_anothers() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    log.decide("busy", "sess-a", t0);
    assert_eq!(
        log.decide("busy", "sess-b", t0),
        Decision::Emit { suppressed: 0 }
    );
    assert_eq!(
        log.decide("rate_limited", "sess-a", t0),
        Decision::Emit { suppressed: 0 }
    );
}

/// Many distinct uploaders cannot flood the log: past the global ceiling,
/// refusals are counted, not written.
#[test]
fn the_refusal_log_has_a_global_ceiling() {
    let log = RefusalLog::new();
    let t0 = Instant::now();
    let emitted = (0..GLOBAL_ROWS_PER_WINDOW + 20)
        .filter(|i| {
            matches!(
                log.decide("missing_token", &format!("198.51.100.{i}"), t0),
                Decision::Emit { .. }
            )
        })
        .count();
    assert_eq!(emitted, GLOBAL_ROWS_PER_WINDOW as usize);
}
