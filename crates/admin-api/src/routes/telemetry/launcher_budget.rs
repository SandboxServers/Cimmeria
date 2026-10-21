//! The launcher's own budget on `/api/telemetry/upload-chunk` (AU-15). The
//! rows of a launcher chunk are filtered by [`super::launcher_rows`] and
//! charged here, never against the player session ledger in
//! [`super::session_budget`], so neither kind spends the other's allowance.
//!
//! Two ceilings. Each session has a lifetime allowance of rows
//! ([`DEFAULT_LAUNCHER_ROWS_PER_SESSION`]), and every launcher caller shares a
//! rows-per-minute ceiling ([`DEFAULT_LAUNCHER_ROWS_PER_MINUTE`]), which bounds
//! a flood of fresh session ids. The session table holds at most
//! [`LAUNCHER_LEDGER_SESSIONS`] entries; a new session the full table cannot
//! take is suppressed (fail closed), never admitted unbudgeted.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::routes::dev_session::quota::QuotaExceeded;
use crate::routes::dev_session::{env_u32, AuthError, TokenClaims};

use super::dto::TelemetryEvent;
use super::launcher_rows::admit_launcher_rows;

/// Rows a launcher session may replay over its lifetime (R15).
pub(super) const DEFAULT_LAUNCHER_ROWS_PER_SESSION: u32 = 300;

/// The env var that overrides [`DEFAULT_LAUNCHER_ROWS_PER_SESSION`].
pub(super) const LAUNCHER_ROWS_PER_SESSION_ENV: &str =
    "CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_SESSION";

/// Launcher rows replayed per minute across all launcher callers (R8).
pub(super) const DEFAULT_LAUNCHER_ROWS_PER_MINUTE: u32 = 3_000;

/// The env var that overrides [`DEFAULT_LAUNCHER_ROWS_PER_MINUTE`].
pub(super) const LAUNCHER_ROWS_PER_MINUTE_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_MINUTE";

/// Sessions the launcher ledger tracks at once.
pub(super) const LAUNCHER_LEDGER_SESSIONS: usize = 4_096;

/// Idle time after which a session's ledger entry is pruned.
pub(super) const LAUNCHER_LEDGER_TTL: Duration = Duration::from_secs(20 * 60);

/// The window of the global per-minute ceiling.
const MINUTE: Duration = Duration::from_secs(60);

/// The scope string of a refusal by the global per-minute ceiling.
const ROWS_SCOPE: &str = "upload/launcher_rows";

/// The launcher's budget: a lifetime allowance per session, and one
/// per-minute ceiling over all of them.
#[derive(Debug)]
pub(super) struct LauncherBudget {
    per_session: u32,
    per_minute: u32,
    /// Sessions tracked: rows charged so far, and when last charged.
    sessions: Mutex<HashMap<String, (u32, Instant)>>,
    /// The current minute's start, and the rows charged in it.
    minute: Mutex<(Instant, usize)>,
    /// Sessions the table holds at most.
    capacity: usize,
}

impl LauncherBudget {
    /// A budget with explicit ceilings, for tests and [`Self::from_env`].
    pub(super) fn new(per_session: u32, per_minute: u32) -> Self {
        Self {
            per_session,
            per_minute,
            sessions: Mutex::new(HashMap::new()),
            minute: Mutex::new((Instant::now(), 0)),
            capacity: LAUNCHER_LEDGER_SESSIONS,
        }
    }

    /// The budget from the environment, or the defaults for what is unset.
    pub(super) fn from_env() -> Self {
        Self::new(
            env_u32(
                LAUNCHER_ROWS_PER_SESSION_ENV,
                DEFAULT_LAUNCHER_ROWS_PER_SESSION,
            ),
            env_u32(
                LAUNCHER_ROWS_PER_MINUTE_ENV,
                DEFAULT_LAUNCHER_ROWS_PER_MINUTE,
            ),
        )
    }

    /// The same budget with a session table of `sessions` entries, for tests.
    #[cfg(test)]
    pub(super) fn with_capacity(mut self, sessions: usize) -> Self {
        self.capacity = sessions;
        self
    }

    /// Charge `rows` rows for `sid` at `now`. `Err(QuotaExceeded)` if the
    /// global minute would pass `per_minute` (nothing is charged); otherwise
    /// `Ok(n)`, where `n <= rows` rows still fit the session's lifetime
    /// budget (those `n` are charged globally). A new session the full table
    /// cannot take gets `Ok(0)`.
    pub(super) fn charge(&self, sid: &str, rows: usize, now: Instant) -> Result<usize, AuthError> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|p| p.into_inner());
        if !sessions.contains_key(sid) {
            sessions
                .retain(|_, (_, seen)| now.saturating_duration_since(*seen) <= LAUNCHER_LEDGER_TTL);
            if sessions.len() >= self.capacity {
                return Ok(0);
            }
        }
        let used = sessions.get(sid).map_or(0, |(used, _)| *used as usize);
        let room = (self.per_session as usize).saturating_sub(used);
        let n = rows.min(room);

        let mut minute = self.minute.lock().unwrap_or_else(|p| p.into_inner());
        if now.saturating_duration_since(minute.0) >= MINUTE {
            *minute = (now, 0);
        }
        if minute.1 + n > self.per_minute as usize {
            let left = MINUTE.saturating_sub(now.saturating_duration_since(minute.0));
            return Err(AuthError::QuotaExceeded(QuotaExceeded {
                scope: ROWS_SCOPE,
                // Round up, as the other quota windows do, so a retry at the
                // advertised time is never refused again in the same minute.
                retry_after_secs: left.as_secs().saturating_add(1),
            }));
        }
        minute.1 += n;
        let entry = sessions.entry(sid.to_string()).or_insert((0, now));
        entry.0 += n as u32;
        entry.1 = now;
        Ok(n)
    }
}

/// What the launcher path keeps of one decoded chunk.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct LauncherAdmission {
    /// Rows the filter dropped: counted as bad.
    pub dropped: u64,
    /// Per kept row, whether the budget admits it.
    pub admitted: Vec<bool>,
}

/// The launcher path after the decode: keep only the launcher rows, at most
/// `max_rows` of them, then charge the budget for those. An `Err` refuses
/// the whole chunk, and nothing is replayed.
pub(super) fn admit_launcher_chunk(
    budget: &LauncherBudget,
    claims: &TokenClaims,
    events: &mut Vec<TelemetryEvent>,
    max_rows: usize,
    now: Instant,
) -> Result<LauncherAdmission, AuthError> {
    let dropped = admit_launcher_rows(events, max_rows);
    let n = budget.charge(&claims.sid, events.len(), now)?;
    let admitted = (0..events.len()).map(|row| row < n).collect();
    Ok(LauncherAdmission { dropped, admitted })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ledger_prunes_and_fails_closed() {
        let budget = LauncherBudget::new(10, 1_000).with_capacity(2);
        let t0 = Instant::now();
        assert_eq!(budget.charge("a", 1, t0).unwrap(), 1);
        assert_eq!(budget.charge("b", 1, t0).unwrap(), 1);
        assert_eq!(
            budget.charge("c", 1, t0).unwrap(),
            0,
            "a third session the full table cannot take is suppressed"
        );
        let later = t0 + LAUNCHER_LEDGER_TTL + Duration::from_secs(1);
        assert_eq!(
            budget.charge("c", 1, later).unwrap(),
            1,
            "the idle entries are pruned, so the new session gets its rows"
        );
    }
}
