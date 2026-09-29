//! The rate limit every log sink applies, one bucket per distinct message.
//!
//! A sink sees a stream where most lines are noise and a few are the
//! finding: the client's lock trace is 65 000 lines a session, the
//! "Error opening static cache archive" line appears twice. A fixed 1-in-N
//! sample would lose the rare line; a bucket per distinct message (see
//! [`super::text::message_shape`] for what "distinct" means) keeps every
//! first occurrence and holds a repeating one to a few a second, carrying
//! how many it swallowed on the next line that gets through. The mechanics
//! are [`NameThrottle`]'s; this wraps one behind a lock with the process
//! clock and the [`capture`](crate::capture) limits.
//!
//! The lock is held for a hash lookup only. A sink detour can run on any
//! game thread, so the wrapper never blocks on anything else.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Monotonic origin for the sinks' throttle clock.
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Milliseconds since the first sink event.
pub fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// One sink's rate limit.
pub struct SinkThrottle {
    inner: Mutex<Option<NameThrottle>>,
}

impl SinkThrottle {
    /// An empty limit; the table is built on first use, with the limits in
    /// force then.
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Decide for one message with throttle key `key`.
    ///
    /// A poisoned lock is ignored: a panic in some other detour must not
    /// silence this sink.
    pub fn check(&self, key: &str) -> Decision {
        self.check_at(key, now_ms())
    }

    /// [`check`](Self::check) at an explicit time, for tests.
    pub fn check_at(&self, key: &str, now_ms: u64) -> Decision {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .get_or_insert_with(|| {
                let (burst, rate) = crate::capture::sink_limits();
                NameThrottle::with_limits(burst, rate)
            })
            .check(key, now_ms)
    }
}

impl Default for SinkThrottle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::name_throttle::{BURST, RATE_PER_SEC};

    /// The defaults are the event-registry hook's, so one mental model
    /// covers every stream.
    #[test]
    fn the_default_limits_are_the_registry_hooks() {
        assert_eq!(
            crate::capture::DEFAULT_LIMITS,
            (BURST, RATE_PER_SEC),
            "capture's defaults and NameThrottle's constants must agree"
        );
    }

    #[test]
    fn a_rare_message_gets_through_behind_a_hot_one() {
        let t = SinkThrottle::new();
        let mut hot = 0;
        for i in 0..2000u64 {
            if matches!(t.check_at("hot", i), Decision::Emit { .. }) {
                hot += 1;
            }
        }
        // Two seconds of a 1 kHz message: the burst plus about two seconds
        // of refills, nowhere near 2000.
        assert!(hot <= (BURST + 2 * RATE_PER_SEC + 1) as usize, "{hot}");
        assert_eq!(t.check_at("rare", 2000), Decision::Emit { suppressed: 0 });
    }

    #[test]
    fn the_suppressed_count_rides_the_next_emit() {
        let t = SinkThrottle::new();
        for _ in 0..BURST {
            t.check_at("m", 0);
        }
        for _ in 0..3 {
            assert_eq!(t.check_at("m", 0), Decision::Suppress);
        }
        let refill = 1000 / u64::from(RATE_PER_SEC);
        assert_eq!(t.check_at("m", refill), Decision::Emit { suppressed: 3 });
    }
}
