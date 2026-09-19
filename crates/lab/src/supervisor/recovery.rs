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
use serde_json::Value;

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

/// The **persistent-hook re-application hook-point** (ADR §6): the
/// supervisor records every `hook_install` marked `persistent` and, after
/// a crash relaunch, replays exactly those installs — never writes, never
/// native calls, never non-persistent hooks. The bridge is the single
/// client, so the supervisor is the authority on what to re-apply; the
/// DLL-side registry dies with the client.
///
/// Keyed by the hook id the DLL returned, so a `hook_remove` drops the
/// right entry. Pure and unit-tested; the supervisor calls
/// [`note_install`](Self::note_install) / [`note_remove`](Self::note_remove)
/// from `bridge_call` and [`to_reapply`](Self::to_reapply) from the
/// recovery path.
#[derive(Debug, Clone, Default)]
pub struct PersistentHooks {
    entries: Vec<PersistentHook>,
}

/// One persistent hook: the id the DLL assigned and the original
/// `hook_install` params to replay verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentHook {
    pub id: u32,
    pub install_params: Value,
}

impl PersistentHooks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Note a completed `hook_install`. Records it **only when the install
    /// params carry `persistent: true`** — this is the "re-apply only
    /// persistent hooks" decision, kept here so it is unit-tested in one
    /// place. Returns whether it was recorded.
    pub fn note_install(&mut self, id: u32, install_params: &Value) -> bool {
        let persistent = install_params
            .get("persistent")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !persistent {
            return false;
        }
        // A repeat id (shouldn't happen — ids are monotonic per launch)
        // replaces rather than duplicates.
        self.entries.retain(|e| e.id != id);
        self.entries.push(PersistentHook {
            id,
            install_params: install_params.clone(),
        });
        true
    }

    /// Drop a persistent hook after a `hook_remove`. Returns whether one
    /// was removed.
    pub fn note_remove(&mut self, id: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != before
    }

    /// The `hook_install` param sets to replay after a crash, in install
    /// order. Every one is persistent by construction.
    pub fn to_reapply(&self) -> Vec<Value> {
        self.entries
            .iter()
            .map(|e| e.install_params.clone())
            .collect()
    }

    /// Number of persistent hooks currently tracked (surfaced by
    /// `lab_client_status`). Named `count` rather than `len` deliberately —
    /// this is a status figure, not a container length.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Drop all tracked hooks. Used by the recovery path to re-key the set
    /// with the fresh ids the relaunched client assigns (the old ids die
    /// with the crashed client).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    /// The re-application hook-point records **only** persistent installs,
    /// drops them on remove, and replays exactly those params — never a
    /// non-persistent hook. This is the "recovery re-applies only
    /// persistent hooks" guarantee.
    #[test]
    fn persistent_hooks_reapply_only_persistent() {
        let mut h = PersistentHooks::new();

        let persistent = json!({ "addr": "0x401000", "conv": "cdecl", "persistent": true });
        let ephemeral = json!({ "addr": "0x402000", "conv": "cdecl", "persistent": false });
        let no_flag = json!({ "addr": "0x403000", "conv": "cdecl" });

        assert!(
            h.note_install(1, &persistent),
            "persistent install recorded"
        );
        assert!(
            !h.note_install(2, &ephemeral),
            "non-persistent install not recorded"
        );
        assert!(
            !h.note_install(3, &no_flag),
            "missing persistent flag defaults to not-recorded"
        );

        let reapply = h.to_reapply();
        assert_eq!(reapply.len(), 1, "only the persistent hook is replayed");
        assert_eq!(reapply[0], persistent);
    }

    /// A `hook_remove` drops the persistent entry so a later crash does not
    /// resurrect a hook the agent explicitly removed.
    #[test]
    fn remove_drops_persistent_entry() {
        let mut h = PersistentHooks::new();
        h.note_install(1, &json!({ "addr": "0x401000", "persistent": true }));
        h.note_install(2, &json!({ "addr": "0x402000", "persistent": true }));
        assert_eq!(h.count(), 2);

        assert!(h.note_remove(1));
        assert!(!h.note_remove(1), "removing twice is a no-op");
        assert_eq!(h.count(), 1);
        assert_eq!(
            h.to_reapply()[0]["addr"],
            "0x402000",
            "the surviving hook is the one not removed"
        );
    }

    /// A re-installed id replaces rather than duplicates (ids are
    /// monotonic per launch, but be defensive).
    #[test]
    fn reinstall_same_id_replaces() {
        let mut h = PersistentHooks::new();
        h.note_install(1, &json!({ "addr": "0x401000", "persistent": true }));
        h.note_install(1, &json!({ "addr": "0x409999", "persistent": true }));
        assert_eq!(h.count(), 1);
        assert_eq!(h.to_reapply()[0]["addr"], "0x409999");
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
