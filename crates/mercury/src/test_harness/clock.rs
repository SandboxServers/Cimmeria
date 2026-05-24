//! Test clock — deterministic time control for the loopback harness.
//!
//! Each [`LoopbackPeer`](super::LoopbackPeer) owns its own `Arc<TestClock>`.
//! `Channel` reads the current time through its injected
//! [`Clock`](crate::clock::Clock) trait, so advancing the test clock
//! deterministically drives keepalive / RTO / inactivity machinery without
//! `std::thread::sleep` or `tokio::time::sleep` calls.
//!
//! Per-peer ownership is deliberate: there is no process-global state, so
//! `cargo nextest` parallel runs cannot perturb each other's clocks.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::clock::Clock;

/// Pluggable monotonic clock under explicit test control.
///
/// `now()` returns `base + offset`. `advance` extends `offset`; `freeze`
/// pins the current return value until `resume`.
#[derive(Debug)]
pub struct TestClock {
    base: Instant,
    state: Mutex<State>,
}

#[derive(Debug, Clone, Copy)]
struct State {
    offset: Duration,
    frozen_at: Option<Duration>,
}

impl TestClock {
    /// Construct a clock anchored at the current `Instant`. Subsequent
    /// `now()` reads return that anchor until [`Self::advance`] moves the
    /// offset forward.
    pub fn new() -> Self {
        Self {
            base: Instant::now(),
            state: Mutex::new(State {
                offset: Duration::ZERO,
                frozen_at: None,
            }),
        }
    }

    /// Advance the clock by `by`. Subsequent `now()` calls return the
    /// advanced time. Has no effect on `tokio::time` — only the
    /// `Channel`'s `Clock`-mediated reads see the jump.
    pub fn advance(&self, by: Duration) {
        let mut state = self.state.lock().expect("TestClock state poisoned");
        state.offset += by;
    }

    /// Stop the clock — subsequent `now()` reads return the same value
    /// until [`Self::resume`]. Useful for ordering assertions where the
    /// test author needs two reads to compare equal.
    pub fn freeze(&self) {
        let mut state = self.state.lock().expect("TestClock state poisoned");
        if state.frozen_at.is_none() {
            state.frozen_at = Some(state.offset);
        }
    }

    /// Resume the clock at its frozen offset. Subsequent `now()` reads
    /// reflect `advance` calls again.
    pub fn resume(&self) {
        let mut state = self.state.lock().expect("TestClock state poisoned");
        state.frozen_at = None;
    }

    /// Current offset from the construction anchor — useful for sanity
    /// asserts in tests ("did we actually advance?").
    pub fn offset(&self) -> Duration {
        self.state.lock().expect("TestClock state poisoned").offset
    }
}

impl Default for TestClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        let state = self.state.lock().expect("TestClock state poisoned");
        let offset = state.frozen_at.unwrap_or(state.offset);
        self.base + offset
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn now_advances_with_advance_calls() {
        let clock = TestClock::new();
        let t0 = clock.now();
        clock.advance(Duration::from_millis(500));
        let t1 = clock.now();
        assert_eq!(t1.duration_since(t0), Duration::from_millis(500));
    }

    #[test]
    fn freeze_pins_now_until_resume() {
        let clock = TestClock::new();
        clock.freeze();
        let t0 = clock.now();
        clock.advance(Duration::from_millis(200));
        let t1 = clock.now();
        assert_eq!(t0, t1, "frozen clock must not advance");
        clock.resume();
        let t2 = clock.now();
        assert_eq!(
            t2.duration_since(t0),
            Duration::from_millis(200),
            "resumed clock reflects all advances that happened while frozen"
        );
    }

    #[test]
    fn usable_as_dyn_clock_behind_arc() {
        let clock: Arc<dyn Clock> = Arc::new(TestClock::new());
        let _ = clock.now();
    }
}
