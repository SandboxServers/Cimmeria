//! Heartbeat staleness detection.
//!
//! The bridge exposes a Tick-drain counter (`heartbeat` JSON-RPC
//! method, bumped once per `FEngineLoop::Tick`). The supervisor polls
//! it; a counter that stops advancing means the client's main thread is
//! wedged — a modal load, a crash dialog, or a hang — and the process
//! must be terminated so it can be relaunched (ADR §6).
//!
//! This module is the pure decision logic, injectable-clock so it is
//! unit-testable off-process. The polling + the actual terminate live
//! in [`super::process`]; the watchdog only decides.

use std::time::Duration;

/// What the watchdog concluded from the latest observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatState {
    /// The counter is advancing, or hasn't been stalled long enough to
    /// call it dead yet.
    Alive,
    /// The counter has not advanced for at least `stale_after`. The
    /// caller should terminate the process.
    Stale,
}

/// Tracks heartbeat advancement against a staleness threshold.
///
/// Feed it `(counter, now_ms)` on every poll. It remembers the last
/// value and the wall-clock time it last *advanced*; when the value
/// sits still past the threshold it reports [`HeartbeatState::Stale`].
///
/// A *failed* poll (the bridge call timed out or the socket is dead) is
/// itself a strong hung/crash signal and is handled by the caller —
/// this type only reasons about successful reads of the counter.
#[derive(Debug, Clone)]
pub struct HeartbeatWatchdog {
    stale_after_ms: i64,
    last_count: Option<u64>,
    last_advance_ms: i64,
}

impl HeartbeatWatchdog {
    pub fn new(stale_after: Duration) -> Self {
        Self {
            stale_after_ms: stale_after.as_millis() as i64,
            last_count: None,
            last_advance_ms: 0,
        }
    }

    /// Observe a heartbeat reading. Returns the current liveness verdict.
    pub fn observe(&mut self, count: u64, now_ms: i64) -> HeartbeatState {
        match self.last_count {
            None => {
                // First reading — nothing to compare against yet.
                self.last_count = Some(count);
                self.last_advance_ms = now_ms;
                HeartbeatState::Alive
            }
            Some(prev) if count > prev => {
                // Advanced: the game is ticking.
                self.last_count = Some(count);
                self.last_advance_ms = now_ms;
                HeartbeatState::Alive
            }
            Some(_) => {
                // No advance. Wedged iff it's been still long enough.
                if now_ms - self.last_advance_ms >= self.stale_after_ms {
                    HeartbeatState::Stale
                } else {
                    HeartbeatState::Alive
                }
            }
        }
    }

    /// Milliseconds since the counter last advanced, given `now_ms`.
    /// Reported in `lab_client_status` as the heartbeat age. Zero before
    /// the first observation.
    pub fn age_ms(&self, now_ms: i64) -> i64 {
        if self.last_count.is_none() {
            0
        } else {
            (now_ms - self.last_advance_ms).max(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wd() -> HeartbeatWatchdog {
        HeartbeatWatchdog::new(Duration::from_secs(5))
    }

    /// Advancing counter stays Alive, and the age resets on each advance.
    #[test]
    fn advancing_counter_is_alive() {
        let mut w = wd();
        assert_eq!(w.observe(10, 1_000), HeartbeatState::Alive); // first read
        assert_eq!(w.observe(11, 2_000), HeartbeatState::Alive);
        assert_eq!(w.observe(50, 9_000), HeartbeatState::Alive);
        assert_eq!(w.age_ms(9_100), 100, "age is measured from last advance");
    }

    /// A stalled counter trips Stale only after the threshold elapses.
    #[test]
    fn stalled_counter_goes_stale_after_threshold() {
        let mut w = wd();
        assert_eq!(w.observe(100, 0), HeartbeatState::Alive);
        assert_eq!(w.observe(101, 1_000), HeartbeatState::Alive);
        // Now it sticks at 101.
        assert_eq!(w.observe(101, 3_000), HeartbeatState::Alive); // 2s < 5s
        assert_eq!(w.observe(101, 5_500), HeartbeatState::Alive); // 4.5s < 5s
        assert_eq!(
            w.observe(101, 6_000),
            HeartbeatState::Stale,
            "5s of no advance ⇒ terminate"
        );
    }

    /// A resume after a near-stall (counter advances again) clears the
    /// pending staleness — a slow frame is not a crash.
    #[test]
    fn advance_after_near_stall_recovers() {
        let mut w = wd();
        w.observe(0, 0);
        w.observe(0, 4_000); // stalled 4s, still under 5s
        assert_eq!(w.observe(1, 4_500), HeartbeatState::Alive);
        // The clock resets: another 4s stall is fine again.
        assert_eq!(w.observe(1, 8_000), HeartbeatState::Alive);
    }
}
