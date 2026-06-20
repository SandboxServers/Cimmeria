//! Atomic sender counters and the public [`SenderStats`] snapshot.

use std::sync::atomic::{AtomicU64, Ordering};

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
