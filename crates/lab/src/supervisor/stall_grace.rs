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
//! Busy is sticky for the stall ([`StallTracker`]): once one sample inside
//! the stall shows the main thread working, the stall stays in grace until
//! the Tick counter advances or the grace runs out. A load is not CPU-bound
//! end to end (a disk-bound stretch, a wait on the async loader), and a
//! single idle window must not kill it mid-way. A stall that never showed
//! work dies at the old rules, at the old time.
//!
//! Pure and injectable-clock, like [`super::heartbeat`].

use std::time::Duration;

use super::heartbeat::{next_fail_count, BUSY_GRACE_MS};

/// Longest main-thread stall tolerated while the main thread is busy (a
/// world load). Covers the 19-42 s loads seen on a loaded machine, plus the
/// hitch after `mapLoaded`, with room to spare.
pub const LOAD_GRACE: Duration = Duration::from_secs(120);

/// Overrides [`LOAD_GRACE`], in whole seconds. `0` turns the grace off;
/// values above [`MAX_LOAD_GRACE`] are clamped to it.
pub const LOAD_GRACE_ENV: &str = "CIMMERIA_LAB_LOAD_GRACE_SECS";

/// The largest grace [`LOAD_GRACE_ENV`] can set: an hour is already far
/// past any load, and the clamp keeps the millisecond maths in `i64`.
pub const MAX_LOAD_GRACE: Duration = Duration::from_secs(3600);

/// How long after a launch a client that has never answered the bridge
/// is left alone. With several clients booting at once a bridge took
/// about 25 s to come up (2026-10-10), past the 5-failure rule.
pub const BOOT_GRACE: Duration = Duration::from_secs(90);

/// Whether a failed heartbeat should be ignored: the bridge has not
/// answered once since this launch and the launch is younger than
/// `grace`.
pub fn in_boot_grace(answered_once: bool, since_launch: Duration, grace: Duration) -> bool {
    !answered_once && since_launch < grace
}

/// Main-thread CPU, in thousandths of the wall time between two samples,
/// at or above which the stalled thread counts as busy. 20 = 2 %: over the
/// ~6 s between two failed polls that is 120 ms of CPU, far below what a
/// map load spends and far above what a blocked thread spends.
pub const BUSY_CPU_PERMILLE: i64 = 20;

/// The load grace from the raw [`LOAD_GRACE_ENV`] value. Unset, empty or
/// unparseable falls back to [`LOAD_GRACE`]; anything above
/// [`MAX_LOAD_GRACE`] is clamped to it.
pub fn load_grace_from(raw: Option<&str>) -> Duration {
    raw.map(str::trim)
        .and_then(|s| s.parse::<u64>().ok())
        .map(|secs| Duration::from_secs(secs.min(MAX_LOAD_GRACE.as_secs())))
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

/// Decide a poll. `busy`: the main thread has shown work during this stall
/// ([`StallTracker`]); `stalled_ms` is how long the Tick counter has not
/// advanced; `max_fails` is the consecutive-failure kill threshold.
pub fn decide(poll: Poll, max_fails: u32, busy: bool, stalled_ms: i64, grace: Duration) -> Verdict {
    let tripped = match poll {
        Poll::Answered { stale } => stale,
        Poll::Failed { fails } => fails >= max_fails,
    };
    if !tripped {
        return Verdict::Continue;
    }
    let grace_ms = i64::try_from(grace.as_millis()).unwrap_or(i64::MAX);
    if busy && stalled_ms < grace_ms {
        return Verdict::Grace { stalled_ms };
    }
    Verdict::Kill {
        loading: busy,
        stalled_ms,
    }
}

/// The heartbeat poll's raw outcome, as the watchdog loop sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heartbeat {
    /// The heartbeat answered; `stale` as in [`Poll::Answered`].
    Answered { stale: bool },
    /// The heartbeat failed. `ms_since_last_ok`: when any bridge call last
    /// succeeded (a busy bridge forgives the miss, [`BUSY_GRACE_MS`]).
    Failed { ms_since_last_ok: Option<i64> },
}

/// One watchdog poll, decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub verdict: Verdict,
    pub poll: Poll,
    /// This poll's sample showed the main thread working.
    pub busy_now: bool,
    /// Some sample inside the current stall did (what the grace uses).
    pub busy_seen: bool,
}

/// The watchdog's per-poll decision with its state: the consecutive
/// failure count and whether the current stall has shown work. The loop
/// feeds it every poll and acts on [`Step::verdict`]; it holds no other
/// kill logic.
#[derive(Debug, Clone)]
pub struct StallTracker {
    max_fails: u32,
    grace: Duration,
    fails: u32,
    /// Start of the stall the samples below belong to (the last Tick
    /// advance, as `now_ms - stalled_ms`).
    stall_start_ms: Option<i64>,
    /// Polls seen in this stall.
    polls_in_stall: u32,
    busy_seen: bool,
}

impl StallTracker {
    pub fn new(max_fails: u32, grace: Duration) -> Self {
        Self {
            max_fails,
            grace,
            fails: 0,
            stall_start_ms: None,
            polls_in_stall: 0,
            busy_seen: false,
        }
    }

    /// Decide one poll. `sample` is the main thread's `(cpu_ms, wall_ms)`
    /// since the previous poll, `None` when it could not be read (treated
    /// as idle, so an unreadable thread gets the old rules). `stalled_ms`:
    /// how long the Tick counter has not advanced at `now_ms`.
    pub fn step(
        &mut self,
        hb: Heartbeat,
        sample: Option<(i64, i64)>,
        now_ms: i64,
        stalled_ms: i64,
    ) -> Step {
        let poll = match hb {
            Heartbeat::Answered { stale } => {
                self.fails = 0;
                Poll::Answered { stale }
            }
            Heartbeat::Failed { ms_since_last_ok } => {
                self.fails = next_fail_count(self.fails, ms_since_last_ok, BUSY_GRACE_MS);
                Poll::Failed { fails: self.fails }
            }
        };

        // A Tick advance starts a new stall clock, and busy is per stall.
        let start = now_ms - stalled_ms;
        if self.stall_start_ms != Some(start) {
            self.stall_start_ms = Some(start);
            self.polls_in_stall = 0;
            self.busy_seen = false;
        }
        let busy_now = sample.is_some_and(|(cpu, wall)| main_thread_busy(cpu, wall));
        // The first two samples of a stall cover frames the thread still
        // ticked: the advancing poll's own, and the next one, which spans
        // the stall's onset. Only samples wholly inside the stall count.
        if self.polls_in_stall >= 2 && busy_now {
            self.busy_seen = true;
        }
        self.polls_in_stall = self.polls_in_stall.saturating_add(1);

        Step {
            verdict: decide(poll, self.max_fails, self.busy_seen, stalled_ms, self.grace),
            poll,
            busy_now,
            busy_seen: self.busy_seen,
        }
    }
}

#[cfg(test)]
#[path = "stall_grace_tests.rs"]
mod tests;
