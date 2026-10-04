//! Per-session accounting and the runaway-client guard for
//! `/api/telemetry/upload-chunk`.
//!
//! Each session (the token's `sid`) gets a running total of events
//! replayed and suppressed, and a fixed window of [`WINDOW_SECS`] with a
//! budget of [`EVENTS_PER_WINDOW`] replayed events. A governed DLL sends
//! well under 100 events a minute and a lab session with the `raw` capture
//! switch about 6,000, so the budget only trips for a client that has
//! lost its governor or its mind.
//!
//! Over budget, the chunk is still accepted (a refused chunk is retried by
//! the uploader and would only add load), but only **priority** events are
//! replayed: warn/error levels, session metadata, and the DLL's boot, hook,
//! entity-lifecycle, Mercury-anomaly and governor reports. Everything else
//! is counted as suppressed, and the handler logs every chunk that
//! suppressed something at `warn` with the session's totals. Never silent.
//!
//! The table holds at most [`MAX_SESSIONS`] sessions. A new session evicts
//! the least recently seen session whose window has already ended, so an
//! eviction never hands a session still inside its window a fresh budget
//! when it comes back. When every tracked session is inside its window,
//! the newcomer is counted against one shared [`OVERFLOW_SID`] entry
//! instead: bounded, and it never resets anyone's allowance.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::routes::dev_session::TokenClaims;

use super::dto::TelemetryEvent;
use super::replay::{replay_ndjson_gated, ReplayCounts, ReplayError};

/// Budget window.
pub(super) const WINDOW_SECS: i64 = 60;

/// Events replayed per session per window before only priority events are.
pub(super) const EVENTS_PER_WINDOW: u64 = 30_000;

/// Sessions tracked at once.
pub(super) const MAX_SESSIONS: usize = 1024;

/// The shared entry that counts sessions arriving while the table is full
/// of sessions inside their window. Not a valid token `sid`.
pub(super) const OVERFLOW_SID: &str = "<overflow>";

/// One session's counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SessionTotals {
    /// Events replayed since the server started seeing the session.
    pub(super) accepted_total: u64,
    /// Events suppressed by the budget since then.
    pub(super) suppressed_total: u64,
    /// Start of the current window, in seconds.
    pub(super) window_start: i64,
    /// Events counted against the current window.
    pub(super) window_count: u64,
    /// Last time the session uploaded, in seconds.
    pub(super) last_seen: i64,
}

/// The session table.
#[derive(Debug)]
pub(super) struct SessionLedger {
    sessions: HashMap<String, SessionTotals>,
    budget: u64,
    window_secs: i64,
    max_sessions: usize,
}

impl Default for SessionLedger {
    fn default() -> Self {
        Self::new(EVENTS_PER_WINDOW, WINDOW_SECS, MAX_SESSIONS)
    }
}

impl SessionLedger {
    /// A ledger with an explicit budget, for tests.
    pub(super) fn new(budget: u64, window_secs: i64, max_sessions: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            budget,
            window_secs,
            max_sessions,
        }
    }

    /// Decide for one event of session `sid` at `now_secs`: `true` means
    /// replay it. Priority events are always replayed (and counted).
    pub(super) fn admit(&mut self, sid: &str, priority: bool, now_secs: i64) -> bool {
        let key = self.slot_for(sid, now_secs);
        let t = self.sessions.entry(key).or_insert(SessionTotals {
            window_start: now_secs,
            ..SessionTotals::default()
        });
        t.last_seen = now_secs;
        if now_secs - t.window_start >= self.window_secs {
            t.window_start = now_secs;
            t.window_count = 0;
        }
        if priority || t.window_count < self.budget {
            t.window_count += 1;
            t.accepted_total += 1;
            true
        } else {
            t.suppressed_total += 1;
            false
        }
    }

    /// Which entry counts `sid`: its own when tracked or when there is
    /// room (evicting a session whose window has ended if needed), else
    /// the shared overflow entry.
    fn slot_for(&mut self, sid: &str, now_secs: i64) -> String {
        if self.sessions.contains_key(sid) {
            return sid.to_string();
        }
        // The overflow entry does not count against the cap.
        let tracked = self.sessions.len() - usize::from(self.sessions.contains_key(OVERFLOW_SID));
        if tracked < self.max_sessions {
            return sid.to_string();
        }
        let window = self.window_secs;
        let expired = self
            .sessions
            .iter()
            .filter(|(k, t)| k.as_str() != OVERFLOW_SID && now_secs - t.window_start >= window)
            .min_by_key(|(_, t)| t.last_seen)
            .map(|(k, _)| k.clone());
        match expired {
            Some(oldest) => {
                self.sessions.remove(&oldest);
                sid.to_string()
            }
            None => OVERFLOW_SID.to_string(),
        }
    }

    /// The counters that apply to `sid`: its own if tracked, else the
    /// shared overflow entry's (if the session was counted there).
    pub(super) fn totals(&self, sid: &str) -> Option<SessionTotals> {
        self.sessions
            .get(sid)
            .or_else(|| self.sessions.get(OVERFLOW_SID))
            .copied()
    }

    /// Sessions tracked (the overflow entry included).
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether `sid` has its own entry.
    #[cfg(test)]
    pub(super) fn tracks(&self, sid: &str) -> bool {
        self.sessions.contains_key(sid)
    }
}

/// The process-wide ledger the upload handler uses.
static LEDGER: Mutex<Option<SessionLedger>> = Mutex::new(None);

/// Run `f` on the process-wide ledger. Poisoning is ignored: the counters
/// are advisory and must never take the ingest path down.
fn with_ledger<R>(f: impl FnOnce(&mut SessionLedger) -> R) -> R {
    let mut g = LEDGER.lock().unwrap_or_else(PoisonError::into_inner);
    f(g.get_or_insert_with(SessionLedger::default))
}

/// DLL targets replayed even over budget: the governor's own reports and
/// the must-keep families of its classification table
/// (`cimmeria-client-telemetry` `governor::classify`), so the server-side
/// guard never cuts what the client-side one promised to keep.
const PRIORITY_PREFIXES: &[&str] = &[
    "client.telemetry.",
    "client.dll.",
    "client.hooks.",
    "client.entity.",
    "client.mercury.error",
    "client.mercury.fragment",
    "client.mercury.bundle",
    "client.mercury.request_misparse",
    "client.mercury.unpack_fault",
    "client.dispatch.method_dropped",
    // Every ability row (AB-C1 to AB-C5: press, drop, send, recv, applied,
    // shown): must-keep in the DLL's governor, already held to a
    // per-name budget at the hook.
    "client.ability.",
];

/// Replay one chunk for `claims.sid` at `now_secs` under the process-wide
/// ledger: what the upload handler calls. Returns the chunk's counts and
/// the session's totals after it.
pub(super) fn replay_budgeted(
    claims: &TokenClaims,
    ndjson: &str,
    now_secs: i64,
) -> Result<(ReplayCounts, SessionTotals), ReplayError> {
    let counts = replay_ndjson_gated(claims, ndjson, |ev| {
        let priority = is_priority(ev);
        with_ledger(|l| l.admit(&claims.sid, priority, now_secs))
    })?;
    let totals = with_ledger(|l| l.totals(&claims.sid)).unwrap_or_default();
    Ok((counts, totals))
}

/// Whether an event is replayed even over budget.
pub(super) fn is_priority(ev: &TelemetryEvent) -> bool {
    let loud = |level: &str| {
        ["warn", "warning", "error", "fatal"]
            .iter()
            .any(|l| level.eq_ignore_ascii_case(l))
    };
    match ev {
        TelemetryEvent::ClientNative(e) => {
            loud(&e.level) || PRIORITY_PREFIXES.iter().any(|p| e.target.starts_with(p))
        }
        TelemetryEvent::ClientLog(e) => loud(&e.level),
        TelemetryEvent::DebugLog(e) => loud(&e.level),
        TelemetryEvent::SessionMeta(_) => true,
        TelemetryEvent::KeyDump(_) => false,
    }
}
