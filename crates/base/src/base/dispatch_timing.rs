//! Timing for the cell-to-base dispatch loop (TG-DB-05).
//!
//! The loop in `service.rs` times every message the cell sends, around
//! `route_cell_message`. A dispatch that takes [`SLOW_DISPATCH`] or longer
//! writes one WARN. The WARN is throttled per `msg_kind` (Pattern D), so a
//! handler that stays slow writes one row per window, not one per message.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A dispatch at or over this long is logged.
pub(super) const SLOW_DISPATCH: Duration = Duration::from_millis(250);

/// Window per `msg_kind` for the slow-dispatch WARN.
const SLOW_WINDOW: Duration = Duration::from_secs(10);

/// Fixed message for the slow-dispatch WARN. The details are fields.
const SLOW_MESSAGE: &str = "cell-to-base dispatch was slow";

/// Pattern D state for the slow-dispatch WARN: per `msg_kind`, the time of
/// the last row and the number of rows swallowed since.
#[derive(Default)]
pub(super) struct SlowDispatchThrottle {
    keys: HashMap<&'static str, (Instant, u64)>,
}

impl SlowDispatchThrottle {
    /// `Some(suppressed)` when a row may be written for `kind` at `now`, where
    /// `suppressed` counts the rows swallowed since the last one. `None` while
    /// the window for `kind` is still open.
    fn admit(&mut self, kind: &'static str, now: Instant) -> Option<u64> {
        match self.keys.get_mut(kind) {
            None => {
                self.keys.insert(kind, (now, 0));
                Some(0)
            }
            Some((last, swallowed)) => {
                if now.saturating_duration_since(*last) >= SLOW_WINDOW {
                    let suppressed = *swallowed;
                    *last = now;
                    *swallowed = 0;
                    Some(suppressed)
                } else {
                    *swallowed += 1;
                    None
                }
            }
        }
    }
}

/// Record one finished dispatch of `kind` that took `elapsed` with
/// `queue_depth` messages still waiting. Writes the slow-dispatch WARN when
/// `elapsed` reaches [`SLOW_DISPATCH`] and the window for `kind` is open.
pub(super) fn note_dispatch(
    throttle: &mut SlowDispatchThrottle,
    kind: &'static str,
    elapsed: Duration,
    queue_depth: usize,
    now: Instant,
) {
    if elapsed < SLOW_DISPATCH {
        return;
    }
    if let Some(suppressed) = throttle.admit(kind, now) {
        tracing::warn!(
            event = "cell_dispatch_slow",
            msg_kind = kind,
            elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            queue_depth = queue_depth,
            suppressed = suppressed,
            "{}",
            SLOW_MESSAGE
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// Count the slow-dispatch WARNs captured for `kind`.
    fn slow_rows(capture: &crate::test_support::LogCaptureGuard, kind: &str) -> Vec<String> {
        capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN
                    && c.has_field("event", "cell_dispatch_slow")
                    && c.has_field("msg_kind", kind)
            })
            .map(|c| c.fields.get("suppressed").cloned().unwrap_or_default())
            .collect()
    }

    /// A burst of slow dispatches of one kind writes one row at once, then
    /// one row per window that carries how many were swallowed.
    #[test]
    fn slow_dispatch_burst_writes_one_row_per_window() {
        let capture = LogCapture::install();
        let mut throttle = SlowDispatchThrottle::default();
        let slow = SLOW_DISPATCH * 2;
        let t0 = Instant::now();

        for _ in 0..50 {
            note_dispatch(&mut throttle, "entered_aoi", slow, 3, t0);
        }
        assert_eq!(slow_rows(&capture, "entered_aoi"), vec!["0"]);

        note_dispatch(&mut throttle, "entered_aoi", slow, 3, t0 + SLOW_WINDOW);
        assert_eq!(slow_rows(&capture, "entered_aoi"), vec!["0", "49"]);
    }

    /// A second kind's first slow dispatch is not swallowed by the first
    /// kind's open window, and a fast dispatch never writes a row.
    #[test]
    fn slow_dispatch_throttle_is_per_kind() {
        let capture = LogCapture::install();
        let mut throttle = SlowDispatchThrottle::default();
        let t0 = Instant::now();

        note_dispatch(&mut throttle, "entered_aoi", SLOW_DISPATCH, 0, t0);
        note_dispatch(&mut throttle, "entered_aoi", SLOW_DISPATCH, 0, t0);
        note_dispatch(&mut throttle, "left_aoi", SLOW_DISPATCH, 7, t0);
        assert_eq!(slow_rows(&capture, "entered_aoi"), vec!["0"]);
        assert_eq!(slow_rows(&capture, "left_aoi"), vec!["0"]);

        note_dispatch(
            &mut throttle,
            "grant_xp",
            SLOW_DISPATCH - Duration::from_millis(1),
            0,
            t0,
        );
        assert!(slow_rows(&capture, "grant_xp").is_empty());
    }
}
