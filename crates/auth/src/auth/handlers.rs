//! HTTP/SOAP handlers for Phase 1 (UserAuth) and Phase 2 (ServerSelection),
//! plus credential validation and random generators. Request parsing is in
//! `soap_request.rs`, response XML in `soap_response.rs`.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use rand::RngExt;

use crate::audit::{emit_login_event, LoginOutcome};
use crate::credential_redaction::CredentialPrefix;

use super::credentials::{
    classify_credential, validate_credentials, AuthCredError, CredentialGateError,
};
use super::soap_request::{parse_login_request, parse_server_selection};
use super::soap_response::{login_error, login_success_xml, select_error, server_location_xml};
use super::{HandlerState, PendingLogin, SessionRecord, TlsConn, PROTOCOL_DIGEST, SESSION_TTL};

// ── Axum handlers ────────────────────────────────────────────────────────────

/// Phase 1: `POST /SGWLogin/UserAuth`
#[tracing::instrument(
    name = "auth.user_auth",
    level = "info",
    skip(state, over_tls, body),
    fields(
        peer = %addr,
        account_name = tracing::field::Empty,
        account_id = tracing::field::Empty,
        result = tracing::field::Empty,
    ),
)]
pub(super) async fn handle_user_auth(
    State(state): State<Arc<HandlerState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    // Present only when the request arrived over the TLS listener (the TLS
    // Router clone carries `tls_marker_layer`). Gates plaintext-password
    // acceptance — plaintext is never honoured over plain HTTP.
    over_tls: Option<Extension<TlsConn>>,
    body: String,
) -> Response {
    let over_tls = over_tls.is_some();
    tracing::debug!(over_tls, "Phase 1: UserAuth");
    tracing::trace!(body_len = body.len(), "Phase 1 raw SOAP request");

    let client_ip = addr.ip().to_string();

    let req = match parse_login_request(&body) {
        Ok(r) => r,
        Err(e) => {
            // `e` names the attribute and the defect class only, never the
            // value, so a malformed password is not echoed into the log.
            tracing::Span::current().record("result", "bad_request");
            tracing::warn!(
                reason = e.reason(),
                attribute = e.attribute(),
                over_tls,
                "Phase 1 SOAP request rejected: {e}"
            );
            return login_error(13, "Internal error.");
        }
    };

    // Helper macro to emit audit events concisely.
    macro_rules! audit {
        ($outcome:expr) => {
            if let (Some(tx), Some(buf)) = (&state.login_tx, &state.login_buffer) {
                emit_login_event(
                    tx,
                    buf,
                    &req.account_name,
                    None,
                    &client_ip,
                    "credential_check",
                    $outcome,
                    None,
                    None,
                );
            }
        };
        ($outcome:expr, id=$id:expr) => {
            if let (Some(tx), Some(buf)) = (&state.login_tx, &state.login_buffer) {
                emit_login_event(
                    tx,
                    buf,
                    &req.account_name,
                    Some($id),
                    &client_ip,
                    "credential_check",
                    $outcome,
                    None,
                    None,
                );
            }
        };
    }

    if req.sku != "SGW_BETA" {
        return login_error(3, "The specified service does not exist.");
    }
    // Validate the account name before it reaches the span or any audit row:
    // it is client-controlled, and after XML decoding a control character or
    // newline can arrive as `&#10;` as well as raw. Log only its length.
    let name_ok = req.account_name.len() >= 3
        && req.account_name.len() <= 20
        && req
            .account_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    if !name_ok {
        tracing::Span::current().record("result", "bad_request");
        tracing::info!(
            reason = "malformed_account_name",
            account_name_len = req.account_name.len(),
            "Phase 1 login rejected: account name fails the format check"
        );
        return login_error(1, "The specified account name is invalid.");
    }
    tracing::Span::current().record("account_name", req.account_name.as_str());
    // Classify the supplied credential: a 40-char hex string is the original
    // client's SHA-1 hash (allowed over HTTP or TLS); anything else is treated
    // as a plaintext password, which is only honoured over TLS.
    let credential = match classify_credential(&req.password, over_tls) {
        Ok(c) => c,
        Err(CredentialGateError::PlaintextRequiresTls) => {
            tracing::warn!(user = %req.account_name, "plaintext credential rejected over plain HTTP");
            audit!(LoginOutcome::PlaintextRequiresTls);
            // Reuse the malformed-password code: the client must not learn that
            // a plaintext-over-TLS path exists.
            return login_error(2, "The specified password is invalid.");
        }
        Err(CredentialGateError::PlaintextLength) => {
            return login_error(2, "The specified password is invalid.");
        }
    };
    if !state.developer_mode && req.protocol_digest.to_uppercase() != PROTOCOL_DIGEST {
        tracing::warn!(got = ?req.protocol_digest, expected = PROTOCOL_DIGEST, "Protocol digest mismatch");
        audit!(LoginOutcome::ProtocolMismatch);
        return login_error(
            17,
            "Protocol version mismatch; your client version is not supported.",
        );
    }

    // Credential check.
    // If DB is available, validate against the account table.
    // Only in developer mode with NO database configured are valid-format
    // credentials accepted without a check. A configured database whose pool
    // is missing refuses every login (fail closed).
    let (account_id, access_level): (u32, u32) = if let Some(ref db) = state.db {
        match validate_credentials(db, &req.account_name, credential).await {
            Ok(acct) => (acct.account_id, acct.access_level),
            Err(AuthCredError::InvalidCredentials) => {
                tracing::info!(user = %req.account_name, "Invalid credentials");
                audit!(LoginOutcome::InvalidCredentials);
                cimmeria_discord::emit_player_auth_failed(
                    req.account_name.as_str(),
                    addr,
                    "invalid_credentials",
                );
                // C++ FailureCode::BadUserPassword = 4 (not 3, which is InvalidService).
                return login_error(4, "The account name or password is incorrect.");
            }
            Err(AuthCredError::AccountDisabled) => {
                tracing::info!(user = %req.account_name, "Account disabled");
                audit!(LoginOutcome::AccountDisabled);
                cimmeria_discord::emit_player_auth_failed(
                    req.account_name.as_str(),
                    addr,
                    "account_disabled",
                );
                return login_error(5, "This account has been suspended.");
            }
            Err(AuthCredError::DbError(e)) => {
                tracing::error!(user = %req.account_name, error = %e, "DB query failed");
                audit!(LoginOutcome::DbError);
                cimmeria_discord::emit_db_error("auth_credential_check", e.to_string());
                return login_error(10, "A request to the database server failed.");
            }
        }
    } else if state.developer_mode && !state.db_configured {
        tracing::warn!(
            user = %req.account_name,
            reason = "dev_mode_no_db_login",
            "Developer mode with no database configured: accepting credentials \
             unchecked as account 1, access level 99"
        );
        (1, 99) // dev mode, no database configured: max access level
    } else {
        // Either a database is configured but no pool is wired (the
        // orchestrator refuses to start in that state, so this is a wiring
        // bug), or there is no database and developer mode is off.
        let reason = if state.db_configured {
            "db_pool_missing"
        } else {
            "no_database"
        };
        tracing::error!(
            user = %req.account_name,
            reason,
            "Login refused: no database connection to check credentials against"
        );
        audit!(LoginOutcome::DbError);
        return login_error(10, "A request to the database server failed.");
    };

    if state.shards.is_empty() {
        audit!(LoginOutcome::NoShards, id = account_id);
        return login_error(7, "No shards are available to the authentication server.");
    }

    // 40-char alphanumeric SID matching the C++ session cookie format.
    let sid = random_alphanumeric(40);
    tracing::debug!(sid_prefix = %CredentialPrefix(&sid), "Phase 1 generated SID");
    {
        state.sessions.lock().unwrap().insert(
            sid.clone(),
            SessionRecord {
                account_id,
                access_level,
                account_name: req.account_name.clone(),
                client_ip: addr.ip(),
                created: Instant::now(),
            },
        );
    }

    tracing::Span::current().record("account_id", account_id);
    tracing::Span::current().record("result", "success");
    tracing::info!(
        account_id,
        account_name = %req.account_name,
        access_level,
        ip = %client_ip,
        "Phase 1 success"
    );
    audit!(LoginOutcome::Success, id = account_id);

    let xml = login_success_xml(account_id, &state.shards);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/xml".to_string()),
            (header::SET_COOKIE, format!("SID={sid}")),
        ],
        xml,
    )
        .into_response()
}

/// Phase 2: `POST /SGWLogin/ServerSelection`
#[tracing::instrument(
    name = "auth.server_selection",
    level = "info",
    skip(state, headers, body),
    fields(
        peer = %addr,
        account_id = tracing::field::Empty,
        shard = tracing::field::Empty,
        result = tracing::field::Empty,
    ),
)]
pub(super) async fn handle_server_selection(
    State(state): State<Arc<HandlerState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: String,
) -> Response {
    tracing::debug!("Phase 2: ServerSelection");
    tracing::trace!(body_len = body.len(), "Phase 2 raw SOAP request");

    let client_ip = addr.ip().to_string();

    let sid = match extract_sid(&headers) {
        Some(s) => s,
        None => return select_error(15, "Your logon session has expired. Please log in again."),
    };

    // Consume session (remove from map) — each SID is single-use for Phase 2.
    let session = { state.sessions.lock().unwrap().remove(&sid) };
    let session = match session {
        Some(s) if s.created.elapsed() < SESSION_TTL => s,
        Some(_) => {
            tracing::info!("Session expired for SID");
            return select_error(15, "Your logon session has expired. Please log in again.");
        }
        None => return select_error(15, "Your logon session has expired. Please log in again."),
    };

    // Cross-IP detection (issue #442, warn-only first): the SID was issued
    // to a specific client IP at Phase 1. Consuming it from a different IP
    // is the replay signature of a harvested session token. WARN (not
    // reject) so NAT/dual-stack false positives are measured before the
    // gate hardens; single-use SID semantics are unchanged.
    if !crate::auth::client_ips_match(session.client_ip, addr.ip()) {
        tracing::warn!(
            account_id = session.account_id,
            account_name = %session.account_name,
            sid_prefix = %CredentialPrefix(&sid),
            session_ip = %session.client_ip,
            client_ip = %addr.ip(),
            reason = "session_ip_mismatch",
            "Phase 2 SID consumed from a different IP than it was issued to — possible stolen session"
        );
    }

    let selected = match parse_server_selection(&body) {
        Ok(s) => s,
        Err(e) => {
            tracing::Span::current().record("result", "bad_request");
            tracing::warn!(
                account_id = session.account_id,
                account_name = %session.account_name,
                reason = e.reason(),
                attribute = e.attribute(),
                "Phase 2 SOAP request rejected: {e}"
            );
            return select_error(13, "Internal error.");
        }
    };

    let shard = match state.shards.iter().find(|s| s.name == selected) {
        Some(s) => s.clone(),
        None => return select_error(8, "No such shard."),
    };

    // Protected shard access control (matches C++ AccessDenied behaviour).
    if shard.protected && session.access_level < 2 {
        tracing::info!(
            account_id = session.account_id,
            account_name = %session.account_name,
            shard = %shard.name,
            access_level = session.access_level,
            "Access denied to protected shard"
        );
        return select_error(6, "Access denied.");
    }

    // 64-char hex AES-256 session key, 20-char hex ticket.
    let session_key = random_hex(32);
    let ticket = random_hex(10);
    tracing::debug!(ticket_prefix = %CredentialPrefix(&ticket), "Phase 2 generated session credentials");

    {
        state.pending_logins.lock().unwrap().insert(
            ticket.clone(),
            PendingLogin {
                account_id: session.account_id,
                account_name: session.account_name.clone(),
                access_level: session.access_level,
                ticket: ticket.clone(),
                session_key: session_key.clone(),
                client_ip: addr.ip(),
                created: Instant::now(),
            },
        );
    }

    tracing::Span::current().record("account_id", session.account_id);
    tracing::Span::current().record("shard", shard.name.as_str());
    tracing::Span::current().record("result", "success");
    tracing::info!(
        account_id = session.account_id,
        account_name = %session.account_name,
        shard = %shard.name,
        ip = %client_ip,
        ticket_prefix = %CredentialPrefix(&ticket),
        "Phase 2 success — ticket issued"
    );

    if let (Some(tx), Some(buf)) = (&state.login_tx, &state.login_buffer) {
        emit_login_event(
            tx,
            buf,
            &session.account_name,
            Some(session.account_id),
            &client_ip,
            "shard_selection",
            LoginOutcome::Success,
            Some(&shard.name),
            None,
        );
    }

    let xml = server_location_xml(&shard, &session_key, &ticket);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

// ── XML helpers ──────────────────────────────────────────────────────────────

fn extract_sid(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie
        .split(';')
        .map(str::trim)
        .find(|s| s.starts_with("SID="))
        .map(|s| s["SID=".len()..].to_string())
}

/// Generate `byte_count` random bytes as uppercase hex.
pub(super) fn random_hex(byte_count: usize) -> String {
    let mut rng = rand::rng();
    (0..byte_count)
        .map(|_| format!("{:02X}", rng.random::<u8>()))
        .collect()
}

/// Generate a random alphanumeric string of the given character length.
///
/// Matches the C++ session ID format: 40-char string drawn from [0-9a-zA-Z].
pub(super) fn random_alphanumeric(char_count: usize) -> String {
    const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::rng();
    (0..char_count)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        use std::collections::HashMap;
        use std::sync::Mutex;
        Arc::new(HandlerState {
            shards: vec![super::super::ShardInfo {
                name: "TestShard".into(),
                host: "127.0.0.1".into(),
                port: 32832,
                protected: false,
            }],
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
        use axum::body::Body;
        use axum::http::Request;
        use axum::routing::post;
        use axum::Router;
        use tower::ServiceExt; // oneshot

        let app = Router::new()
            .route("/SGWLogin/UserAuth", post(handle_user_auth))
            .with_state(state);
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="anyone" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let request = Request::builder()
            .method("POST")
            .uri("/SGWLogin/UserAuth")
            .header("Content-Type", "text/xml")
            .extension(axum::extract::ConnectInfo(
                "127.0.0.1:5555".parse::<SocketAddr>().unwrap(),
            ))
            .body(Body::from(body))
            .unwrap();
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
        capture
            .find_event(tracing::Level::ERROR, "Login refused", "db_pool_missing")
            .expect("refusal must log reason=db_pool_missing at ERROR");
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
        capture
            .find_event(
                tracing::Level::WARN,
                "accepting credentials unchecked",
                "dev_mode_no_db_login",
            )
            .expect("the developer fallback must log reason=dev_mode_no_db_login at WARN");
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
            super::super::SessionRecord {
                account_id: 0x4420_0001,
                access_level: 0,
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
}
