//! Launcher telemetry pipeline health (launcher-telemetry C7). Counts how each
//! send ended, how many events were sent or lost, and reports the recovery after
//! a run of failures. It keeps counts and codes only: no message, path or field
//! value is ever stored here.
use super::launcher_session::SendOutcome;
use std::sync::{Arc, Mutex};

/// How many sends ended in each failure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct FailureCounts {
    pub mint_failed: u64,
    pub held: u64,
    pub dropped: u64,
}

/// The pipeline's running health, shared by the forwarder and its readers.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct TelemetryHealth {
    pub sent: u64,
    pub totals: FailureCounts,
    pub since_success: FailureCounts,
    /// Events accepted by a successful send, across all sends.
    pub events_sent: u64,
    /// Events dropped from the queue, across all sends.
    pub events_lost: u64,
    /// Events dropped since the last success.
    pub events_lost_since_success: u64,
    /// `"sent"`, `"held"`, `"mint_failed"` or `"dropped"`.
    pub last_outcome: Option<&'static str>,
    /// The `MintFailed` reason (C8). Kept across later outcomes.
    pub last_reason: Option<&'static str>,
    pub last_success_ms: Option<u64>,
    /// The first failure since the last success.
    pub first_failure_ms: Option<u64>,
}

/// The failures that ended before a successful send, and how long they lasted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recovery {
    pub failures: FailureCounts,
    /// Events lost since the last success.
    pub events_lost: u64,
    pub outage_ms: u64,
}

/// The process-wide handle type the forwarder records into.
pub type SharedHealth = Arc<Mutex<TelemetryHealth>>;

impl TelemetryHealth {
    /// Adds `sent` to `events_sent` and `lost` to both lost counters.
    pub fn record_events(&mut self, sent: u64, lost: u64) {
        self.events_sent += sent;
        self.events_lost += lost;
        self.events_lost_since_success += lost;
    }

    /// `Empty` changes nothing. `Sent` after one or more failures, or after
    /// lost events, returns `Some(Recovery)` and resets `since_success`,
    /// `events_lost_since_success` and `first_failure_ms`.
    pub fn record(&mut self, outcome: &SendOutcome, now_ms: u64) -> Option<Recovery> {
        let label = match *outcome {
            SendOutcome::Empty => return None,
            SendOutcome::Sent => return self.succeeded(now_ms),
            SendOutcome::MintFailed { reason } => {
                self.last_reason = Some(reason);
                self.totals.mint_failed += 1;
                self.since_success.mint_failed += 1;
                "mint_failed"
            }
            SendOutcome::Held => {
                self.totals.held += 1;
                self.since_success.held += 1;
                "held"
            }
            SendOutcome::Dropped => {
                self.totals.dropped += 1;
                self.since_success.dropped += 1;
                "dropped"
            }
        };
        self.last_outcome = Some(label);
        self.first_failure_ms.get_or_insert(now_ms);
        None
    }

    fn succeeded(&mut self, now_ms: u64) -> Option<Recovery> {
        self.sent += 1;
        self.last_outcome = Some("sent");
        self.last_success_ms = Some(now_ms);
        let failures = std::mem::take(&mut self.since_success);
        let events_lost = std::mem::take(&mut self.events_lost_since_success);
        let first = self.first_failure_ms.take();
        if failures == FailureCounts::default() && events_lost == 0 {
            return None;
        }
        Some(Recovery {
            failures,
            events_lost,
            outage_ms: first.map_or(0, |since| now_ms.saturating_sub(since)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINT: SendOutcome = SendOutcome::MintFailed {
        reason: "transport",
    };

    #[test]
    fn an_empty_batch_changes_nothing() {
        let mut health = TelemetryHealth::default();
        assert_eq!(health.record(&SendOutcome::Empty, 5), None);
        assert_eq!(health, TelemetryHealth::default());
    }

    #[test]
    fn a_sent_batch_alone_gives_no_recovery() {
        let mut health = TelemetryHealth::default();
        assert_eq!(health.record(&SendOutcome::Sent, 10), None);
        assert_eq!(health.sent, 1);
        assert_eq!(health.last_outcome, Some("sent"));
        assert_eq!(health.last_success_ms, Some(10));
    }

    #[test]
    fn mint_failed_then_held_then_sent_recovers_once_and_resets() {
        let mut health = TelemetryHealth::default();
        assert_eq!(health.record(&MINT, 100), None);
        assert_eq!(health.record(&SendOutcome::Held, 160), None);
        let recovery = health.record(&SendOutcome::Sent, 400).expect("a recovery");
        assert_eq!(
            recovery.failures,
            FailureCounts {
                mint_failed: 1,
                held: 1,
                dropped: 0
            }
        );
        assert_eq!(recovery.outage_ms, 300);
        assert_eq!(health.since_success, FailureCounts::default());
        assert_eq!(health.first_failure_ms, None);
        assert_eq!(health.record(&SendOutcome::Sent, 500), None);
    }

    #[test]
    fn totals_keep_counting_across_a_recovery() {
        let mut health = TelemetryHealth::default();
        health.record(&MINT, 1);
        health.record(&SendOutcome::Sent, 2);
        health.record(&SendOutcome::Dropped, 3);
        assert_eq!(
            health.totals,
            FailureCounts {
                mint_failed: 1,
                held: 0,
                dropped: 1
            }
        );
        assert_eq!(health.sent, 1);
    }

    #[test]
    fn the_last_mint_reason_survives_later_outcomes() {
        let mut health = TelemetryHealth::default();
        health.record(&MINT, 1);
        health.record(&SendOutcome::Held, 2);
        assert_eq!(health.last_reason, Some("transport"));
        assert_eq!(health.last_outcome, Some("held"));
    }

    #[test]
    fn lost_events_are_counted_and_reported_on_the_next_success() {
        let mut health = TelemetryHealth::default();
        health.record_events(5, 2);
        health.record(&SendOutcome::Dropped, 10);
        health.record_events(3, 0);
        let recovery = health.record(&SendOutcome::Sent, 20).expect("a recovery");
        assert_eq!(recovery.events_lost, 2);
        assert_eq!(health.events_sent, 8);
        assert_eq!(health.events_lost, 2);
        assert_eq!(health.events_lost_since_success, 0);
    }

    #[test]
    fn lost_events_alone_still_give_a_recovery() {
        let mut health = TelemetryHealth::default();
        health.record_events(0, 4);
        let recovery = health.record(&SendOutcome::Sent, 7).expect("a recovery");
        assert_eq!(recovery.events_lost, 4);
        assert_eq!(recovery.failures, FailureCounts::default());
        assert_eq!(recovery.outage_ms, 0);
    }
}
