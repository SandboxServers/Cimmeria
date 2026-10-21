//! The log rows for a refused dev-session request. No row names the peer
//! address: the quota keys stay in memory, and nothing is hashed (directive
//! 6, D-LT18). A refusal that repeats is counted rather than written: one
//! row per (route, reason) per [`WINDOW`], and the next row carries the
//! rows held back since the last one in `suppressed`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use super::token::AuthError;

/// A (route, reason) pair writes at most one row per this window.
const WINDOW: Duration = Duration::from_secs(60);

/// Per (route, reason): when its last row was written, and the rows held
/// back since. The keys are two closed sets of literals, so the table stays
/// a handful of entries and holds no address.
#[derive(Debug, Default)]
pub(super) struct Throttle(Mutex<HashMap<(&'static str, &'static str), Slot>>);

#[derive(Debug, Clone, Copy)]
struct Slot {
    written: Instant,
    suppressed: u64,
}

impl Throttle {
    /// `Some(suppressed)` when a row for `(route, reason)` is written at
    /// `now`, `None` when the row is held back and counted.
    fn admit(&self, route: &'static str, reason: &'static str, now: Instant) -> Option<u64> {
        // Released before the caller writes the row: emitting runs the log pipeline.
        let mut slots = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (route, reason);
        if let Some(slot) = slots.get_mut(&key) {
            if now.saturating_duration_since(slot.written) < WINDOW {
                slot.suppressed += 1;
                return None;
            }
            slot.written = now;
            return Some(std::mem::take(&mut slot.suppressed));
        }
        slots.insert(
            key,
            Slot {
                written: now,
                suppressed: 0,
            },
        );
        Some(0)
    }
}

/// The process-wide table behind [`log_refusal`].
fn throttle() -> &'static Throttle {
    static THROTTLE: OnceLock<Throttle> = OnceLock::new();
    THROTTLE.get_or_init(Throttle::default)
}

/// Log one refused request from `route` (`mint` or `refresh`), unless the
/// throttle holds it back.
pub(super) fn log_refusal(route: &'static str, err: &AuthError) {
    log_refusal_in(throttle(), route, err, Instant::now());
}

/// [`log_refusal`] against a given table and clock, so the tests start from
/// an empty table.
pub(super) fn log_refusal_in(
    throttle: &Throttle,
    route: &'static str,
    err: &AuthError,
    now: Instant,
) {
    match err {
        AuthError::QuotaExceeded(q) => {
            let reason = "dev_session_quota_exceeded";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::warn!(
                    route,
                    scope = q.scope,
                    retry_after_secs = q.retry_after_secs,
                    reason,
                    suppressed,
                    "dev-session request refused: {q}"
                );
            }
        }
        // Not throttled: the debug row is never forwarded.
        AuthError::BadInstallId(_) | AuthError::BadField { .. } => tracing::debug!(
            route,
            reason = "dev_session_bad_request",
            "dev-session request refused: {err}"
        ),
        // The operator's misconfiguration, not the caller's: every launcher
        // and lab session is refused until it is fixed, and before this row
        // the only trace of it was a 500 on the caller's side.
        AuthError::SecretMissing | AuthError::SecretTooShort { .. } => {
            let reason = "dev_session_secret_unusable";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::error!(
                    route,
                    reason,
                    suppressed,
                    "dev-session request refused: {err}"
                );
            }
        }
        AuthError::KillSwitchActive => {
            let reason = "dev_session_kill_switch";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::warn!(
                    route,
                    reason,
                    suppressed,
                    "dev-session request refused: {err}"
                );
            }
        }
        AuthError::SessionLifetimeExceeded { elapsed, cap } => {
            let reason = "dev_session_lifetime_exceeded";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::info!(
                    route,
                    elapsed,
                    cap,
                    reason,
                    suppressed,
                    "dev-session request refused: {err}"
                );
            }
        }
        AuthError::Expired { exp, now: clock } => {
            let reason = "dev_session_expired";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::info!(
                    route,
                    exp,
                    now = clock,
                    reason,
                    suppressed,
                    "dev-session request refused: {err}"
                );
            }
        }
        AuthError::BadPayload(_) | AuthError::BadSignature => {
            let reason = "dev_session_bad_token";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::info!(
                    route,
                    reason,
                    suppressed,
                    "dev-session request refused: {err}"
                );
            }
        }
        // LT-01's 503: the launcher is told nothing to upload to. Logged here
        // rather than at the call site, which would write the row twice.
        AuthError::NoUploadEndpoint => {
            let reason = "dev_session_no_upload_endpoint";
            if let Some(suppressed) = throttle.admit(route, reason, now) {
                tracing::warn!(
                    route,
                    reason,
                    suppressed,
                    "dev-session request refused: no https upload endpoint to advertise, \
                     so the launcher cannot upload telemetry"
                );
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;

    use super::super::handlers::{mint_inner, Tables};
    use super::super::tests::{ip, policy, request, EnvGuard, Row, Rows};
    use super::*;

    /// Runs `f` with a fresh capture and returns the rows it emitted.
    fn capture(f: impl FnOnce()) -> Vec<Row> {
        let rows = Rows::default();
        let sub = tracing_subscriber::registry().with(rows.clone());
        tracing::subscriber::with_default(sub, f);
        let captured = rows.0.lock().unwrap().clone();
        captured
    }

    fn reason(row: &Row) -> &str {
        &row.1["reason"]
    }

    #[test]
    fn a_reason_writes_one_row_a_minute_and_counts_the_rest() {
        let t = Throttle::default();
        let t0 = Instant::now();
        let expired = AuthError::Expired { exp: 1, now: 2 };
        let rows = capture(|| {
            log_refusal_in(&t, "mint", &AuthError::KillSwitchActive, t0);
            log_refusal_in(
                &t,
                "mint",
                &AuthError::KillSwitchActive,
                t0 + Duration::from_secs(1),
            );
            // The same reason on another route, and another reason, each get a row.
            log_refusal_in(
                &t,
                "refresh",
                &AuthError::KillSwitchActive,
                t0 + Duration::from_secs(2),
            );
            log_refusal_in(&t, "mint", &expired, t0 + Duration::from_secs(3));
            // Past the window: the next row carries the one held back.
            log_refusal_in(
                &t,
                "mint",
                &AuthError::KillSwitchActive,
                t0 + Duration::from_secs(61),
            );
        });
        let reasons: Vec<&str> = rows.iter().map(reason).collect();
        assert_eq!(
            reasons,
            [
                "dev_session_kill_switch",
                "dev_session_kill_switch",
                "dev_session_expired",
                "dev_session_kill_switch",
            ],
            "{rows:#?}"
        );
        assert_eq!(rows[3].1["suppressed"], "1", "{rows:#?}");
        assert_eq!(rows[0].1["suppressed"], "0", "{rows:#?}");
        assert!(rows
            .iter()
            .all(|r| r.0 == Level::WARN || r.0 == Level::INFO));
    }

    /// The real mint path, refused by its per-address quota: one WARN with
    /// the reason, no `peer` key, and the address appears nowhere in the row.
    /// Reverted to a row that carries the peer, this fails.
    #[test]
    fn a_refused_mint_names_no_peer_address() {
        let _g = EnvGuard::install();
        let t = Tables::new();
        let p = policy(1, 100, 10);
        let now = Instant::now();
        let now_unix = chrono::Utc::now().timestamp();
        let attacker = ip("198.51.100.7");
        mint_inner(&t, &p, attacker, request("first"), now, now_unix).unwrap();
        let rows = capture(|| {
            let err = mint_inner(&t, &p, attacker, request("second"), now, now_unix).unwrap_err();
            log_refusal_in(&Throttle::default(), "mint", &err, now);
        });
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert_eq!(rows[0].0, Level::WARN);
        assert_eq!(reason(&rows[0]), "dev_session_quota_exceeded");
        assert!(!rows[0].1.contains_key("peer"), "{rows:#?}");
        assert!(!format!("{rows:?}").contains("198.51.100.7"), "{rows:#?}");
    }

    /// No upload endpoint configured and no Host: LT-01's 503 writes one
    /// WARN with no peer.
    #[test]
    fn a_mint_with_no_upload_endpoint_writes_one_row_without_a_peer() {
        let _g = EnvGuard::install();
        std::env::remove_var("CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT");
        let t = Tables::new();
        let p = policy(10, 10, 10);
        let peer = ip("203.0.113.7");
        let rows = capture(|| {
            let err = mint_inner(
                &t,
                &p,
                peer,
                request("no-endpoint"),
                Instant::now(),
                chrono::Utc::now().timestamp(),
            )
            .unwrap_err();
            assert!(matches!(err, AuthError::NoUploadEndpoint), "got {err:?}");
            log_refusal_in(&Throttle::default(), "mint", &err, Instant::now());
        });
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert_eq!(rows[0].0, Level::WARN);
        assert_eq!(reason(&rows[0]), "dev_session_no_upload_endpoint");
        assert!(!rows[0].1.contains_key("peer"), "{rows:#?}");
        assert!(!format!("{rows:?}").contains("203.0.113.7"), "{rows:#?}");
    }
}
