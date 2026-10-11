//! Unit tests for the Phase 1 and Phase 2 handlers in `handlers.rs`.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{ConnectInfo, State};
use axum::http::{header, StatusCode};

use super::handlers::*;
use super::{HandlerState, SessionRecord};

/// **Seed-GM guard.** The dev/seed accounts must be at least GameMaster
/// (accesslevel 2) so they can run GM commands and reach protected shards
/// (the `access_level < 2` gate in `handle_server_selection`). A
/// regression that reverts any seeded account to Moderator (1) — below
/// the GM threshold — trips this. Covers every account the seed file
/// promotes, so a partial regression is caught too. Runs against the CI
/// live DB loaded from `db/database.sql` (which seeds the account table).
#[tokio::test]
async fn live_db_seed_dev_accounts_are_at_least_gamemaster() {
    use crate::test_support::require_db_or_skip;
    let pool = require_db_or_skip!();

    // Every account promoted in db/sgw/Accounts/Seed/account.sql.
    for name in [
        "test",
        "cady",
        "jorsh",
        "cake",
        "lomiada1",
        "nonwo1984",
        "ishido972",
    ] {
        let level: i32 =
            sqlx::query_scalar("SELECT accesslevel FROM account WHERE account_name = $1")
                .bind(name)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("seed account '{name}' must exist: {e}"));
        assert!(
            level >= 2,
            "seed account '{name}' must be >= GameMaster (2) so it can run GM commands; got {level}"
        );
    }
}

#[test]
fn random_hex_length() {
    assert_eq!(random_hex(10).len(), 20); // ticket
    assert_eq!(random_hex(32).len(), 64); // session key
}

#[test]
fn random_alphanumeric_length_and_charset() {
    let sid = random_alphanumeric(40);
    assert_eq!(sid.len(), 40);
    assert!(sid.chars().all(|c| c.is_ascii_alphanumeric()));
}

/// Build a minimal `HandlerState` for driving `handle_user_auth` through
/// the Router without a DB or event channel. `developer_mode` is true and
/// no database is configured, so the credential check short-circuits —
/// but the plaintext-over-HTTP gate runs *before* that, which is exactly
/// what the test below pins.
fn test_handler_state() -> Arc<HandlerState> {
    test_handler_state_with(true, false)
}

fn test_handler_state_with(developer_mode: bool, db_configured: bool) -> Arc<HandlerState> {
    let shards = vec![super::ShardInfo {
        name: "TestShard".into(),
        host: "127.0.0.1".into(),
        port: 32832,
        protected: false,
    }];
    test_handler_state_shards(developer_mode, db_configured, shards)
}

fn test_handler_state_shards(
    developer_mode: bool,
    db_configured: bool,
    shards: Vec<super::ShardInfo>,
) -> Arc<HandlerState> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    Arc::new(HandlerState {
        shards,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
        developer_mode,
        db_configured,
        db: None,
        login_tx: None,
        login_buffer: None,
    })
}

/// **TLS-gate guard.** A plaintext password (not 40-char hex) offered over
/// the plain-HTTP listener — i.e. with no `TlsConn` extension present —
/// must be rejected with a login error, never accepted. This drives the
/// real `handle_user_auth` (in developer_mode, so no DB) through the
/// Router *without* the `tls_marker_layer`, mirroring the production
/// plain-HTTP path. Reverting the `over_tls` gate in the handler (or in
/// `classify_credential`) would let this plaintext through to a success
/// response and trip this guard.
#[tokio::test]
async fn plaintext_password_over_plain_http_is_rejected() {
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use axum::Router;
    use std::net::SocketAddr;
    use tower::ServiceExt; // oneshot

    // No tls_marker_layer here — this is the plain-HTTP Router shape.
    let app = Router::new()
        .route("/SGWLogin/UserAuth", post(handle_user_auth))
        .with_state(test_handler_state());

    // A plaintext password (well-formed request otherwise). It is NOT a
    // 40-char hex string, so it classifies as plaintext.
    let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="someuser" Password="my-plaintext-password" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;

    let request = Request::builder()
        .method("POST")
        .uri("/SGWLogin/UserAuth")
        .header("Content-Type", "text/xml")
        // Drive connect-info so ConnectInfo<SocketAddr> resolves.
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:5555".parse::<SocketAddr>().unwrap(),
        ))
        .body(Body::from(body))
        .unwrap();

    let response = app.oneshot(request).await.expect("request");
    assert_eq!(response.status(), StatusCode::OK, "auth always returns 200");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let xml = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        xml.contains("SGWLoginError"),
        "plaintext over plain HTTP must produce a login error, got: {xml}"
    );
    assert!(
        !xml.contains("SGWLoginSuccess"),
        "plaintext over plain HTTP must NOT succeed, got: {xml}"
    );
}

/// POST a well-formed Phase 1 request (legacy SHA-1 hex credential, the
/// stock protocol digest) to a plain-HTTP Router over `state`; returns
/// the `Set-Cookie` header (if any) and the response body.
async fn post_user_auth(state: Arc<HandlerState>) -> (Option<String>, String) {
    let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="anyone" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
    post_user_auth_with(state, body, false).await
}

/// POST `body` to a Router over `state`, from 127.0.0.1, with or without
/// the TLS marker extension. Returns the `Set-Cookie` header and body.
async fn post_user_auth_with(
    state: Arc<HandlerState>,
    body: &str,
    over_tls: bool,
) -> (Option<String>, String) {
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt; // oneshot

    let app = Router::new()
        .route("/SGWLogin/UserAuth", post(handle_user_auth))
        .with_state(state);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/SGWLogin/UserAuth")
        .header("Content-Type", "text/xml")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:5555".parse::<SocketAddr>().unwrap(),
        ));
    if over_tls {
        builder = builder.extension(super::TlsConn);
    }
    let request = builder.body(Body::from(body.to_string())).unwrap();
    let response = app.oneshot(request).await.expect("request");
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (cookie, String::from_utf8(bytes.to_vec()).unwrap())
}

/// **Fail-closed guard.** Developer mode with a database *configured*
/// but no pool must refuse the login with the database error, never accept
/// it as account 1 / access level 99. Reverting the `!db_configured`
/// condition on the developer fallback turns this into a success.
#[tokio::test]
async fn dev_mode_with_configured_db_and_no_pool_refuses_login() {
    let capture = crate::test_support::LogCapture::install();
    let state = test_handler_state_with(true, true);
    let sessions = Arc::clone(&state.sessions);

    let (cookie, xml) = post_user_auth(state).await;

    assert!(
        xml.contains("SGWLoginError") && !xml.contains("SGWLoginSuccess"),
        "a configured database with no pool must refuse the login, got: {xml}"
    );
    assert!(
        xml.contains("database server failed"),
        "refusal must be the database error (code 10), got: {xml}"
    );
    assert!(cookie.is_none(), "a refused login must not set a SID");
    assert!(
        sessions.lock().unwrap().is_empty(),
        "a refused login must not create a session"
    );
    let event = capture
        .find_event(tracing::Level::ERROR, "Login refused", "db_pool_missing")
        .expect("refusal must log reason=db_pool_missing at ERROR");
    // Rename pin: the row names the account as `account_name`, never `user`.
    assert_eq!(
        event.fields.get("account_name").map(String::as_str),
        Some("anyone")
    );
    assert!(
        !event.fields.contains_key("user"),
        "the refusal row must not carry a `user` field"
    );
}

/// Developer mode off and no database at all: refused as well.
#[tokio::test]
async fn no_dev_mode_and_no_database_refuses_login() {
    let (cookie, xml) = post_user_auth(test_handler_state_with(false, false)).await;
    assert!(xml.contains("SGWLoginError"), "got: {xml}");
    assert!(cookie.is_none());
}

/// The deliberate developer fallback (developer mode, no database
/// configured) still logs in, and says so at WARN on every acceptance.
#[tokio::test]
async fn dev_mode_without_configured_db_accepts_and_warns() {
    let capture = crate::test_support::LogCapture::install();
    let (cookie, xml) = post_user_auth(test_handler_state_with(true, false)).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    assert!(cookie.is_some_and(|c| c.starts_with("SID=")));
    let event = capture
        .find_event(
            tracing::Level::WARN,
            "accepting credentials unchecked",
            "dev_mode_no_db_login",
        )
        .expect("the developer fallback must log reason=dev_mode_no_db_login at WARN");
    // Rename pin: the row names the account as `account_name`, never `user`.
    assert_eq!(
        event.fields.get("account_name").map(String::as_str),
        Some("anyone")
    );
    assert!(
        !event.fields.contains_key("user"),
        "the developer fallback row must not carry a `user` field"
    );
}

/// **Cross-IP SID guard (#442).** A Phase-1 SID issued to IP A and
/// consumed from IP B is the replay signature of a harvested session
/// token and must log a WARN with `reason = "session_ip_mismatch"`.
/// The SID is still consumed (single-use semantics, warn-only first —
/// see the issue's NAT/dual-stack caveat), so the assertion is the log
/// event, not a rejection. Reverting the check in
/// `handle_server_selection` removes the event and trips this guard.
#[tokio::test]
async fn phase2_sid_consumed_from_different_ip_logs_mismatch() {
    use axum::http::{header, HeaderMap};
    use std::net::SocketAddr;

    let capture = crate::test_support::LogCapture::install();
    let state = test_handler_state();

    // Seed a Phase-1 record issued to 198.51.100.10.
    let sid = random_alphanumeric(40);
    state.sessions.lock().unwrap().insert(
        sid.clone(),
        super::SessionRecord {
            account_id: 0x4420_0001,
            access_level: 0,
            test_account: false,
            account_name: "ipcheck".to_string(),
            client_ip: "198.51.100.10".parse().unwrap(),
            created: std::time::Instant::now(),
        },
    );

    // Phase 2 from a different IP (203.0.113.20) with the SID cookie.
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, format!("SID={sid}").parse().unwrap());
    let state = Arc::clone(&state);
    let addr: SocketAddr = "203.0.113.20:56789".parse().unwrap();
    handle_server_selection(
        State(state),
        ConnectInfo(addr),
        headers,
        "<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin=\"http://www.stargateworlds.com/xml/sgwlogin\" ServerSelection=\"TestShard\" />".to_string(),
    )
    .await;

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "consumed from a different IP",
            "session_ip_mismatch",
        )
        .expect("cross-IP SID consumption must log reason=session_ip_mismatch at WARN");
    assert_eq!(
        warn.fields.get("session_ip").map(String::as_str),
        Some("198.51.100.10")
    );
    assert_eq!(
        warn.fields.get("client_ip").map(String::as_str),
        Some("203.0.113.20")
    );
}

/// Phase 1 body for `sku` and `password` with the stock protocol digest.
fn phase1_body(sku: &str, password: &str) -> String {
    format!(
        r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="{sku}" AccountName="anyone" Password="{password}" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#
    )
}

/// Insert a Phase-1 SID record for `account_id`, issued at `created`.
fn seed_session(state: &HandlerState, sid: &str, account_id: u32, created: Instant) {
    state.sessions.lock().unwrap().insert(
        sid.to_string(),
        SessionRecord {
            account_id,
            access_level: 0,
            test_account: false,
            account_name: "refusal-check".to_string(),
            client_ip: "127.0.0.1".parse().unwrap(),
            created,
        },
    );
}

/// Drive `handle_server_selection` from 127.0.0.1 with an optional SID
/// cookie, selecting `shard`. The response is not inspected: the tests
/// assert on the refusal row.
async fn post_server_selection(state: Arc<HandlerState>, sid: Option<&str>, shard: &str) {
    use axum::http::{header, HeaderMap};
    let mut headers = HeaderMap::new();
    if let Some(sid) = sid {
        headers.insert(header::COOKIE, format!("SID={sid}").parse().unwrap());
    }
    let addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
    let body = format!(
        "<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin=\"http://www.stargateworlds.com/xml/sgwlogin\" ServerSelection=\"{shard}\" />"
    );
    handle_server_selection(State(state), ConnectInfo(addr), headers, body).await;
}

/// **Refusal-reason guard.** An unknown SKU is refused with a login error
/// and an INFO row carrying `reason = "unknown_sku"` and the peer IP.
/// Reverting the log line removes the row and trips this test.
#[tokio::test]
async fn refusal_unknown_sku_logs_reason() {
    let capture = crate::test_support::LogCapture::install();
    let body = phase1_body("SGW_OTHER", "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3");
    let (cookie, xml) = post_user_auth_with(test_handler_state(), &body, false).await;
    assert!(
        xml.contains("SGWLoginError") && cookie.is_none(),
        "got: {xml}"
    );
    let event = capture
        .find_event(tracing::Level::INFO, "unknown service SKU", "unknown_sku")
        .expect("unknown SKU must log reason=unknown_sku at INFO");
    assert_eq!(
        event.fields.get("peer_ip").map(String::as_str),
        Some("127.0.0.1")
    );
}

/// **Refusal-reason guard.** An over-long plaintext password over TLS is
/// refused with `reason = "plaintext_length"`, and the password itself
/// never reaches any captured row.
#[tokio::test]
async fn refusal_plaintext_length_logs_reason_without_password() {
    let capture = crate::test_support::LogCapture::install();
    let password = "x".repeat(129); // MAX_PLAINTEXT_LEN is 128
    let body = phase1_body("SGW_BETA", &password);
    let (cookie, xml) = post_user_auth_with(test_handler_state(), &body, true).await;
    assert!(
        xml.contains("SGWLoginError") && cookie.is_none(),
        "got: {xml}"
    );
    capture
        .find_event(
            tracing::Level::INFO,
            "password length is outside",
            "plaintext_length",
        )
        .expect("over-long plaintext must log reason=plaintext_length at INFO");
    let leaked = capture.all().iter().any(|c| {
        c.message.as_deref().unwrap_or("").contains(&password)
            || c.fields.values().any(|v| v.contains(&password))
    });
    assert!(!leaked, "the plaintext password must never be logged");
}

/// **Refusal-reason guard.** With no shards registered, a valid login is
/// refused at WARN with `reason = "no_shards"` and the account pair.
#[tokio::test]
async fn refusal_no_shards_logs_reason_with_account() {
    let capture = crate::test_support::LogCapture::install();
    let state = test_handler_state_shards(true, false, Vec::new());
    let (cookie, xml) = post_user_auth(state).await;
    assert!(
        xml.contains("SGWLoginError") && cookie.is_none(),
        "got: {xml}"
    );
    let event = capture
        .find_event(tracing::Level::WARN, "no shards are available", "no_shards")
        .expect("no shards must log reason=no_shards at WARN");
    assert_eq!(
        event.fields.get("account_id").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        event.fields.get("account_name").map(String::as_str),
        Some("anyone")
    );
}

/// **Refusal-reason guard.** Phase 2 with no SID cookie logs
/// `reason = "sid_missing"` at INFO.
#[tokio::test]
async fn refusal_sid_missing_logs_reason() {
    let capture = crate::test_support::LogCapture::install();
    post_server_selection(test_handler_state(), None, "TestShard").await;
    let event = capture
        .find_event(tracing::Level::INFO, "no SID cookie", "sid_missing")
        .expect("missing SID must log reason=sid_missing at INFO");
    assert_eq!(
        event.fields.get("peer_ip").map(String::as_str),
        Some("127.0.0.1")
    );
}

/// **Refusal-reason guard.** A SID that was never issued (or was already
/// consumed) logs `reason = "sid_unknown"`, and the SID is not logged.
#[tokio::test]
async fn refusal_sid_unknown_logs_reason_without_sid() {
    let capture = crate::test_support::LogCapture::install();
    let sid = random_alphanumeric(40);
    post_server_selection(test_handler_state(), Some(sid.as_str()), "TestShard").await;
    capture
        .find_event(
            tracing::Level::INFO,
            "not issued or already consumed",
            "sid_unknown",
        )
        .expect("unknown SID must log reason=sid_unknown at INFO");
    let leaked = capture
        .all()
        .iter()
        .any(|c| c.fields.values().any(|v| v.contains(&sid)));
    assert!(!leaked, "the SID must never be logged");
}

/// **Refusal-reason guard.** A SID older than `SESSION_TTL` logs
/// `reason = "sid_expired"` with the account pair and `sid_age_secs`.
#[tokio::test]
async fn refusal_sid_expired_logs_reason_with_account_and_age() {
    let capture = crate::test_support::LogCapture::install();
    let state = test_handler_state();
    let sid = random_alphanumeric(40);
    // The handler reads the clock itself (`created.elapsed()`), so the age
    // is set by backdating `created`. On a freshly booted host the process
    // uptime can be shorter than the TTL, and `Instant` cannot go that far
    // back: skip the expiry path there rather than panic.
    let Some(issued) =
        Instant::now().checked_sub(super::SESSION_TTL + std::time::Duration::from_secs(1))
    else {
        eprintln!("skipped: host uptime is shorter than the session TTL");
        return;
    };
    seed_session(&state, &sid, 42, issued);
    post_server_selection(state, Some(sid.as_str()), "TestShard").await;
    let event = capture
        .find_event(tracing::Level::INFO, "SID expired", "sid_expired")
        .expect("expired SID must log reason=sid_expired at INFO");
    assert_eq!(
        event.fields.get("account_id").map(String::as_str),
        Some("42")
    );
    let age: u64 = event
        .fields
        .get("sid_age_secs")
        .and_then(|v| v.parse().ok())
        .expect("sid_age_secs must be numeric");
    assert!(age >= 301, "age must exceed the 300 s TTL, got {age}");
}

/// **Refusal-reason guard.** A valid SID selecting a shard that does not
/// exist logs `reason = "unknown_shard"` with the account pair.
#[tokio::test]
async fn refusal_unknown_shard_logs_reason_with_account() {
    let capture = crate::test_support::LogCapture::install();
    let state = test_handler_state();
    let sid = random_alphanumeric(40);
    seed_session(&state, &sid, 42, Instant::now());
    post_server_selection(state, Some(sid.as_str()), "NoSuchShard").await;
    let event = capture
        .find_event(tracing::Level::INFO, "no such shard", "unknown_shard")
        .expect("unknown shard must log reason=unknown_shard at INFO");
    assert_eq!(
        event.fields.get("account_id").map(String::as_str),
        Some("42")
    );
    assert_eq!(
        event.fields.get("account_name").map(String::as_str),
        Some("refusal-check")
    );
}
