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
use tracing::Instrument;

use crate::audit::{emit_login_event, LoginOutcome};
use crate::credential_redaction::CredentialPrefix;

use super::credentials::{
    classify_credential, validate_credentials, AuthCredError, ClientCredential, CredentialGateError,
};
use super::soap_request::{parse_login_request, parse_server_selection};
use super::soap_response::{login_error, login_success_xml, select_error, server_location_xml};
use super::ticket_login::ticket_account;
use super::{
    HandlerState, PendingLogin, SessionRecord, TicketCheck, TlsConn, PROTOCOL_DIGEST, SESSION_TTL,
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
        tracing::info!(
            reason = "unknown_sku",
            peer_ip = %client_ip,
            "Phase 1 login rejected: unknown service SKU"
        );
        return login_error(3, "The specified service does not exist.");
    }
    // Validate the account name before it reaches the span or any audit row:
    // it is client-controlled, and after XML decoding a control character or
    // newline can arrive as `&#10;` as well as raw. Log only its length. The
    // client and `account_name` allow ASCII only; a Unicode letter reached the
    // audit row before.
    let name_ok = req.account_name.len() >= 3
        && req.account_name.len() <= 20
        && req
            .account_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
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
            tracing::warn!(account_name = %req.account_name, "plaintext credential rejected over plain HTTP");
            audit!(LoginOutcome::PlaintextRequiresTls);
            // Reuse the malformed-password code: the client must not learn that
            // a plaintext-over-TLS path exists.
            return login_error(2, "The specified password is invalid.");
        }
        Err(CredentialGateError::PlaintextLength) => {
            tracing::info!(
                reason = "plaintext_length",
                peer_ip = %client_ip,
                "Phase 1 login rejected: plaintext password length is outside the allowed range"
            );
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

    // A launch ticket (CT-21) arrives as the legacy SHA-1 credential. Over TLS a
    // live match is the account. Over plain HTTP the ticket has already crossed
    // the wire, so it is revoked and the login refused.
    let ticket = match credential {
        ClientCredential::LegacySha1Hex(hash) => {
            state
                .launch_tickets
                .verify(&req.account_name, hash, Instant::now())
        }
        ClientCredential::Plaintext(_) => TicketCheck::NoMatch,
    };
    let ticket_id = match ticket {
        TicketCheck::Valid { account_id } if !over_tls => {
            let consume = consume_span(account_id);
            record_consume(&consume, Instant::now(), "refused", "ticket_requires_tls");
            state.launch_tickets.revoke_account(account_id);
            tracing::warn!(
                account_id,
                account_name = %req.account_name,
                reason = "ticket_requires_tls",
                "launch ticket presented over plain HTTP; ticket revoked"
            );
            audit!(LoginOutcome::TicketRequiresTls, id = account_id);
            return login_error(4, "The account name or password is incorrect.");
        }
        TicketCheck::Valid { account_id } => Some(account_id),
        TicketCheck::NoMatch => None,
    };

    // Credential check.
    // A matching ticket names the account. Otherwise, if DB is available,
    // validate against the account table.
    // Only in developer mode with NO database configured are valid-format
    // credentials accepted without a check. A configured database whose pool
    // is missing refuses every login (fail closed).
    let (account_id, access_level, test_account): (u32, u32, bool) = if let Some(ticket_id) =
        ticket_id
    {
        let consume = consume_span(ticket_id);
        let started = Instant::now();
        let looked_up = ticket_account(
            state.db.as_deref(),
            state.developer_mode,
            state.db_configured,
            ticket_id,
        )
        .instrument(consume.clone())
        .await;
        match &looked_up {
            Ok(_) => record_consume(&consume, started, "ok", "ok"),
            Err(AuthCredError::InvalidCredentials) => {
                record_consume(&consume, started, "refused", "ticket_account_missing")
            }
            Err(AuthCredError::AccountDisabled) => {
                record_consume(&consume, started, "refused", "ticket_account_disabled")
            }
            Err(AuthCredError::DbError(_)) => {
                record_consume(&consume, started, "error", "db_error")
            }
        }
        match looked_up {
            Ok(acct) => acct,
            Err(AuthCredError::InvalidCredentials) => {
                state.launch_tickets.revoke_account(ticket_id);
                tracing::info!(
                    account_id = ticket_id,
                    account_name = %req.account_name,
                    reason = "ticket_account_missing",
                    "Launch ticket names an account with no row; ticket revoked"
                );
                audit!(LoginOutcome::InvalidCredentials);
                return login_error(4, "The account name or password is incorrect.");
            }
            Err(AuthCredError::AccountDisabled) => {
                state.launch_tickets.revoke_account(ticket_id);
                tracing::info!(
                    account_id = ticket_id,
                    account_name = %req.account_name,
                    reason = "ticket_account_disabled",
                    "Launch ticket account is disabled; ticket revoked"
                );
                audit!(LoginOutcome::AccountDisabled);
                return login_error(5, "This account has been suspended.");
            }
            Err(AuthCredError::DbError(e)) => {
                tracing::error!(
                    account_id = ticket_id,
                    account_name = %req.account_name,
                    error = %e,
                    "ticket account lookup failed"
                );
                audit!(LoginOutcome::DbError);
                return login_error(10, "A request to the database server failed.");
            }
        }
    } else if let Some(ref db) = state.db {
        match validate_credentials(db, &req.account_name, credential).await {
            Ok(acct) => (acct.account_id, acct.access_level, acct.test_account),
            Err(AuthCredError::InvalidCredentials) => {
                tracing::info!(account_name = %req.account_name, "Invalid credentials");
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
                tracing::info!(account_name = %req.account_name, "Account disabled");
                audit!(LoginOutcome::AccountDisabled);
                cimmeria_discord::emit_player_auth_failed(
                    req.account_name.as_str(),
                    addr,
                    "account_disabled",
                );
                return login_error(5, "This account has been suspended.");
            }
            Err(AuthCredError::DbError(e)) => {
                tracing::error!(account_name = %req.account_name, error = %e, "DB query failed");
                audit!(LoginOutcome::DbError);
                cimmeria_discord::emit_db_error("auth_credential_check", e.to_string());
                return login_error(10, "A request to the database server failed.");
            }
        }
    } else if state.developer_mode && !state.db_configured {
        tracing::warn!(
            account_name = %req.account_name,
            reason = "dev_mode_no_db_login",
            "Developer mode with no database configured: accepting credentials \
             unchecked as account 1, access level 99"
        );
        // Dev mode, no database configured: max access level, and no
        // `test_account` flag to read, so the session counts as a player.
        (1, 99, false)
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
            account_name = %req.account_name,
            reason,
            "Login refused: no database connection to check credentials against"
        );
        audit!(LoginOutcome::DbError);
        return login_error(10, "A request to the database server failed.");
    };

    if state.shards.is_empty() {
        tracing::warn!(
            account_id,
            account_name = %req.account_name,
            reason = "no_shards",
            peer_ip = %client_ip,
            "Phase 1 login refused: no shards are available"
        );
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
                test_account,
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
    // One audit row per login: a ticket login is recorded as `launch_ticket`.
    audit!(
        if ticket_id.is_some() {
            LoginOutcome::LaunchTicket
        } else {
            LoginOutcome::Success
        },
        id = account_id
    );

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
        None => {
            tracing::info!(
                reason = "sid_missing",
                peer_ip = %client_ip,
                "Phase 2 refused: no SID cookie"
            );
            return select_error(15, "Your logon session has expired. Please log in again.");
        }
    };

    // Consume session (remove from map) — each SID is single-use for Phase 2.
    let session = { state.sessions.lock().unwrap().remove(&sid) };
    let session = match session {
        Some(s) if s.created.elapsed() < SESSION_TTL => s,
        Some(s) => {
            tracing::info!(
                reason = "sid_expired",
                account_id = s.account_id,
                account_name = %s.account_name,
                sid_age_secs = s.created.elapsed().as_secs(),
                peer_ip = %client_ip,
                "Phase 2 refused: SID expired"
            );
            return select_error(15, "Your logon session has expired. Please log in again.");
        }
        None => {
            tracing::info!(
                reason = "sid_unknown",
                peer_ip = %client_ip,
                "Phase 2 refused: SID not issued or already consumed"
            );
            return select_error(15, "Your logon session has expired. Please log in again.");
        }
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
        None => {
            tracing::info!(
                reason = "unknown_shard",
                account_id = session.account_id,
                account_name = %session.account_name,
                peer_ip = %client_ip,
                "Phase 2 refused: no such shard"
            );
            return select_error(8, "No such shard.");
        }
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
                test_account: session.test_account,
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

/// `auth.launch_ticket.consume` (C11): the spend of a launch ticket. It carries
/// the account id, the outcome and the reason only, never the ticket or the name.
fn consume_span(account_id: u32) -> tracing::Span {
    tracing::info_span!(
        target: "cimmeria_auth",
        "auth.launch_ticket.consume",
        account_id,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// Records how a ticket spend ended on `span`: `outcome`, `reason` and the
/// whole-millisecond latency since `started`.
fn record_consume(span: &tracing::Span, started: Instant, outcome: &str, reason: &str) {
    span.record("outcome", outcome);
    span.record("reason", reason);
    span.record("latency_ms", started.elapsed().as_millis() as u64);
}

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
