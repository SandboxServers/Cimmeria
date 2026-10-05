//! The world-load grace: whether a main-thread stall that tripped one of
//! the watchdog's kill rules is a load (wait) or a hang (kill).
//!
//! Every bridge call, the heartbeat included, is answered in the client's
//! main-thread Tick drain. A UE3 map load blocks that thread for its whole
//! duration, so to the watchdog a load looks exactly like a hang: each poll
//! times out, and five in a row (or a Tick counter that stops advancing for
//! [`super::HEARTBEAT_STALE_AFTER`]) used to terminate the client. Loads of
//! Ihpet_Crater_Light (world 1300) took 19-42 s on a busy machine and were
//! killed mid-load.
//!
//! Nothing can be asked of the client while its main thread is blocked, so
//! the signal has to be observable from outside it: the main thread's own
//! CPU time ([`super::main_thread`]). A loading thread is deserialising
//! packages and keeps burning CPU; a deadlocked thread or one parked in a
//! crash dialog burns none. While the stalled main thread is busy, the kill
//! waits up to the load grace ([`LOAD_GRACE`], or
//! [`LOAD_GRACE_ENV`] in seconds); an idle stall, or one longer than the
//! grace, is killed as before.
//!
//! Pure and injectable-clock, like [`super::heartbeat`].

use std::time::Duration;

/// Longest main-thread stall tolerated while the main thread is busy (a
/// world load). Covers the 19-42 s loads seen on a loaded machine, plus the
/// hitch after `mapLoaded`, with room to spare.
pub const LOAD_GRACE: Duration = Duration::from_secs(120);

/// Overrides [`LOAD_GRACE`], in whole seconds. `0` turns the grace off.
pub const LOAD_GRACE_ENV: &str = "CIMMERIA_LAB_LOAD_GRACE_SECS";

/// Main-thread CPU, in thousandths of the wall time between two samples,
/// at or above which the stalled thread counts as busy. 20 = 2 %: over the
/// ~6 s between two failed polls that is 120 ms of CPU, far below what a
/// map load spends and far above what a blocked thread spends.
pub const BUSY_CPU_PERMILLE: i64 = 20;

/// The load grace from the raw [`LOAD_GRACE_ENV`] value. Unset, empty or
/// unparseable falls back to [`LOAD_GRACE`].
pub fn load_grace_from(raw: Option<&str>) -> Duration {
    raw.map(str::trim)
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(LOAD_GRACE)
}

/// Whether the main thread did real work between two samples:
/// `cpu_ms` of its CPU time over `wall_ms` of wall clock.
pub fn main_thread_busy(cpu_ms: i64, wall_ms: i64) -> bool {
    wall_ms > 0 && cpu_ms * 1000 >= wall_ms * BUSY_CPU_PERMILLE
}

/// One watchdog poll, as the kill rules see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// The heartbeat answered. `stale`: the Tick counter has not advanced
    /// for [`super::HEARTBEAT_STALE_AFTER`].
    Answered { stale: bool },
    /// The heartbeat failed; `fails` consecutive failures so far.
    Failed { fails: u32 },
}

/// What the watchdog does after a poll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// No kill rule tripped.
    Continue,
    /// A kill rule tripped, but the stalled main thread is busy and the
    /// stall is within the load grace: wait.
    Grace { stalled_ms: i64 },
    /// Terminate. `loading`: the main thread was busy, so this is a load
    /// that outlasted the grace rather than an idle hang.
    Kill { loading: bool, stalled_ms: i64 },
}

/// Decide a poll. `busy` is [`main_thread_busy`] over the interval since
/// the previous poll; `stalled_ms` is how long the Tick counter has not
/// advanced; `max_fails` is the consecutive-failure kill threshold.
pub fn decide(poll: Poll, max_fails: u32, busy: bool, stalled_ms: i64, grace: Duration) -> Verdict {
    let tripped = match poll {
        Poll::Answered { stale } => stale,
        Poll::Failed { fails } => fails >= max_fails,
    };
    if !tripped {
        return Verdict::Continue;
    }
    if busy && stalled_ms < grace.as_millis() as i64 {
        return Verdict::Grace { stalled_ms };
    }
    Verdict::Kill {
        loading: busy,
        stalled_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAILS: u32 = 5;

    /// Live regression (2026-10-05): a 19-42 s load of world 1300 failed
    /// five heartbeats in a row and was terminated mid-load. A busy main
    /// thread inside the grace waits.
    #[test]
    fn a_load_stall_within_the_grace_is_not_killed() {
        let v = decide(Poll::Failed { fails: 5 }, FAILS, true, 30_000, LOAD_GRACE);
        assert_eq!(v, Verdict::Grace { stalled_ms: 30_000 });
        let v = decide(Poll::Failed { fails: 19 }, FAILS, true, 119_999, LOAD_GRACE);
        assert_eq!(
            v,
            Verdict::Grace {
                stalled_ms: 119_999
            }
        );
    }

    /// The grace is a cap: a busy stall that outlasts it is killed, and
    /// the kill says it was loading.
    #[test]
    fn a_load_stall_beyond_the_cap_is_killed() {
        let v = decide(Poll::Failed { fails: 20 }, FAILS, true, 120_000, LOAD_GRACE);
        assert_eq!(
            v,
            Verdict::Kill {
                loading: true,
                stalled_ms: 120_000
            }
        );
    }

    /// An idle stall (a deadlock or a crash dialog) dies at five failures,
    /// as before, however short the stall; four is still a wait.
    #[test]
    fn an_idle_stall_still_dies_at_five_failures() {
        assert_eq!(
            decide(Poll::Failed { fails: 4 }, FAILS, false, 24_000, LOAD_GRACE),
            Verdict::Continue
        );
        assert_eq!(
            decide(Poll::Failed { fails: 5 }, FAILS, false, 30_000, LOAD_GRACE),
            Verdict::Kill {
                loading: false,
                stalled_ms: 30_000
            }
        );
    }

    /// The stale-Tick rule gets the same grace: answered polls with a
    /// stopped counter wait while busy, die when idle or past the cap.
    #[test]
    fn the_stale_tick_rule_respects_the_grace() {
        let stale = Poll::Answered { stale: true };
        assert_eq!(
            decide(stale, FAILS, true, 9_000, LOAD_GRACE),
            Verdict::Grace { stalled_ms: 9_000 }
        );
        assert_eq!(
            decide(stale, FAILS, false, 9_000, LOAD_GRACE),
            Verdict::Kill {
                loading: false,
                stalled_ms: 9_000
            }
        );
        assert_eq!(
            decide(stale, FAILS, true, 121_000, LOAD_GRACE),
            Verdict::Kill {
                loading: true,
                stalled_ms: 121_000
            }
        );
        assert_eq!(
            decide(Poll::Answered { stale: false }, FAILS, false, 0, LOAD_GRACE),
            Verdict::Continue
        );
    }

    /// A grace of zero (`CIMMERIA_LAB_LOAD_GRACE_SECS=0`) restores the old
    /// behaviour: a busy stall dies at the trip.
    #[test]
    fn a_zero_grace_turns_the_grace_off() {
        assert_eq!(
            decide(
                Poll::Failed { fails: 5 },
                FAILS,
                true,
                30_000,
                Duration::ZERO
            ),
            Verdict::Kill {
                loading: true,
                stalled_ms: 30_000
            }
        );
    }

    #[test]
    fn the_env_override_parses_seconds() {
        assert_eq!(load_grace_from(None), LOAD_GRACE);
        assert_eq!(load_grace_from(Some("")), LOAD_GRACE);
        assert_eq!(load_grace_from(Some("abc")), LOAD_GRACE);
        assert_eq!(load_grace_from(Some(" 300 ")), Duration::from_secs(300));
        assert_eq!(load_grace_from(Some("0")), Duration::ZERO);
    }

    /// 2 % of the interval is busy; a blocked thread's few ms are not.
    #[test]
    fn busy_is_a_share_of_the_interval() {
        assert!(main_thread_busy(120, 6_000));
        assert!(main_thread_busy(4_000, 6_000));
        assert!(!main_thread_busy(119, 6_000));
        assert!(!main_thread_busy(0, 6_000));
        assert!(!main_thread_busy(50, 0), "no interval, no verdict");
    }
}
