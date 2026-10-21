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
pub(crate) struct Throttle(Mutex<HashMap<(&'static str, &'static str), Slot>>);

#[derive(Debug, Clone, Copy)]
struct Slot {
    written: Instant,
    suppressed: u64,
}

impl Throttle {
    /// `Some(suppressed)` when a row for `(route, reason)` is written at
    /// `now`, `None` when the row is held back and counted. The launcher
    /// summary route shares this table under its own route name.
    pub(crate) fn admit(
        &self,
        route: &'static str,
        reason: &'static str,
        now: Instant,
    ) -> Option<u64> {
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
            // Keyed on the scope, so one flooded budget cannot hold back the
            // first row of another scope in the same window.
            let reason = "dev_session_quota_exceeded";
            if let Some(suppressed) = throttle.admit(route, q.scope, now) {
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
                // `BadPayload` holds caller-supplied text (base64 error bytes),
                // so the row names the reason and not the error.
                tracing::info!(
                    route,
                    reason,
                    suppressed,
                    "dev-session request refused: the token was rejected"
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

    use axum::extract::ConnectInfo;

    use super::super::handlers::{mint, mint_inner, Tables};
    use super::super::quota::QuotaExceeded;
    use super::super::tests::{capture, ip, policy, request, EnvGuard, Row};
    use super::*;

    fn reason(row: &Row) -> &str {
        &row.1["reason"]
    }

    #[test]
    fn a_reason_writes_one_row_a_minute_and_counts_the_rest() {
        let t = Throttle::default();
        let t0 = Instant::now();
        let expired = AuthError::Expired { exp: 1, now: 2 };
        let ((), rows) = capture(|| {
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

    /// Two scopes in one window each write their first row; the same scope
    /// twice writes one, and the next window carries the held-back count.
    /// Keyed on the reason string instead, the `mint/install` row is lost.
    #[test]
    fn each_quota_scope_has_its_own_first_row() {
        let t = Throttle::default();
        let t0 = Instant::now();
        let quota = |scope: &'static str| {
            AuthError::QuotaExceeded(QuotaExceeded {
                scope,
                retry_after_secs: 5,
            })
        };
        let ((), rows) = capture(|| {
            log_refusal_in(&t, "mint", &quota("mint/ip"), t0);
            log_refusal_in(
                &t,
                "mint",
                &quota("mint/install"),
                t0 + Duration::from_secs(1),
            );
            log_refusal_in(&t, "mint", &quota("mint/ip"), t0 + Duration::from_secs(2));
            log_refusal_in(&t, "mint", &quota("mint/ip"), t0 + Duration::from_secs(61));
        });
        let scopes: Vec<&str> = rows.iter().map(|r| r.1["scope"].as_str()).collect();
        assert_eq!(scopes, ["mint/ip", "mint/install", "mint/ip"], "{rows:#?}");
        assert_eq!(rows[2].1["suppressed"], "1", "{rows:#?}");
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
        let ((), rows) = capture(|| {
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
        let ((), rows) = capture(|| {
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

    /// The real `mint` handler, refused for want of an upload endpoint, from a
    /// known peer: one WARN on route `mint`, no peer key, no address anywhere.
    /// Fails if the `.inspect_err` in `handlers::mint` is dropped.
    #[test]
    fn the_mint_handler_logs_its_refusal_without_the_peer() {
        let _g = EnvGuard::install();
        std::env::remove_var("CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT");
        let peer: std::net::SocketAddr = "203.0.113.8:4000".parse().unwrap();
        let ((), rows) = capture(|| {
            let fut = mint(
                ConnectInfo(peer),
                axum::http::HeaderMap::new(),
                axum::Json(request("handler-no-endpoint")),
            );
            let res = tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(fut);
            assert!(matches!(res, Err(AuthError::NoUploadEndpoint)));
        });
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert_eq!(rows[0].0, Level::WARN);
        assert_eq!(rows[0].1["route"], "mint");
        assert_eq!(reason(&rows[0]), "dev_session_no_upload_endpoint");
        assert!(!rows[0].1.contains_key("peer"), "{rows:#?}");
        assert!(!format!("{rows:?}").contains("203.0.113.8"), "{rows:#?}");
    }

    /// Every arm that writes a row, each through a fresh table: no row has a
    /// `peer` key or a dotted-quad value.
    #[test]
    fn no_refusal_row_carries_a_peer_or_an_address() {
        let errors = [
            AuthError::SecretMissing,
            AuthError::SecretTooShort { got: 1, min: 64 },
            AuthError::KillSwitchActive,
            AuthError::BadPayload("payload base64: Invalid byte 45, offset 3".into()),
            AuthError::BadSignature,
            AuthError::Expired { exp: 1, now: 2 },
            AuthError::SessionLifetimeExceeded { elapsed: 3, cap: 4 },
            AuthError::QuotaExceeded(QuotaExceeded {
                scope: "mint/ip",
                retry_after_secs: 5,
            }),
            AuthError::BadInstallId("too short"),
            AuthError::BadField {
                field: "machine_id",
                reason: "empty",
            },
            AuthError::NoUploadEndpoint,
        ];
        let ((), rows) = capture(|| {
            for err in &errors {
                log_refusal_in(&Throttle::default(), "mint", err, Instant::now());
            }
        });
        assert_eq!(rows.len(), errors.len(), "{rows:#?}");
        for row in &rows {
            assert!(!row.1.contains_key("peer"), "{rows:#?}");
            assert!(!row.1.values().any(|v| looks_like_ipv4(v)), "{rows:#?}");
        }
    }

    /// True when `text` holds a dotted quad such as `203.0.113.8`.
    fn looks_like_ipv4(text: &str) -> bool {
        text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .any(|run| {
                let octets: Vec<&str> = run.split('.').collect();
                octets.len() == 4
                    && octets.iter().all(|o| {
                        !o.is_empty() && o.len() <= 3 && o.bytes().all(|b| b.is_ascii_digit())
                    })
            })
    }
}
