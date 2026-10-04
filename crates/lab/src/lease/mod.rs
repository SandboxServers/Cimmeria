//! The lab lease: one agent drives the lab client at a time, enforced.
//!
//! The folder lock (`%LOCALAPPDATA%\cimmeria-lab\live.lock`) was an honour
//! system: nothing stopped a session that skipped it, and a stdio
//! supervisor's watchdog relaunched a client another session had closed.
//! A lease lives in the supervisor process itself, so with the shared daemon
//! every session sees the same one, and every tool that drives the client
//! refuses without it ([`policy`]).
//!
//! Rules (ADR addendum in `docs/architecture/live-research-lab.md`):
//! - `lab_lease_acquire {owner, purpose, ttl_s}` grants a lease for `ttl_s`
//!   seconds (default [`DEFAULT_TTL_S`], at most [`MAX_TTL_S`]), or refuses
//!   naming the holder, their purpose and since when.
//! - `force: true` with a `reason` takes the lease over: logged at WARN,
//!   the previous holder kept on the new lease and in the history.
//! - Every lease-guarded tool call renews the lease (a touch).
//! - An expired lease is logged and gone; with no lease the watchdog does
//!   not relaunch a dead client (`watchdog_idle_no_lease`).
//!
//! One book per process ([`global`]), shared by the default supervisor and
//! the in-process second-player supervisor: the lease covers the lab, not
//! one client.
//!
//! Telemetry: one `lab.lease` event per acquire, renew, release, expire and
//! force (target `lab.lease`, field `event`), with owner and purpose. A touch
//! is `debug`, not a row: it happens on every guarded call.

pub mod policy;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};

use crate::supervisor::now_ms;

/// Default lease length.
pub const DEFAULT_TTL_S: u64 = 600;
/// Longest lease one acquire or renew grants.
pub const MAX_TTL_S: u64 = 3600;
/// Shortest lease (a lease that expires between two tool calls is useless).
pub const MIN_TTL_S: u64 = 30;
/// Ended leases remembered, so a stale `lease_id` gets a useful refusal.
const HISTORY: usize = 16;

/// A granted lease.
#[derive(Debug, Clone, Serialize)]
pub struct Lease {
    pub lease_id: String,
    pub owner: String,
    pub purpose: String,
    pub ttl_s: u64,
    pub acquired_ms: i64,
    pub renewed_ms: i64,
    pub expires_ms: i64,
    /// The holder this lease displaced with `force`, if any.
    pub took_over_from: Option<Ended>,
}

/// A lease that is over, and how it ended.
#[derive(Debug, Clone, Serialize)]
pub struct Ended {
    #[serde(skip)]
    pub lease_id: String,
    pub owner: String,
    pub purpose: String,
    pub acquired_ms: i64,
    pub ended_ms: i64,
    /// `released`, `expired` or `taken_over`.
    pub how: &'static str,
    /// Who took it over and why (`taken_over` only).
    pub by: Option<String>,
    pub reason: Option<String>,
}

/// `lab_lease_acquire` arguments, validated by [`LeaseBook::acquire`].
#[derive(Debug, Clone, Default)]
pub struct AcquireRequest {
    pub owner: String,
    pub purpose: String,
    pub ttl_s: Option<u64>,
    pub force: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Default)]
struct Inner {
    current: Option<Lease>,
    history: VecDeque<Ended>,
}

/// The process's lease state.
#[derive(Debug, Default)]
pub struct LeaseBook {
    inner: Mutex<Inner>,
}

static GLOBAL: OnceLock<Arc<LeaseBook>> = OnceLock::new();

/// The process-wide book every supervisor shares.
pub fn global() -> Arc<LeaseBook> {
    GLOBAL
        .get_or_init(|| Arc::new(LeaseBook::default()))
        .clone()
}

/// Log expired leases promptly, not only when the next call notices.
pub fn spawn_sweeper(book: Arc<LeaseBook>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            book.sweep();
        }
    });
}

pub fn rfc3339(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn new_lease_id() -> String {
    format!("lease-{}", uuid::Uuid::new_v4().simple())
}

fn holder_text(l: &Lease, now: i64) -> String {
    format!(
        "{} ({}) since {}, expires {} (in {} s)",
        l.owner,
        l.purpose,
        rfc3339(l.acquired_ms),
        rfc3339(l.expires_ms),
        (l.expires_ms - now).max(0) / 1000
    )
}

fn lease_json(l: &Lease, now: i64) -> Value {
    json!({
        "owner": l.owner,
        "purpose": l.purpose,
        "since": rfc3339(l.acquired_ms),
        "renewed_at": rfc3339(l.renewed_ms),
        "expires_at": rfc3339(l.expires_ms),
        "remaining_s": (l.expires_ms - now).max(0) / 1000,
        "ttl_s": l.ttl_s,
        "took_over_from": l.took_over_from.as_ref().map(ended_json),
    })
}

fn ended_json(e: &Ended) -> Value {
    json!({
        "owner": e.owner,
        "purpose": e.purpose,
        "since": rfc3339(e.acquired_ms),
        "ended_at": rfc3339(e.ended_ms),
        "how": e.how,
        "by": e.by,
        "reason": e.reason,
    })
}

/// What [`LeaseBook::acquire`] returns to the caller: the id is the
/// capability, so it is only ever handed to the acquirer.
pub fn grant_json(l: &Lease) -> Value {
    json!({
        "lease_id": l.lease_id,
        "owner": l.owner,
        "purpose": l.purpose,
        "ttl_s": l.ttl_s,
        "expires_at": rfc3339(l.expires_ms),
        "took_over_from": l.took_over_from.as_ref().map(ended_json),
        "note": "Pass lease_id to every tool that drives the client; each such call renews the lease. lab_lease_release when you are done.",
    })
}

fn validate_ttl(ttl_s: Option<u64>) -> Result<u64, String> {
    let t = ttl_s.unwrap_or(DEFAULT_TTL_S);
    if !(MIN_TTL_S..=MAX_TTL_S).contains(&t) {
        return Err(format!(
            "ttl_s {t}: a lease lasts {MIN_TTL_S}..={MAX_TTL_S} seconds (default {DEFAULT_TTL_S})"
        ));
    }
    Ok(t)
}

impl Inner {
    fn remember(&mut self, e: Ended) {
        if self.history.len() == HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(e);
    }

    /// End the current lease if it has run out. Logs the expiry once.
    fn expire_if_due(&mut self, now: i64) {
        let due = self.current.as_ref().is_some_and(|l| l.expires_ms <= now);
        if !due {
            return;
        }
        let l = self.current.take().expect("checked above");
        tracing::info!(target: "lab.lease", event = "expire", owner = %l.owner,
            purpose = %l.purpose, expired_at = %rfc3339(l.expires_ms),
            "lab lease expired");
        self.remember(Ended {
            lease_id: l.lease_id,
            owner: l.owner,
            purpose: l.purpose,
            acquired_ms: l.acquired_ms,
            ended_ms: l.expires_ms,
            how: "expired",
            by: None,
            reason: None,
        });
    }

    /// Why `id` is not the current lease, from the history when it is known.
    fn stale_reason(&self, id: &str) -> Option<String> {
        self.history
            .iter()
            .rev()
            .find(|e| e.lease_id == id)
            .map(|e| match e.how {
                "taken_over" => format!(
                    "lease {id} was taken over at {} by {} (reason: {})",
                    rfc3339(e.ended_ms),
                    e.by.as_deref().unwrap_or("?"),
                    e.reason.as_deref().unwrap_or("?")
                ),
                how => format!("lease {id} {how} at {}", rfc3339(e.ended_ms)),
            })
    }
}

impl LeaseBook {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves plain data; keep going.
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn acquire(&self, req: AcquireRequest) -> Result<Lease, String> {
        self.acquire_at(req, now_ms())
    }

    pub fn acquire_at(&self, req: AcquireRequest, now: i64) -> Result<Lease, String> {
        let owner = req.owner.trim().to_string();
        let purpose = req.purpose.trim().to_string();
        if owner.is_empty() || purpose.is_empty() {
            return Err(
                "owner and purpose are required: who you are (session or agent name) and what you are doing".into(),
            );
        }
        let ttl_s = validate_ttl(req.ttl_s)?;
        let mut st = self.lock();
        st.expire_if_due(now);

        let mut took_over_from = None;
        if let Some(cur) = st.current.as_ref() {
            if !req.force {
                return Err(format!(
                    "the lab is leased to {}. Wait for it, ask its owner to lab_lease_release, \
                     or take it over with lab_lease_acquire {{force: true, reason}}",
                    holder_text(cur, now)
                ));
            }
            let reason = req
                .reason
                .as_deref()
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .ok_or("force: true needs a reason (it is logged and shown to the holder)")?
                .to_string();
            let prev = st.current.take().expect("checked above");
            tracing::warn!(target: "lab.lease", event = "force", owner = %owner,
                purpose = %purpose, prev_owner = %prev.owner, prev_purpose = %prev.purpose,
                prev_since = %rfc3339(prev.acquired_ms), reason = %reason,
                "lab lease taken over");
            let ended = Ended {
                lease_id: prev.lease_id,
                owner: prev.owner,
                purpose: prev.purpose,
                acquired_ms: prev.acquired_ms,
                ended_ms: now,
                how: "taken_over",
                by: Some(owner.clone()),
                reason: Some(reason),
            };
            st.remember(ended.clone());
            took_over_from = Some(ended);
        }

        let lease = Lease {
            lease_id: new_lease_id(),
            owner,
            purpose,
            ttl_s,
            acquired_ms: now,
            renewed_ms: now,
            expires_ms: now + (ttl_s as i64) * 1000,
            took_over_from,
        };
        tracing::info!(target: "lab.lease", event = "acquire", owner = %lease.owner,
            purpose = %lease.purpose, ttl_s, forced = lease.took_over_from.is_some(),
            expires_at = %rfc3339(lease.expires_ms), "lab lease acquired");
        st.current = Some(lease.clone());
        Ok(lease)
    }

    /// Explicit renew: extend by `ttl_s` (default: the lease's own).
    pub fn renew(&self, id: &str, ttl_s: Option<u64>) -> Result<Lease, String> {
        self.renew_at(id, ttl_s, now_ms())
    }

    pub fn renew_at(&self, id: &str, ttl_s: Option<u64>, now: i64) -> Result<Lease, String> {
        let mut st = self.lock();
        st.expire_if_due(now);
        let ttl = match ttl_s {
            Some(t) => validate_ttl(Some(t))?,
            None => match st.current.as_ref() {
                Some(l) => l.ttl_s,
                None => DEFAULT_TTL_S,
            },
        };
        let stale = st.stale_reason(id);
        let Some(l) = st.current.as_mut().filter(|l| l.lease_id == id) else {
            return Err(stale.unwrap_or_else(|| format!("lease {id} is not the current lease")));
        };
        l.ttl_s = ttl;
        l.renewed_ms = now;
        l.expires_ms = now + (ttl as i64) * 1000;
        tracing::info!(target: "lab.lease", event = "renew", owner = %l.owner,
            purpose = %l.purpose, ttl_s = ttl, expires_at = %rfc3339(l.expires_ms),
            "lab lease renewed");
        Ok(l.clone())
    }

    pub fn release(&self, id: &str) -> Result<Ended, String> {
        self.release_at(id, now_ms())
    }

    pub fn release_at(&self, id: &str, now: i64) -> Result<Ended, String> {
        let mut st = self.lock();
        st.expire_if_due(now);
        if st.current.as_ref().is_none_or(|l| l.lease_id != id) {
            let stale = st.stale_reason(id);
            return Err(stale.unwrap_or_else(|| format!("lease {id} is not the current lease")));
        }
        let l = st.current.take().expect("checked above");
        tracing::info!(target: "lab.lease", event = "release", owner = %l.owner,
            purpose = %l.purpose, held_s = (now - l.acquired_ms) / 1000, "lab lease released");
        let e = Ended {
            lease_id: l.lease_id,
            owner: l.owner,
            purpose: l.purpose,
            acquired_ms: l.acquired_ms,
            ended_ms: now,
            how: "released",
            by: None,
            reason: None,
        };
        st.remember(e.clone());
        Ok(e)
    }

    /// The gate for a lease-guarded tool: `id` must be the current lease.
    /// A pass renews it (a touch).
    pub fn check(&self, id: Option<&str>, tool: &str) -> Result<(), String> {
        self.check_at(id, tool, now_ms())
    }

    pub fn check_at(&self, id: Option<&str>, tool: &str, now: i64) -> Result<(), String> {
        let mut st = self.lock();
        st.expire_if_due(now);
        let held_by = st
            .current
            .as_ref()
            .map(|l| format!(" The lab is leased to {}.", holder_text(l, now)))
            .unwrap_or_default();
        let Some(id) = id.map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(format!(
                "{tool} drives the lab client and needs a lease: call lab_lease_acquire \
                 {{owner, purpose}} and pass the lease_id it returns.{held_by}"
            ));
        };
        let stale = st.stale_reason(id);
        match st.current.as_mut() {
            Some(l) if l.lease_id == id => {
                l.renewed_ms = now;
                l.expires_ms = now + (l.ttl_s as i64) * 1000;
                tracing::debug!(target: "lab.lease", event = "touch", owner = %l.owner, tool,
                    "lab lease renewed by a guarded call");
                Ok(())
            }
            _ => Err(format!(
                "{tool} refused: {}. Acquire a new lease with lab_lease_acquire.{held_by}",
                stale.unwrap_or_else(|| format!("lease {id} is not the current lease"))
            )),
        }
    }

    /// Whether a valid lease exists (the watchdog's relaunch gate).
    pub fn is_held(&self) -> bool {
        self.is_held_at(now_ms())
    }

    pub fn is_held_at(&self, now: i64) -> bool {
        let mut st = self.lock();
        st.expire_if_due(now);
        st.current.is_some()
    }

    pub fn sweep(&self) {
        self.lock().expire_if_due(now_ms());
    }

    /// `lab_lease_status`: who holds the lab, never the lease id.
    pub fn status(&self) -> Value {
        self.status_at(now_ms())
    }

    pub fn status_at(&self, now: i64) -> Value {
        let mut st = self.lock();
        st.expire_if_due(now);
        json!({
            "held": st.current.is_some(),
            "lease": st.current.as_ref().map(|l| lease_json(l, now)),
            "recent": st.history.iter().rev().take(5).map(ended_json).collect::<Vec<_>>(),
            "rules": {
                "default_ttl_s": DEFAULT_TTL_S,
                "max_ttl_s": MAX_TTL_S,
                "guarded_calls_renew": true,
                "watchdog_relaunches_only_while_leased": true,
            },
        })
    }
}

#[cfg(test)]
mod tests;
