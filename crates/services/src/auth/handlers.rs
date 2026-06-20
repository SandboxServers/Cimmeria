//! HTTP/SOAP handlers for Phase 1 (UserAuth) and Phase 2 (ServerSelection),
//! plus XML parsing, credential validation, and random generators.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use quick_xml::{events::Event, Reader};
use rand::RngExt;

use crate::audit::emit_login_event;

use super::credentials::{
    classify_credential, validate_credentials, AuthCredError, CredentialGateError,
};
use super::{
    HandlerState, LoginReq, PendingLogin, SessionRecord, TlsConn, LOGIN_NS, PROTOCOL_DIGEST,
    SELECT_NS, SESSION_TTL, XML_DECL,
};

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
    tracing::trace!(body = %body, "Phase 1 raw SOAP request");

    let client_ip = addr.ip().to_string();

    let req = match parse_login_request(&body) {
        Ok(r) => r,
        Err(e) => {
            tracing::Span::current().record("result", "bad_request");
            tracing::warn!("Bad login request: {e}");
            return login_error(13, "Internal error.");
        }
    };
    tracing::Span::current().record("account_name", req.account_name.as_str());

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
    // Classify the supplied credential: a 40-char hex string is the original
    // client's SHA-1 hash (allowed over HTTP or TLS); anything else is treated
    // as a plaintext password, which is only honoured over TLS.
    let credential = match classify_credential(&req.password, over_tls) {
        Ok(c) => c,
        Err(CredentialGateError::PlaintextRequiresTls) => {
            tracing::warn!(user = %req.account_name, "plaintext credential rejected over plain HTTP");
            audit!("plaintext_requires_tls");
            // Reuse the malformed-password code: the client must not learn that
            // a plaintext-over-TLS path exists.
            return login_error(2, "The specified password is invalid.");
        }
        Err(CredentialGateError::PlaintextLength) => {
            return login_error(2, "The specified password is invalid.");
        }
    };
    let name_ok = req.account_name.len() >= 3
        && req.account_name.len() <= 20
        && req
            .account_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    if !name_ok {
        return login_error(1, "The specified account name is invalid.");
    }
    if !state.developer_mode && req.protocol_digest.to_uppercase() != PROTOCOL_DIGEST {
        tracing::warn!(got = %req.protocol_digest, expected = PROTOCOL_DIGEST, "Protocol digest mismatch");
        audit!("protocol_mismatch");
        return login_error(
            17,
            "Protocol version mismatch; your client version is not supported.",
        );
    }

    // Credential check.
    // If DB is available, validate against the account table.
    // In developer mode without DB, any valid-format credentials are accepted.
    let (account_id, access_level): (u32, u32) = if let Some(ref db) = state.db {
        match validate_credentials(db, &req.account_name, credential).await {
            Ok(acct) => (acct.account_id, acct.access_level),
            Err(AuthCredError::InvalidCredentials) => {
                tracing::info!(user = %req.account_name, "Invalid credentials");
                audit!("invalid_credentials");
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
                audit!("account_disabled");
                cimmeria_discord::emit_player_auth_failed(
                    req.account_name.as_str(),
                    addr,
                    "account_disabled",
                );
                return login_error(5, "This account has been suspended.");
            }
            Err(AuthCredError::DbError(e)) => {
                tracing::error!(user = %req.account_name, error = %e, "DB query failed");
                audit!("db_error");
                cimmeria_discord::emit_db_error("auth_credential_check", e.to_string());
                return login_error(10, "A request to the database server failed.");
            }
        }
    } else if state.developer_mode {
        tracing::debug!(user = %req.account_name, "developer mode: accepting credentials (no DB)");
        (1, 99) // dev mode: max access level
    } else {
        audit!("db_error");
        return login_error(10, "A request to the database server failed.");
    };

    if state.shards.is_empty() {
        audit!("no_shards", id = account_id);
        return login_error(7, "No shards are available to the authentication server.");
    }

    // 40-char alphanumeric SID matching the C++ session cookie format.
    let sid = random_alphanumeric(40);
    tracing::debug!(sid = %sid, "Phase 1 generated SID");
    {
        state.sessions.lock().unwrap().insert(
            sid.clone(),
            SessionRecord {
                account_id,
                access_level,
                account_name: req.account_name.clone(),
                created: Instant::now(),
            },
        );
    }

    tracing::Span::current().record("account_id", account_id);
    tracing::Span::current().record("result", "success");
    tracing::info!(user = %req.account_name, account_id, access_level, ip = %client_ip, "Phase 1 success");
    audit!("success", id = account_id);

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
    tracing::trace!(body = %body, "Phase 2 raw SOAP request");

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

    let selected = match parse_server_selection(&body) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Bad server selection: {e}");
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
            user = %session.account_name, shard = %shard.name,
            access_level = session.access_level,
            "Access denied to protected shard"
        );
        return select_error(6, "Access denied.");
    }

    // 64-char hex AES-256 session key, 20-char hex ticket.
    let session_key = random_hex(32);
    let ticket = random_hex(10);
    tracing::debug!(ticket = %ticket, "Phase 2 generated session credentials");

    {
        state.pending_logins.lock().unwrap().insert(
            ticket.clone(),
            PendingLogin {
                account_id: session.account_id,
                account_name: session.account_name.clone(),
                access_level: session.access_level,
                ticket: ticket.clone(),
                session_key: session_key.clone(),
                created: Instant::now(),
            },
        );
    }

    tracing::Span::current().record("account_id", session.account_id);
    tracing::Span::current().record("shard", shard.name.as_str());
    tracing::Span::current().record("result", "success");
    tracing::info!(
        user = %session.account_name,
        shard = %shard.name,
        ip = %client_ip,
        ticket_prefix = %&ticket[..6],
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
            "success",
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

fn parse_login_request(body: &str) -> Result<LoginReq, String> {
    let mut reader = Reader::from_str(body);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if e.local_name().as_ref() == b"SGWLoginRequest" =>
            {
                let mut req = LoginReq::default();
                for attr in e.attributes().flatten() {
                    let val = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                    match attr.key.as_ref() {
                        b"SKU" => req.sku = val,
                        b"AccountName" => req.account_name = val,
                        b"Password" => req.password = val,
                        b"ProtocolDigest" => req.protocol_digest = val,
                        _ => {}
                    }
                }
                return Ok(req);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Err("SGWLoginRequest element not found".into())
}

fn parse_server_selection(body: &str) -> Result<String, String> {
    let mut reader = Reader::from_str(body);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if e.local_name().as_ref() == b"SGWSelectServerRequest" =>
            {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"ServerSelection" {
                        return Ok(String::from_utf8_lossy(attr.value.as_ref()).into_owned());
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Err("SGWSelectServerRequest/ServerSelection not found".into())
}

fn extract_sid(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie
        .split(';')
        .map(str::trim)
        .find(|s| s.starts_with("SID="))
        .map(|s| s["SID=".len()..].to_string())
}

fn login_error(_code: u32, msg: &str) -> Response {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    // The client uses ErrorStr for display and ignores ErrorNum.
    let xml = format!(
        "{XML_DECL}\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginError ns3:ErrorStr=\"{msg}\" ns3:ErrorNum=\"1\" />\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

fn login_success_xml(account_id: u32, shards: &[super::ShardInfo]) -> String {
    let entries: String = shards
        .iter()
        .map(|s| {
            format!(
                "<Shard ServerName=\"{}\" Fullness=\"LOW\" Busy=\"LOW\" />",
                s.name
            )
        })
        .collect();

    format!(
        "{XML_DECL}\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginSuccess>\
         <AccountInfo ExpireDate=\"0000-00-00T00:00:00.000Z\" AccountId=\"{account_id}\" />\
         <SGWShardListResp>{entries}</SGWShardListResp>\
         </SGWLoginSuccess>\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    )
}

fn select_error(_code: u32, msg: &str) -> Response {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    let xml = format!(
        "{XML_DECL}\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerSelectionError ns1:ErrorStr=\"{msg}\" ns1:ErrorNum=\"1\" />\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

fn server_location_xml(shard: &super::ShardInfo, session_key: &str, ticket: &str) -> String {
    format!(
        "{XML_DECL}\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerLocation SessionKey=\"{session_key}\" Port=\"{port}\" IP=\"{ip}\" BWMailBox=\"1\">\
         <TICKET Ticket=\"{ticket}\" />\
         </ServerLocation>\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
        port = shard.port,
        ip = shard.host,
    )
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
    async fn seed_dev_accounts_are_at_least_gamemaster() {
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

    #[test]
    fn parse_valid_login_request() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="test" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let req = parse_login_request(body).unwrap();
        assert_eq!(req.sku, "SGW_BETA");
        assert_eq!(req.account_name, "test");
        assert_eq!(req.password, "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3");
    }

    #[test]
    fn parse_server_selection_request() {
        let body = r#"<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="Shard" />"#;
        let sel = parse_server_selection(body).unwrap();
        assert_eq!(sel, "Shard");
    }

    #[test]
    fn login_success_xml_contains_shard() {
        let shards = vec![super::super::ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }];
        let xml = login_success_xml(42, &shards);
        assert!(xml.contains(r#"AccountId="42""#));
        assert!(xml.contains(r#"ServerName="Shard""#));
        // Bare XML — no SOAP envelope (matches C++ LogonConnection output).
        assert!(!xml.contains("SOAP-ENV:Envelope"));
        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));
        assert!(xml.contains("ns2:SGWLoginResponse"));
    }

    #[test]
    fn server_location_xml_contains_key_and_ticket() {
        let shard = super::super::ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        };
        let xml = server_location_xml(&shard, "AAAA", "BBBB");
        assert!(xml.contains(r#"SessionKey="AAAA""#));
        assert!(xml.contains(r#"Ticket="BBBB""#));
        assert!(xml.contains(r#"Port="32832""#));
    }

    /// Pin the parser's narrow contract: when the SGWLoginRequest element
    /// is present but required attributes (SKU/AccountName/Password) are
    /// missing, the parse succeeds with default (empty) fields. The
    /// handler validates above the parser; a refactor that pushed
    /// validation down here would silently break that layering.
    #[test]
    fn parse_login_request_does_not_validate_missing_attributes() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" />"#;
        let req = parse_login_request(body).expect("element present must parse Ok");
        assert_eq!(req.sku, "", "missing SKU must surface as empty, not error");
        assert_eq!(req.account_name, "");
        assert_eq!(req.password, "");
    }

    /// Same narrow-contract pin, but for password length: a password the
    /// handler will reject (not 40 hex chars) parses through unchanged.
    /// Validation lives in the handler — this guard keeps it there.
    #[test]
    fn parse_login_request_does_not_validate_password_length() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="test" Password="short" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let req = parse_login_request(body).expect("well-formed element must parse Ok");
        assert_eq!(
            req.password, "short",
            "parser must hand the raw password through; length check is the handler's job",
        );
    }

    /// Unlike parse_login_request, parse_server_selection requires the
    /// ServerSelection attribute — the function returns early on the
    /// first ServerSelection it sees, otherwise falls through to Err.
    /// A refactor that loosened this would let the auth flow accept a
    /// server-selection message with no shard id.
    #[test]
    fn parse_server_selection_missing_attribute_returns_error() {
        let body = r#"<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" />"#;
        let sel = parse_server_selection(body);
        assert!(sel.is_err(), "missing ServerSelection attribute must fail");
    }

    /// Build a minimal `HandlerState` for driving `handle_user_auth` through
    /// the Router without a DB or event channel. `developer_mode` is true so
    /// the credential check short-circuits — but the plaintext-over-HTTP gate
    /// runs *before* that, which is exactly what the test below pins.
    fn test_handler_state() -> Arc<HandlerState> {
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
            developer_mode: true,
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
}
