//! Crash-recovery bookkeeping: the command journal (with
//! quarantine-on-crash) and the recovery-attempt cap.
//!
//! Both are pure, injectable-clock, and unit-tested off-process
//! (issue #685 scope 5).
//!
//! - [`CommandJournal`] is the supervisor's ring of the last N bridge
//!   commands. The supervisor is the bridge's single client, so it is
//!   the authority on what was sent and which command was in flight when
//!   the client faulted. On crash it marks the in-flight command
//!   *quarantined*; `lab_crash_report` surfaces the ring and the
//!   quarantined command, and recovery never replays it (ADR §6).
//! - [`RecoveryTracker`] caps automatic relaunches at three crashes in
//!   ten minutes so a reliably-crashing probe can't spin the client
//!   forever.

use std::collections::VecDeque;

use serde::Serialize;

/// Lifecycle of one journaled command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    /// Sent, no response yet — the candidate for quarantine on a crash.
    InFlight,
    /// Completed with a success result.
    Completed,
    /// Completed with an error response (a normal outcome, not a crash).
    Failed,
    /// Was in flight when the client crashed; never replayed.
    Quarantined,
}

/// One entry in the command journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandRecord {
    pub seq: u64,
    pub method: String,
    pub ts_ms: i64,
    pub state: CommandState,
}

/// Bounded ring of recent bridge commands.
#[derive(Debug, Clone)]
pub struct CommandJournal {
    cap: usize,
    entries: VecDeque<CommandRecord>,
    next_seq: u64,
}

impl CommandJournal {
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            entries: VecDeque::new(),
            next_seq: 1,
        }
    }

    /// Record a command as sent (in flight). Returns its sequence number,
    /// which the caller passes back to [`complete`](Self::complete).
    /// Evicts the oldest entry once the ring is full.
    pub fn record(&mut self, method: impl Into<String>, ts_ms: i64) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push_back(CommandRecord {
            seq,
            method: method.into(),
            ts_ms,
            state: CommandState::InFlight,
        });
        while self.entries.len() > self.cap {
            self.entries.pop_front();
        }
        seq
    }

    /// Mark a previously-recorded command complete. `ok` chooses
    /// `Completed` vs `Failed`. A crash-quarantined entry is never
    /// overwritten (the response, if any, arrives after we've given up).
    pub fn complete(&mut self, seq: u64, ok: bool) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.seq == seq) {
            if e.state == CommandState::InFlight {
                e.state = if ok {
                    CommandState::Completed
                } else {
                    CommandState::Failed
                };
            }
        }
    }

    /// On crash: mark every still-in-flight command quarantined. There
    /// is normally at most one (the client is synchronous), but marking
    /// all of them is correct and cheap.
    pub fn quarantine_in_flight(&mut self) {
        for e in self.entries.iter_mut() {
            if e.state == CommandState::InFlight {
                e.state = CommandState::Quarantined;
            }
        }
    }

    /// The most recent `n` entries, oldest-first.
    pub fn recent(&self, n: usize) -> Vec<CommandRecord> {
        let start = self.entries.len().saturating_sub(n);
        self.entries.iter().skip(start).cloned().collect()
    }

    /// All quarantined commands.
    pub fn quarantined(&self) -> Vec<CommandRecord> {
        self.entries
            .iter()
            .filter(|e| e.state == CommandState::Quarantined)
            .cloned()
            .collect()
    }
}

/// Caps automatic relaunches: stop after `max` crashes within
/// `window_ms`.
#[derive(Debug, Clone)]
pub struct RecoveryTracker {
    window_ms: i64,
    max: usize,
    crashes: VecDeque<i64>,
}

impl RecoveryTracker {
    /// ADR §6: three crashes in ten minutes.
    pub fn new_default() -> Self {
        Self::new(3, 10 * 60 * 1000)
    }

    pub fn new(max: usize, window_ms: i64) -> Self {
        Self {
            window_ms,
            max,
            crashes: VecDeque::new(),
        }
    }

    /// Record a crash at `now_ms`, pruning any outside the window.
    pub fn record_crash(&mut self, now_ms: i64) {
        self.crashes.push_back(now_ms);
        self.prune(now_ms);
    }

    /// Whether the supervisor may still relaunch. False once `max`
    /// crashes have occurred within the trailing window.
    pub fn should_relaunch(&mut self, now_ms: i64) -> bool {
        self.prune(now_ms);
        self.crashes.len() < self.max
    }

    /// Crashes recorded within the trailing window (for status output).
    pub fn recent_crash_count(&mut self, now_ms: i64) -> usize {
        self.prune(now_ms);
        self.crashes.len()
    }

    fn prune(&mut self, now_ms: i64) {
        let cutoff = now_ms - self.window_ms;
        while self.crashes.front().is_some_and(|&t| t < cutoff) {
            self.crashes.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Record three, complete two, crash: the in-flight one becomes
    /// quarantined; the completed ones are untouched.
    #[test]
    fn quarantine_marks_only_the_in_flight_command() {
        let mut j = CommandJournal::new(16);
        let a = j.record("mem_write", 1);
        let b = j.record("lua_eval", 2);
        let c = j.record("call_native", 3); // stays in flight
        j.complete(a, true);
        j.complete(b, false);

        j.quarantine_in_flight();

        let recent = j.recent(3);
        assert_eq!(recent[0].state, CommandState::Completed);
        assert_eq!(recent[1].state, CommandState::Failed);
        assert_eq!(recent[2].state, CommandState::Quarantined);
        assert_eq!(recent[2].seq, c);

        let q = j.quarantined();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].method, "call_native");
    }

    /// The ring evicts oldest beyond capacity.
    #[test]
    fn ring_caps_and_evicts_oldest() {
        let mut j = CommandJournal::new(2);
        j.record("a", 1);
        j.record("b", 2);
        j.record("c", 3);
        let recent = j.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].method, "b");
        assert_eq!(recent[1].method, "c");
    }

    /// A late response after quarantine does not un-quarantine.
    #[test]
    fn complete_after_quarantine_is_ignored() {
        let mut j = CommandJournal::new(4);
        let s = j.record("mem_write", 1);
        j.quarantine_in_flight();
        j.complete(s, true);
        assert_eq!(j.recent(1)[0].state, CommandState::Quarantined);
    }

    /// Three crashes in ten minutes stops relaunch; the fourth attempt
    /// is refused.
    #[test]
    fn three_crashes_in_ten_minutes_caps_relaunch() {
        let mut r = RecoveryTracker::new_default();
        let base = 1_000_000i64;
        assert!(r.should_relaunch(base)); // no crashes yet
        r.record_crash(base);
        assert!(r.should_relaunch(base + 1000)); // 1 crash
        r.record_crash(base + 2000);
        assert!(r.should_relaunch(base + 3000)); // 2 crashes
        r.record_crash(base + 4000);
        assert!(
            !r.should_relaunch(base + 5000),
            "3 crashes in the window ⇒ stop relaunching"
        );
        assert_eq!(r.recent_crash_count(base + 5000), 3);
    }

    /// Crashes older than the window fall off, re-permitting relaunch.
    #[test]
    fn old_crashes_fall_out_of_window() {
        let mut r = RecoveryTracker::new(3, 10 * 60 * 1000);
        let base = 1_000_000i64;
        r.record_crash(base);
        r.record_crash(base + 1000);
        r.record_crash(base + 2000);
        assert!(!r.should_relaunch(base + 3000));
        // Eleven minutes later, all three have aged out.
        let later = base + 11 * 60 * 1000;
        assert!(r.should_relaunch(later));
        assert_eq!(r.recent_crash_count(later), 0);
    }
}
