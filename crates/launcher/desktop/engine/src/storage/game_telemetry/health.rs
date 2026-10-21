//! Launcher telemetry pipeline health (launcher-telemetry C7). Counts how each
//! send ended and reports the recovery after a run of failures. It keeps counts
//! and codes only: no message, path or field value is ever stored here.
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
    pub outage_ms: u64,
}

/// The process-wide handle type the forwarder records into.
pub type SharedHealth = Arc<Mutex<TelemetryHealth>>;

impl TelemetryHealth {
    /// `Empty` changes nothing. `Sent` after one or more failures returns
    /// `Some(Recovery)` and resets `since_success` and `first_failure_ms`.
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
        let first = self.first_failure_ms.take();
        if failures == FailureCounts::default() {
            return None;
        }
        Some(Recovery {
            failures,
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
}
