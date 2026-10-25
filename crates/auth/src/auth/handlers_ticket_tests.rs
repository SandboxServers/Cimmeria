//! Tests for the launch-ticket login path in `handle_user_auth` (CT-21) and
//! the password check behind the account-link flow.
//!
//! The live-DB tests use the sentinel window from `2_000_401` and delete only
//! their own exact ids. Ticket tests share no store across tests, so they
//! need no ordering.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::ConnectInfo;
use sqlx::PgPool;
use tokio::sync::broadcast;

use super::credentials::sha1_upper_hex;
use super::handlers::handle_user_auth;
use super::{
    verify_account_password, HandlerState, LaunchTicketStore, PasswordCheck, ShardInfo, TlsConn,
};
use crate::audit::LoginEventBuffer;
use crate::test_support::require_db_or_skip;

/// Sentinel base for the ticket live-DB tests (fits in `i32`).
const BASE: i32 = 2_000_401;

const PHASE1_DIGEST: &str = "58AFA196AD3AC4F65CADD99BFF23B799";

/// Phase 1 body for `name` and `password` with the stock protocol digest.
fn login_body(name: &str, password: &str) -> String {
    format!(
        r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="{name}" Password="{password}" ProtocolDigest="{PHASE1_DIGEST}" />"#
    )
}

/// POST `body` to a Router over `state` from 127.0.0.1, with or without the
/// TLS marker. Returns the `SID` cookie (if any) and the response body.
async fn post_login(
    state: Arc<HandlerState>,
    body: &str,
    over_tls: bool,
) -> (Option<String>, String) {
    use axum::body::Body;
    use axum::http::{header, Request};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt; // oneshot

    let app = Router::new()
        .route("/SGWLogin/UserAuth", post(handle_user_auth))
        .with_state(state);
    let addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();
    let mut builder = Request::builder()
        .method("POST")
        .uri("/SGWLogin/UserAuth")
        .header("Content-Type", "text/xml")
        .extension(ConnectInfo(addr));
    if over_tls {
        builder = builder.extension(TlsConn);
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

/// A `HandlerState` with a live pool, a launch-ticket store and an audit
/// buffer the test can read back. Developer mode is off: only the pool or a
/// ticket can log in.
fn live_state(
    pool: &PgPool,
    tickets: Arc<LaunchTicketStore>,
    buffer: LoginEventBuffer,
) -> Arc<HandlerState> {
    let (tx, _rx) = broadcast::channel(16);
    Arc::new(HandlerState {
        shards: vec![ShardInfo {
            name: "TestShard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }],
        sessions: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
        developer_mode: false,
        db_configured: true,
        db: Some(Arc::new(pool.clone())),
        launch_tickets: tickets,
        login_tx: Some(tx),
        login_buffer: Some(buffer),
    })
}

/// Delete the sentinel account `account_id` by exact id.
async fn cleanup(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert a legacy (algo 1) account. `password` is the stored uppercase SHA-1
/// hex, or `None` for a passwordless (OIDC-only) row.
async fn insert_account(
    pool: &PgPool,
    account_id: i32,
    name: &str,
    password: Option<&str>,
    enabled: bool,
) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, 1, 2, $4)",
    )
    .bind(account_id)
    .bind(name)
    .bind(password)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert ticket fixture account");
}

/// Event outcomes recorded in the audit buffer, in order.
fn outcomes(buffer: &LoginEventBuffer) -> Vec<String> {
    buffer
        .snapshot()
        .into_iter()
        .map(|event| event.outcome)
        .collect()
}

/// **Ticket login over TLS.** A minted ticket's SHA-1, sent over the TLS
/// listener, logs in as the ticket's account and records `launch_ticket`.
/// Reverting step 4 (the ticket branch) turns this into a credential failure.
#[tokio::test]
async fn live_db_ticket_over_tls_logs_in() {
    let pool = require_db_or_skip!();
    let id = BASE;
    let name = format!("ticket-{id}");
    insert_account(&pool, id, &name, None, true).await;

    let tickets = Arc::new(LaunchTicketStore::default());
    let minted = tickets.mint(id as u32, &name, "launch-1", Instant::now());
    let buffer = LoginEventBuffer::new();
    let state = live_state(&pool, tickets, buffer.clone());

    let body = login_body(&name, &sha1_upper_hex(&minted.ticket));
    let (cookie, xml) = post_login(state, &body, true).await;

    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    assert!(cookie.is_some_and(|c| c.starts_with("SID=")));
    assert!(
        outcomes(&buffer).contains(&"launch_ticket".to_string()),
        "a ticket login must record launch_ticket, got {:?}",
        outcomes(&buffer)
    );
    cleanup(&pool, id).await;
}

/// **Replay guard.** A ticket presented over plain HTTP is refused with error
/// 4 and revoked, so a second attempt over TLS also fails. Reverting the
/// `!over_tls` arm lets the plain-HTTP attempt log in.
#[tokio::test]
async fn live_db_ticket_over_plain_http_is_refused_and_revoked() {
    let pool = require_db_or_skip!();
    let id = BASE + 1;
    let name = format!("ticket-{id}");
    insert_account(&pool, id, &name, Some(&sha1_upper_hex("unrelated")), true).await;

    let tickets = Arc::new(LaunchTicketStore::default());
    let minted = tickets.mint(id as u32, &name, "launch-2", Instant::now());
    let buffer = LoginEventBuffer::new();
    let state = live_state(&pool, Arc::clone(&tickets), buffer.clone());
    let hash = sha1_upper_hex(&minted.ticket);

    let (cookie, xml) = post_login(Arc::clone(&state), &login_body(&name, &hash), false).await;
    assert!(xml.contains("SGWLoginError"), "got: {xml}");
    assert!(
        xml.contains("account name or password is incorrect"),
        "got: {xml}"
    );
    assert!(cookie.is_none(), "a refused ticket must not set a SID");
    assert!(
        outcomes(&buffer).contains(&"ticket_requires_tls".to_string()),
        "got {:?}",
        outcomes(&buffer)
    );
    assert_eq!(tickets.live_count(), 0, "the ticket must be revoked");

    let (cookie, xml) = post_login(state, &login_body(&name, &hash), true).await;
    assert!(
        xml.contains("SGWLoginError"),
        "replay over TLS must fail, got: {xml}"
    );
    assert!(cookie.is_none());
    cleanup(&pool, id).await;
}

/// **Disabled account.** A live ticket for a disabled account gives error 5
/// ("suspended") and the ticket is revoked. Reverting the disabled arm
/// lets the ticket log in.
#[tokio::test]
async fn live_db_ticket_for_a_disabled_account_is_code_5() {
    let pool = require_db_or_skip!();
    let id = BASE + 2;
    let name = format!("ticket-{id}");
    insert_account(&pool, id, &name, None, false).await;

    let tickets = Arc::new(LaunchTicketStore::default());
    let minted = tickets.mint(id as u32, &name, "launch-3", Instant::now());
    let state = live_state(&pool, Arc::clone(&tickets), LoginEventBuffer::new());

    let body = login_body(&name, &sha1_upper_hex(&minted.ticket));
    let (cookie, xml) = post_login(state, &body, true).await;
    assert!(xml.contains("SGWLoginError"), "got: {xml}");
    assert!(
        xml.contains("has been suspended"),
        "error 5 expected, got: {xml}"
    );
    assert!(cookie.is_none());
    assert_eq!(tickets.live_count(), 0, "the ticket must be revoked");
    cleanup(&pool, id).await;
}

/// **Unmodified client.** A legacy account's real SHA-1 hex over plain HTTP
/// still logs in: the ticket path does not replace the password path (D-CT7).
#[tokio::test]
async fn live_db_unmodified_client_sha1_over_http_still_logs_in() {
    let pool = require_db_or_skip!();
    let id = BASE + 3;
    let name = format!("legacy-{id}");
    insert_account(&pool, id, &name, Some(&sha1_upper_hex("legacy-pw")), true).await;

    let state = live_state(
        &pool,
        Arc::new(LaunchTicketStore::default()),
        LoginEventBuffer::new(),
    );
    let body = login_body(&name, &sha1_upper_hex("legacy-pw"));
    let (cookie, xml) = post_login(state, &body, false).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    assert!(cookie.is_some_and(|c| c.starts_with("SID=")));
    cleanup(&pool, id).await;
}

/// **OIDC-only account.** A row with no password (`password` NULL, algo 1)
/// refuses every password, including the SHA-1 of `""` and of `"x"`, with
/// error 4 and no panic.
#[tokio::test]
async fn live_db_passwordless_account_refuses_every_password() {
    let pool = require_db_or_skip!();
    let id = BASE + 4;
    let name = format!("oidc-{id}");
    insert_account(&pool, id, &name, None, true).await;

    for password in ["", "x"] {
        let state = live_state(
            &pool,
            Arc::new(LaunchTicketStore::default()),
            LoginEventBuffer::new(),
        );
        let body = login_body(&name, &sha1_upper_hex(password));
        let (cookie, xml) = post_login(state, &body, false).await;
        assert!(
            xml.contains("SGWLoginError"),
            "password {password:?}: got {xml}"
        );
        assert!(
            xml.contains("account name or password is incorrect"),
            "got {xml}"
        );
        assert!(!xml.contains("SGWLoginSuccess"));
        assert!(cookie.is_none());
    }
    cleanup(&pool, id).await;
}

/// **Name guard.** A Unicode letter in the account name is refused with
/// error 1 before any database access. Reverting the ASCII-only check lets it
/// past the name check, and then developer mode accepts it.
#[tokio::test]
async fn unicode_account_name_is_code_1() {
    let state = Arc::new(HandlerState {
        shards: vec![ShardInfo {
            name: "TestShard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }],
        sessions: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
        developer_mode: true,
        db_configured: false,
        db: None,
        launch_tickets: Arc::new(LaunchTicketStore::default()),
        login_tx: None,
        login_buffer: None,
    });
    let body = login_body("Jos\u{e9}", &sha1_upper_hex("pw"));
    let (cookie, xml) = post_login(state, &body, false).await;
    assert!(xml.contains("SGWLoginError"), "got: {xml}");
    assert!(
        xml.contains("account name is invalid"),
        "error 1 expected, got: {xml}"
    );
    assert!(cookie.is_none());
}

/// **Password check keeps legacy rows.** `verify_account_password` accepts the
/// right plaintext and refuses a wrong one, and leaves the row on algo 1 with
/// the same `password`. The unmodified client's SHA-1 hex still logs in
/// afterwards. Reverting the algo-1 arm, or migrating, trips this (step 6, D-CT7).
#[tokio::test]
async fn live_db_verify_account_password_keeps_legacy_rows() {
    let pool = require_db_or_skip!();
    let id = BASE + 5;
    let name = format!("pwcheck-{id}");
    let stored = sha1_upper_hex("right-pw");
    insert_account(&pool, id, &name, Some(&stored), true).await;

    assert_eq!(
        verify_account_password(&pool, &name, "right-pw").await,
        PasswordCheck::Ok {
            account_id: id as u32
        }
    );
    assert_eq!(
        verify_account_password(&pool, &name, "wrong-pw").await,
        PasswordCheck::Invalid
    );

    let (algo, password): (i16, Option<String>) =
        sqlx::query_as("SELECT password_algo, password FROM account WHERE account_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(algo, 1, "the check must not migrate the row");
    assert_eq!(password.as_deref(), Some(stored.as_str()));

    let state = live_state(
        &pool,
        Arc::new(LaunchTicketStore::default()),
        LoginEventBuffer::new(),
    );
    let (_cookie, xml) = post_login(state, &login_body(&name, &stored), false).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    cleanup(&pool, id).await;
}
