//! Handler-level tests for mint/refresh: quota enforcement, the
//! bounded refresh chain, and the kill switch.

use std::net::IpAddr;
use std::time::{Duration, Instant};

use axum::response::IntoResponse;

use super::handlers::{
    kill_switch_active, mint_inner, refresh_inner, DevSessionRequest, QuotaPolicy, Tables,
    TOKEN_TTL_SECONDS,
};
use super::token::{decode_token, encode_token, env_lock, AuthError, TokenClaims};
use super::SCOPE_TELEMETRY_WRITE;

const NOW_UNIX: i64 = 1_700_000_000;

/// 64 bytes of hex — `load_secret` reads this via the env var the
/// handlers use, so the tests drive the real loader rather than a
/// parallel one.
const TEST_SECRET_HEX: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90\
a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    prev_secret: Option<String>,
    prev_kill: Option<String>,
}

impl EnvGuard {
    /// Installs a usable HMAC secret and clears the kill switch, then
    /// restores both on drop — the env is process-wide and shared with
    /// the telemetry tests.
    fn install() -> Self {
        let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev_secret = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        let prev_kill = std::env::var("CIMMERIA_TELEMETRY_KILL_SWITCH").ok();
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", TEST_SECRET_HEX);
        std::env::remove_var("CIMMERIA_TELEMETRY_KILL_SWITCH");
        Self {
            _lock: lock,
            prev_secret,
            prev_kill,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prev_secret.take() {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
        match self.prev_kill.take() {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_KILL_SWITCH"),
        }
    }
}

fn policy(mint_per_ip: u32, mint_per_install: u32, refresh_per_ip: u32) -> QuotaPolicy {
    QuotaPolicy {
        window: Duration::from_secs(3_600),
        mint_per_ip,
        mint_per_install,
        refresh_per_ip,
        max_session_secs: 24 * 60 * 60,
    }
}

fn request(install_id: &str) -> DevSessionRequest {
    DevSessionRequest {
        install_id: install_id.into(),
        machine_id: "machine-abc".into(),
        branch: "main".into(),
        git_sha: "0123456".into(),
        launcher_version: "0.1.0".into(),
        tags: Vec::new(),
    }
}

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

fn secret() -> Vec<u8> {
    super::token::load_secret().unwrap()
}

fn status(err: AuthError) -> axum::http::StatusCode {
    err.into_response().status()
}

fn retry_after(err: AuthError) -> Option<String> {
    err.into_response()
        .headers()
        .get(axum::http::header::RETRY_AFTER)
        .map(|v| v.to_str().unwrap().to_string())
}

// A launcher minting once per launch is unaffected: the happy path
// still returns a token carrying exactly the telemetry.write scope,
// an 8h expiry, and iat == mint time.
#[test]
fn mint_issues_a_scoped_eight_hour_token() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let resp = mint_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(resp.expires_at_ms, (NOW_UNIX + TOKEN_TTL_SECONDS) * 1000);
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(claims.iat, NOW_UNIX);
    assert_eq!(claims.exp, NOW_UNIX + TOKEN_TTL_SECONDS);
    assert_eq!(claims.scope, vec![SCOPE_TELEMETRY_WRITE.to_string()]);
}

// The open mint surface: one address minting in a loop is refused
// once it passes the per-IP allowance, with a 429 and a Retry-After
// the launcher's back-off path already honours.
#[test]
fn mint_refuses_one_address_past_the_per_ip_quota() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(2, 100, 10);
    let now = Instant::now();
    let attacker = ip("198.51.100.9");
    for i in 0..2 {
        mint_inner(&t, &p, attacker, request(&format!("id-{i}")), now, NOW_UNIX)
            .unwrap_or_else(|e| panic!("mint {i} should pass, got {e}"));
    }
    let err = mint_inner(&t, &p, attacker, request("id-2"), now, NOW_UNIX).unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));
    assert_eq!(retry_after(err).as_deref(), Some("3601"));
    let err = mint_inner(&t, &p, attacker, request("id-3"), now, NOW_UNIX).unwrap_err();
    assert_eq!(status(err), axum::http::StatusCode::TOO_MANY_REQUESTS);
}

// Rotating install_id does not buy more mints, because the per-IP
// counter is charged before `install_id` is validated. This is the
// property that makes the per-IP quota the load-bearing control.
#[test]
fn rotating_install_id_does_not_evade_the_per_ip_quota() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(1, 100, 10);
    let now = Instant::now();
    let attacker = ip("198.51.100.9");
    mint_inner(&t, &p, attacker, request("first-id"), now, NOW_UNIX).unwrap();
    let err = mint_inner(&t, &p, attacker, request("second-id"), now, NOW_UNIX).unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));
}

// One address exhausting its allowance must not refuse a different
// developer — the defence has to fall on the flooder alone.
#[test]
fn one_exhausted_address_does_not_refuse_another() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(1, 100, 10);
    let now = Instant::now();
    mint_inner(&t, &p, ip("198.51.100.9"), request("a"), now, NOW_UNIX).unwrap();
    assert!(mint_inner(&t, &p, ip("198.51.100.9"), request("a"), now, NOW_UNIX).is_err());
    mint_inner(&t, &p, ip("203.0.113.7"), request("b"), now, NOW_UNIX)
        .expect("a different peer address keeps its own allowance");
}

// The per-install_id quota catches one machine relaunching in a loop
// even when it moves between addresses.
#[test]
fn mint_refuses_one_install_id_past_its_quota_across_addresses() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(100, 2, 10);
    let now = Instant::now();
    mint_inner(&t, &p, ip("203.0.113.1"), request("looping"), now, NOW_UNIX).unwrap();
    mint_inner(&t, &p, ip("203.0.113.2"), request("looping"), now, NOW_UNIX).unwrap();
    let err = mint_inner(&t, &p, ip("203.0.113.3"), request("looping"), now, NOW_UNIX).unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));
}

// An install_id that the launcher could never have produced is
// refused with 400 before it can reach a token claim or a log field.
#[test]
fn mint_rejects_an_install_id_the_launcher_could_not_produce() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(100, 100, 10);
    let now = Instant::now();
    let err = mint_inner(
        &t,
        &p,
        ip("203.0.113.1"),
        request("evil\nlevel=error msg=fake"),
        now,
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::BadInstallId(_)));
    assert_eq!(status(err), axum::http::StatusCode::BAD_REQUEST);

    let err = mint_inner(
        &t,
        &p,
        ip("203.0.113.1"),
        request(&"a".repeat(129)),
        now,
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::BadInstallId(_)));
}

// The kill switch still wins over everything, including the quota.
#[test]
fn kill_switch_refuses_mint_before_any_quota_is_charged() {
    let _g = EnvGuard::install();
    std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "1");
    let t = Tables::new();
    let err = mint_inner(
        &t,
        &policy(0, 0, 0),
        ip("203.0.113.1"),
        request("abc"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::KillSwitchActive));
    assert_eq!(status(err), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    std::env::remove_var("CIMMERIA_TELEMETRY_KILL_SWITCH");
}

// Only the literal `1` enables the kill switch — the contract
// docs/operations/telemetry.md states. Loosening it to accept
// `true`/`yes` would silently turn an operator's `=true` typo from
// "off" into "telemetry dark".
#[test]
fn kill_switch_is_on_only_for_the_literal_one() {
    let _g = EnvGuard::install();
    assert!(!kill_switch_active(), "unset is off");
    for off in ["0", "true", "yes", "", " 1"] {
        std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", off);
        assert!(!kill_switch_active(), "{off:?} must not enable the switch");
    }
    std::env::set_var("CIMMERIA_TELEMETRY_KILL_SWITCH", "1");
    assert!(kill_switch_active());
}

fn token_minted_at(iat: i64, exp: i64) -> String {
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-abc".into(),
        sid: "session-123".into(),
        iat,
        exp,
        scope: vec![SCOPE_TELEMETRY_WRITE.into()],
    };
    encode_token(&claims, &secret()).unwrap()
}

// A normal mid-session refresh still works and still hands back a
// full 8h token — the cap must not disturb the ordinary path.
#[test]
fn refresh_extends_a_live_token_and_preserves_the_original_iat() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let minted_at = NOW_UNIX;
    let token = token_minted_at(minted_at, minted_at + TOKEN_TTL_SECONDS);
    let six_hours_later = minted_at + 6 * 3_600;
    let resp = refresh_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        &token,
        Instant::now(),
        six_hours_later,
    )
    .unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(
        claims.iat, minted_at,
        "iat must keep pointing at the original mint, or the chain is unbounded"
    );
    assert_eq!(claims.exp, six_hours_later + TOKEN_TTL_SECONDS);
    assert_eq!(claims.sid, "session-123");
}

// The bug this guards: before the cap, every refresh reset `iat` and
// granted a fresh full TTL, so one leaked token could be walked
// forward indefinitely. No token issued by any link in the chain may
// expire past the original mint plus the cap.
#[test]
fn chained_refreshes_cannot_push_expiry_past_the_session_cap() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 1_000);
    let minted_at = NOW_UNIX;
    let deadline = minted_at + p.max_session_secs;
    let mut token = token_minted_at(minted_at, minted_at + TOKEN_TTL_SECONDS);
    // Walk forward in 6h steps, refreshing each time, as the holder of
    // a leaked token would.
    for step in 1..=3 {
        let now_unix = minted_at + step * 6 * 3_600;
        let resp = refresh_inner(&t, &p, ip("203.0.113.5"), &token, Instant::now(), now_unix)
            .unwrap_or_else(|e| panic!("refresh at t+{}h should work: {e}", step * 6));
        let claims = decode_token(&resp.token, &secret()).unwrap();
        assert_eq!(
            claims.iat, minted_at,
            "iat must keep pointing at the original mint, or the chain is unbounded"
        );
        assert!(
            claims.exp <= deadline,
            "refresh at t+{}h issued a token expiring {}s past the cap",
            step * 6,
            claims.exp - deadline
        );
        token = resp.token;
    }
    assert_eq!(
        decode_token(&token, &secret()).unwrap().exp,
        deadline,
        "the last refresh inside the cap must expire exactly at it"
    );
    // At the deadline the chain stops, and says so specifically rather
    // than as a generic expiry.
    let err =
        refresh_inner(&t, &p, ip("203.0.113.5"), &token, Instant::now(), deadline).unwrap_err();
    match err {
        AuthError::SessionLifetimeExceeded { elapsed, cap } => {
            assert_eq!(elapsed, p.max_session_secs);
            assert_eq!(cap, p.max_session_secs);
        }
        other => panic!("expected SessionLifetimeExceeded, got {other:?}"),
    }
}

// A token minted before the cap existed carries an `exp` beyond
// `iat + cap`; it must be refused at the cap rather than honoured to
// its own expiry.
#[test]
fn a_token_predating_the_cap_is_refused_at_the_cap() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 10);
    let minted_at = NOW_UNIX;
    let token = token_minted_at(minted_at, minted_at + p.max_session_secs + 8 * 3_600);
    let err = refresh_inner(
        &t,
        &p,
        ip("203.0.113.5"),
        &token,
        Instant::now(),
        minted_at + p.max_session_secs + 3_600,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::SessionLifetimeExceeded { .. }));
    assert_eq!(status(err), axum::http::StatusCode::UNAUTHORIZED);
}

// Approaching the cap, the new expiry is clamped to the session
// deadline instead of overshooting it — otherwise the final refresh
// would hand out a token valid long past the cap.
#[test]
fn refresh_clamps_the_new_expiry_to_the_session_deadline() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 10);
    let minted_at = NOW_UNIX;
    let token = token_minted_at(minted_at, minted_at + 24 * 3_600);
    // One hour before the cap: a full TTL would reach t+32h.
    let now_unix = minted_at + p.max_session_secs - 3_600;
    let resp = refresh_inner(&t, &p, ip("203.0.113.5"), &token, Instant::now(), now_unix).unwrap();
    let claims = decode_token(&resp.token, &secret()).unwrap();
    assert_eq!(
        claims.exp,
        minted_at + p.max_session_secs,
        "the last refresh must expire at the cap, not TTL past it"
    );
    assert_eq!(resp.expires_at_ms, (minted_at + p.max_session_secs) * 1000);
}

// An already-expired token is not refreshable; the launcher mints a
// fresh session on that 401 instead.
#[test]
fn refresh_refuses_an_expired_token() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let token = token_minted_at(NOW_UNIX, NOW_UNIX + 10);
    let err = refresh_inner(
        &t,
        &policy(10, 10, 10),
        ip("203.0.113.5"),
        &token,
        Instant::now(),
        NOW_UNIX + 11,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::Expired { .. }));
    assert_eq!(status(err), axum::http::StatusCode::UNAUTHORIZED);
}

// Refresh is quota-limited on its own counter, so a token holder
// cannot spin the refresh endpoint freely.
#[test]
fn refresh_is_quota_limited_per_address() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 1);
    let token = token_minted_at(NOW_UNIX, NOW_UNIX + TOKEN_TTL_SECONDS);
    let now = Instant::now();
    refresh_inner(&t, &p, ip("198.51.100.9"), &token, now, NOW_UNIX).unwrap();
    let err = refresh_inner(&t, &p, ip("198.51.100.9"), &token, now, NOW_UNIX).unwrap_err();
    assert!(matches!(err, AuthError::QuotaExceeded(_)));
    assert_eq!(status(err), axum::http::StatusCode::TOO_MANY_REQUESTS);
}

// The mint and refresh counters are separate: exhausting refresh must
// not also lock the caller out of minting a fresh session, which is
// exactly what the launcher does next on a refresh failure.
#[test]
fn exhausted_refresh_quota_leaves_mint_available() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 1);
    let token = token_minted_at(NOW_UNIX, NOW_UNIX + TOKEN_TTL_SECONDS);
    let now = Instant::now();
    let peer = ip("198.51.100.9");
    refresh_inner(&t, &p, peer, &token, now, NOW_UNIX).unwrap();
    assert!(refresh_inner(&t, &p, peer, &token, now, NOW_UNIX).is_err());
    mint_inner(&t, &p, peer, request("recovering"), now, NOW_UNIX)
        .expect("the launcher's mint-a-fresh-session fallback must stay open");
}
