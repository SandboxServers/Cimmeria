//! Atomic sender counters, the public [`SenderStats`] snapshot, the
//! `discord_events_total{outcome}` counter, and the throttle on the
//! `discord.dropped` WARN.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::event::EventKind;
use crate::router::channel_for;

/// Gap between two `discord.dropped` WARN rows for the same outcome.
pub(super) const DROP_WARN_WINDOW: Duration = Duration::from_secs(60);

cimmeria_observability::metric_label! {
    /// How one Discord event ended: the `outcome` label of
    /// `discord_events_total`.
    pub(super) enum Outcome {
        /// Posted to its webhook.
        Sent => "sent",
        /// Turned off by config or muted, so never queued or posted.
        Filtered => "filtered",
        /// The bounded queue was full when the event arrived.
        DroppedFull => "dropped_full",
        /// The sender task had shut down.
        DroppedClosed => "dropped_closed",
        /// The event's channel has no webhook URL.
        NoWebhook => "no_webhook",
        /// The channel's token bucket was empty.
        RateLimited => "rate_limited",
        /// The POST failed after retries.
        Failed => "failed",
    }
}

impl Outcome {
    /// Count one event that ended as `self` on `discord_events_total`.
    pub(super) fn count(self) {
        cimmeria_observability::counter!("discord_events_total", "outcome" => self.label());
    }
}

/// What the `discord.dropped` WARN last did for one outcome.
#[derive(Debug, Default)]
struct DropWindow {
    last_warn: Option<Instant>,
    suppressed: u64,
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) enqueued: AtomicU64,
    pub(super) sent: AtomicU64,
    pub(super) filtered: AtomicU64,
    pub(super) dropped_full: AtomicU64,
    pub(super) dropped_closed: AtomicU64,
    pub(super) dropped_rate_limit: AtomicU64,
    pub(super) retried: AtomicU64,
    pub(super) rate_limited_429: AtomicU64,
    pub(super) failed: AtomicU64,
    drop_windows: Mutex<HashMap<Outcome, DropWindow>>,
}

impl Stats {
    pub(super) fn snapshot(&self) -> SenderStats {
        SenderStats {
            enqueued: self.enqueued.load(Ordering::Relaxed),
            sent: self.sent.load(Ordering::Relaxed),
            filtered: self.filtered.load(Ordering::Relaxed),
            dropped_full: self.dropped_full.load(Ordering::Relaxed),
            dropped_closed: self.dropped_closed.load(Ordering::Relaxed),
            dropped_rate_limit: self.dropped_rate_limit.load(Ordering::Relaxed),
            retried: self.retried.load(Ordering::Relaxed),
            rate_limited_429: self.rate_limited_429.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
        }
    }

    /// Count a dropped `kind` event on `discord_events_total`, and WARN
    /// `discord.dropped` unless this outcome already warned within
    /// [`DROP_WARN_WINDOW`]. The counter sees every drop; the WARN carries
    /// `suppressed`, the drops it swallowed since the last row.
    pub(super) fn record_drop(&self, outcome: Outcome, kind: EventKind) {
        outcome.count();
        let Some(suppressed) = self.admit_warn(outcome, Instant::now()) else {
            return;
        };
        tracing::warn!(
            target: "cimmeria_discord",
            event = "discord.dropped",
            outcome = outcome.label(),
            channel = channel_for(kind).as_str(),
            discord_event = kind.as_str(),
            suppressed = suppressed,
            "Discord event dropped"
        );
    }

    /// `Some(n)` when a drop WARN fires now, with `n` drops swallowed since
    /// the last one. `None` when this drop is swallowed.
    fn admit_warn(&self, outcome: Outcome, now: Instant) -> Option<u64> {
        let mut windows = self
            .drop_windows
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let window = windows.entry(outcome).or_default();
        match window.last_warn {
            Some(at) if now.duration_since(at) < DROP_WARN_WINDOW => {
                window.suppressed += 1;
                None
            }
            _ => {
                window.last_warn = Some(now);
                Some(std::mem::take(&mut window.suppressed))
            }
        }
    }
}

/// Snapshot of sender stats. Read via `DiscordRuntime::stats()`; intended
/// to be surfaced by an admin-api stats endpoint when one is wired up.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SenderStats {
    pub enqueued: u64,
    pub sent: u64,
    pub filtered: u64,
    pub dropped_full: u64,
    pub dropped_closed: u64,
    pub dropped_rate_limit: u64,
    pub retried: u64,
    pub rate_limited_429: u64,
    pub failed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A burst warns once with `suppressed = 0`, swallows the rest, and the
    /// next row after the window carries the swallowed count.
    #[test]
    fn burst_warns_once_then_reports_what_it_swallowed() {
        let stats = Stats::default();
        let t0 = Instant::now();
        assert_eq!(stats.admit_warn(Outcome::DroppedFull, t0), Some(0));
        for _ in 0..10 {
            assert_eq!(stats.admit_warn(Outcome::DroppedFull, t0), None);
        }
        let later = t0 + DROP_WARN_WINDOW;
        assert_eq!(stats.admit_warn(Outcome::DroppedFull, later), Some(10));
        assert_eq!(stats.admit_warn(Outcome::DroppedFull, later), None);
    }

    /// A second outcome's first row is not swallowed by the first's window.
    #[test]
    fn each_outcome_has_its_own_window() {
        let stats = Stats::default();
        let t0 = Instant::now();
        assert_eq!(stats.admit_warn(Outcome::DroppedFull, t0), Some(0));
        assert_eq!(stats.admit_warn(Outcome::DroppedClosed, t0), Some(0));
        assert_eq!(stats.admit_warn(Outcome::DroppedFull, t0), None);
    }

    /// Every drop reaches the counter, including the ones the WARN swallows.
    /// `no_webhook` is emitted by no other test, so its total is exact.
    #[test]
    fn record_drop_counts_every_drop() {
        cimmeria_observability::testing::install();
        let before = cimmeria_observability::testing::counter_total(
            "discord_events_total",
            &[("outcome", "no_webhook")],
        );
        let stats = Stats::default();
        for _ in 0..5 {
            stats.record_drop(Outcome::NoWebhook, EventKind::PlayerLogin);
        }
        let after = cimmeria_observability::testing::counter_total(
            "discord_events_total",
            &[("outcome", "no_webhook")],
        );
        assert_eq!(after - before, 5);
    }
}
