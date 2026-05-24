//! Pluggable clock abstraction for `Channel` time-keeping.
//!
//! `Channel` reads the current time at a handful of points (last-sent
//! bookkeeping, RTO sampling, keepalive scheduling, inactivity reaping).
//! In production those reads go straight to `Instant::now()`; in tests
//! we want to advance time deterministically without `tokio::time` shims
//! or `std::thread::sleep` calls that would make the suite slow and
//! flaky.
//!
//! The trait is intentionally minimal — one method, one return type.
//! The trait object lives in `Arc<dyn Clock>` on each `Channel` so the
//! same channel can outlive the constructor it was built in and remain
//! injectable for tests.

use std::sync::Arc;
use std::time::Instant;

/// Read-only monotonic clock used by `Channel`.
pub trait Clock: Send + Sync {
    /// Current monotonic time. Must be non-decreasing across calls on
    /// the same `Clock` instance.
    fn now(&self) -> Instant;
}

/// Production clock that reads `std::time::Instant::now()`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Convenience constructor for the default production clock.
pub fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}
