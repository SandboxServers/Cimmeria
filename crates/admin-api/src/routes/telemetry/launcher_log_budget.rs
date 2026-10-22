//! The row budgets of a launcher log session (C-LF6) and the log branch of the
//! chunk path.
//!
//! Three ceilings, each on whole chunks: a lifetime allowance per session, an
//! allowance per install per UTC day, and one per-minute ceiling over every
//! install. A chunk is checked against all three first and charged to all
//! three only if it fits; one that does not is refused whole and nothing of it
//! is written, so the uploader can retry the same chunk later.
//!
//! The session table and the install-day table hold at most
//! [`LOG_LEDGER_ENTRIES`] entries each. A session the full table cannot take
//! is refused (fail closed). A full install-day table drops the entry of the
//! oldest day, then takes the new install.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::routes::dev_session::quota::install_key;
use crate::routes::dev_session::{env_u32, TokenClaims};

use super::dto::{IngestError, TelemetryEvent};
use super::launcher_budget::{LauncherAdmission, LauncherBudget};
use super::launcher_log_rows::{admit_log_rows, LAUNCHER_LOG_TARGETS};
use super::launcher_rows::LAUNCHER_TARGETS;
use super::refusal_log::Truncation;

/// Rows one log session may replay over its lifetime.
pub(super) const DEFAULT_LOG_ROWS_PER_SESSION: u32 = 40_000;
/// The env var that overrides [`DEFAULT_LOG_ROWS_PER_SESSION`].
pub(super) const LOG_ROWS_PER_SESSION_ENV: &str = "CIMMERIA_TELEMETRY_LOG_ROWS_PER_SESSION";
/// Rows one install may replay per UTC day, over all its sessions.
pub(super) const DEFAULT_LOG_ROWS_PER_INSTALL_DAY: u32 = 80_000;
/// The env var that overrides [`DEFAULT_LOG_ROWS_PER_INSTALL_DAY`].
pub(super) const LOG_ROWS_PER_INSTALL_DAY_ENV: &str = "CIMMERIA_TELEMETRY_LOG_ROWS_PER_INSTALL_DAY";
/// Log rows replayed per minute, all installs together.
pub(super) const DEFAULT_LOG_ROWS_PER_MINUTE: u32 = 40_000;
/// The env var that overrides [`DEFAULT_LOG_ROWS_PER_MINUTE`].
pub(super) const LOG_ROWS_PER_MINUTE_ENV: &str = "CIMMERIA_TELEMETRY_LOG_ROWS_PER_MINUTE";

/// Entries each of the two tables holds.
pub(super) const LOG_LEDGER_ENTRIES: usize = 4_096;
/// Idle time after which a session's entry is pruned.
const LOG_LEDGER_TTL: Duration = Duration::from_secs(20 * 60);
/// The window of the global ceiling.
const MINUTE: Duration = Duration::from_secs(60);
const SECS_PER_DAY: u64 = 86_400;
/// The wait a full session table advertises.
const LEDGER_RETRY_SECS: u64 = 60;

const SESSION_REASON: &str = "log_session_budget";
const INSTALL_REASON: &str = "log_install_budget";
const GLOBAL_REASON: &str = "log_global_budget";

/// The log ledgers: session, install-day and global.
#[derive(Debug)]
pub(super) struct LogBudget {
    per_session: u32,
    per_install_day: u32,
    per_minute: u32,
    /// Rows charged so far per session, and when last charged.
    sessions: Mutex<HashMap<String, (u32, Instant)>>,
    /// Per install key: the UTC day (days since the epoch) and the rows
    /// charged on it.
    installs: Mutex<HashMap<u64, (u64, u32)>>,
    /// The current minute's start, and the rows charged in it.
    minute: Mutex<(Instant, usize)>,
    capacity: usize,
}

impl LogBudget {
    pub(super) fn new(per_session: u32, per_install_day: u32, per_minute: u32) -> Self {
        Self {
            per_session,
            per_install_day,
            per_minute,
            sessions: Mutex::new(HashMap::new()),
            installs: Mutex::new(HashMap::new()),
            minute: Mutex::new((Instant::now(), 0)),
            capacity: LOG_LEDGER_ENTRIES,
        }
    }

    /// The budget from the environment, or the defaults for what is unset.
    pub(super) fn from_env() -> Self {
        Self::new(
            env_u32(LOG_ROWS_PER_SESSION_ENV, DEFAULT_LOG_ROWS_PER_SESSION),
            env_u32(
                LOG_ROWS_PER_INSTALL_DAY_ENV,
                DEFAULT_LOG_ROWS_PER_INSTALL_DAY,
            ),
            env_u32(LOG_ROWS_PER_MINUTE_ENV, DEFAULT_LOG_ROWS_PER_MINUTE),
        )
    }

    /// The same budget with tables of `entries` entries, for tests.
    #[cfg(test)]
    pub(super) fn with_capacity(mut self, entries: usize) -> Self {
        self.capacity = entries;
        self
    }

    /// Installs the day table tracks, for tests.
    #[cfg(test)]
    pub(super) fn tracked_installs(&self) -> usize {
        self.installs
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    /// Charge `rows` rows of session `sid` of install `install` at `now`
    /// (`utc_secs` is the wall clock). All three ceilings are checked before
    /// any is charged. A zero-row chunk charges nothing and takes no slot.
    pub(super) fn admit(
        &self,
        sid: &str,
        install: &str,
        rows: usize,
        now: Instant,
        utc_secs: u64,
    ) -> Result<(), IngestError> {
        if rows == 0 {
            return Ok(());
        }
        let today = utc_secs / SECS_PER_DAY;
        let key = install_key(install);
        let mut sessions = self.sessions.lock().unwrap_or_else(|p| p.into_inner());
        let mut installs = self.installs.lock().unwrap_or_else(|p| p.into_inner());
        let mut minute = self.minute.lock().unwrap_or_else(|p| p.into_inner());

        if !sessions.contains_key(sid) {
            sessions.retain(|_, (_, seen)| now.saturating_duration_since(*seen) <= LOG_LEDGER_TTL);
            if sessions.len() >= self.capacity {
                return Err(refusal(SESSION_REASON, Some(LEDGER_RETRY_SECS)));
            }
        }
        let used = sessions.get(sid).map_or(0, |(used, _)| *used as usize);
        if used + rows > self.per_session as usize {
            return Err(refusal(SESSION_REASON, None));
        }

        let day_used = match installs.get(&key) {
            Some((day, used)) if *day == today => *used as usize,
            _ => 0,
        };
        if day_used + rows > self.per_install_day as usize {
            let to_midnight = (today + 1) * SECS_PER_DAY - utc_secs;
            return Err(refusal(INSTALL_REASON, Some(to_midnight.max(1))));
        }

        if now.saturating_duration_since(minute.0) >= MINUTE {
            *minute = (now, 0);
        }
        if minute.1 + rows > self.per_minute as usize {
            let left = MINUTE.saturating_sub(now.saturating_duration_since(minute.0));
            // Rounded up, so a retry at the advertised time is not refused
            // again in the same minute.
            return Err(IngestError::LogBudget {
                reason: GLOBAL_REASON,
                global: true,
                retry_after_secs: Some(left.as_secs().saturating_add(1)),
            });
        }

        // Everything fits: charge all three.
        if !installs.contains_key(&key) && installs.len() >= self.capacity {
            installs.retain(|_, (day, _)| *day >= today);
            if installs.len() >= self.capacity {
                if let Some(oldest) = installs.iter().min_by_key(|(_, (day, _))| *day) {
                    let oldest = *oldest.0;
                    installs.remove(&oldest);
                }
            }
        }
        minute.1 += rows;
        let entry = sessions.entry(sid.to_string()).or_insert((0, now));
        entry.0 += rows as u32;
        entry.1 = now;
        installs.insert(key, (today, (day_used + rows) as u32));
        Ok(())
    }
}

fn refusal(reason: &'static str, retry_after_secs: Option<u64>) -> IngestError {
    IngestError::LogBudget {
        reason,
        global: false,
        retry_after_secs,
    }
}

/// An event session's chunk must hold no row of a log target (C-LF4).
pub(super) fn check_event_targets(events: &[TelemetryEvent]) -> Result<(), IngestError> {
    let wrong = events.iter().any(|event| {
        matches!(event, TelemetryEvent::ClientNative(row)
            if LAUNCHER_LOG_TARGETS.contains(&row.target.as_str()))
    });
    if wrong {
        Err(IngestError::PurposeTarget)
    } else {
        Ok(())
    }
}

/// The log path after the decode: a row of an event target refuses the chunk
/// (403), a chunk past its row or byte cap is refused (413, not truncated: a
/// log uploader splits and resends), rows that are not valid log rows are
/// dropped and counted, and the kept rows are charged to the log budget as a
/// whole chunk or refused.
pub(super) fn admit_log_chunk(
    budget: &LauncherBudget,
    claims: &TokenClaims,
    events: &mut Vec<TelemetryEvent>,
    truncation: Option<&Truncation>,
    now: Instant,
    utc_secs: u64,
) -> Result<LauncherAdmission, IngestError> {
    let event_target = events.iter().any(|event| {
        matches!(event, TelemetryEvent::ClientNative(row)
            if LAUNCHER_TARGETS.contains(&row.target.as_str()))
    });
    if event_target {
        return Err(IngestError::PurposeTarget);
    }
    if let Some(t) = truncation {
        return Err(IngestError::OverBudget {
            what: t.budget,
            limit: t.limit,
        });
    }
    let total = events.len();
    let natives = events
        .drain(..)
        .filter_map(|event| match event {
            TelemetryEvent::ClientNative(row) => Some(row),
            _ => None,
        })
        .collect();
    let (kept, _) = admit_log_rows(natives);
    *events = kept.into_iter().map(TelemetryEvent::ClientNative).collect();
    budget
        .logs()
        .admit(&claims.sid, &claims.sub, events.len(), now, utc_secs)?;
    Ok(LauncherAdmission {
        dropped: (total - events.len()) as u64,
        admitted: vec![true; events.len()],
    })
}
