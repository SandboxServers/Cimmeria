//! Live-DB checks for the CT-30 routes: `GET /account`, `DELETE /identities/{id}`,
//! `POST /link/start`, and the launch-ticket mint, heartbeat and end. Covers the
//! identity list shape, the reauth and last-method refusals, the account-scoped
//! unlink and its audit event, the concurrent unlink (C15 e), the bearer and
//! `link/start` refusals, the add-method flow through `/authorize`, the
//! disabled and bad-id refusals, the no-store header and the rate limit.
//! Sentinel account ids `0x7000_E801..=0x7000_E809`; subjects `ct30-<n>`; account
//! names `CT30-<id>`. Cleanup deletes exactly the ids in `OWNED` and the
//! accounts of the `SIGNUP_SUBJECTS` identities, before and after each test.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::Router;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};
use cimmeria_test_support::require_db_or_skip;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use url::Url;

use crate::config;
use crate::limits::RateLimiter;
use crate::link_code::LinkCodeStore;
use crate::provider::IdentityProvider;
use crate::providers::dev_mock::DevMockProvider;
use crate::providers::Providers;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::{pkce_challenge, Stores};

const ACC_MAIN: u32 = 0x7000_E801;
const ACC_OTHER: u32 = 0x7000_E802;
const ACC_LAST: u32 = 0x7000_E803;
const ACC_DISABLED: u32 = 0x7000_E804;
const ACC_CONCURRENT: u32 = 0x7000_E805;
const ACC_TICKET: u32 = 0x7000_E806;
const ACC_RATE: u32 = 0x7000_E807;
const ACC_LINK: u32 = 0x7000_E808;
const ACC_UNLINK: u32 = 0x7000_E809;
/// Every sentinel account this file inserts; `clean` deletes exactly these.
const OWNED: &[u32] = &[
    ACC_MAIN,
    ACC_OTHER,
    ACC_LAST,
    ACC_DISABLED,
    ACC_CONCURRENT,
    ACC_TICKET,
    ACC_RATE,
    ACC_LINK,
    ACC_UNLINK,
];
/// The subjects this file signs up with (`dev`); an account holding one of these
/// identities is deleted, and no other account matches.
const SIGNUP_SUBJECTS: &[&str] = &["ct30-21"];
const PEER: &str = "127.0.0.1:41000";
const LAUNCH: &str = "CT30-LAUNCH-0000001";
const OTHER_LAUNCH: &str = "CT30-LAUNCH-0000002";
/// The launcher's PKCE verifier (43 characters) and its redirect.
const VERIFIER: &str = "ct30-verifier-0123456789abcdefghijklmnopqrs";
const CB: &str = "http://127.0.0.1:49152/cb";

/// The config: one trusted dev-mock provider `dev`, the one `link/start` names.
const TABLE: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = true
"#;

/// A router under test and the stores it shares, so a test can mint tokens and
/// read the launch tickets.
struct Env {
    router: Router,
    stores: Arc<Stores>,
    tickets: Arc<LaunchTicketStore>,
}

/// Builds the routes with the fixture config in developer mode.
fn harness(pool: &PgPool) -> Env {
    let env =
        |name: &str| (name == "CIMMERIA_IDP_TEST_SECRET").then(|| "s3cr3t-test-value".to_owned());
    let (cfg, _) = config::load(TABLE, &env, true).expect("the fixture loads");
    let trusted = Arc::new(DevMockProvider::new(
        &cfg.providers[0],
        &cfg.public_base_url,
    ));
    let list: Vec<Arc<dyn IdentityProvider>> = vec![trusted.clone()];
    let stores = Arc::new(Stores::default());
    let codes: &'static LinkCodeStore = Box::leak(Box::new(LinkCodeStore::default()));
    let ctx = IdentityCtx {
        limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
        cfg: Arc::new(cfg),
        providers: Arc::new(Providers::from_parts(list, Some(trusted))),
        stores: Arc::clone(&stores),
        link_codes: codes,
    };
    let tickets = Arc::new(LaunchTicketStore::default());
    let deps = AuthRouteDeps {
        db: Some(Arc::new(pool.clone())),
        launch_tickets: Arc::clone(&tickets),
        developer_mode: true,
    };
    Env {
        router: router(deps, ctx),
        stores,
        tickets,
    }
}

/// Deletes the sentinel accounts in `OWNED` and the accounts that hold a
/// `SIGNUP_SUBJECTS` identity; identities and events cascade.
pub(crate) async fn clean(pool: &PgPool) {
    let ids: Vec<i32> = OWNED.iter().map(|&id| id as i32).collect();
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await;
    let subjects: Vec<String> = SIGNUP_SUBJECTS.iter().map(|s| s.to_string()).collect();
    let _ = sqlx::query(
        "DELETE FROM account WHERE account_id IN \
         (SELECT account_id FROM account_identity WHERE provider = $1 AND subject = ANY($2))",
    )
    .bind("dev")
    .bind(subjects)
    .execute(pool)
    .await;
}

/// Inserts a passwordless sentinel account, enabled or not.
async fn insert_account(pool: &PgPool, account_id: u32, enabled: bool) {
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, NULL, 1, 0, $3, false)",
    )
    .bind(account_id as i32)
    .bind(format!("CT30-{account_id}"))
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Inserts a `dev` identity with subject `ct30-<n>` for `account_id`; returns its id.
async fn insert_identity(pool: &PgPool, account_id: u32, n: u32) -> i64 {
    let (id,): (i64,) = sqlx::query_as(
        "INSERT INTO account_identity (account_id, provider, issuer, subject, display_name) \
         VALUES ($1, 'dev', 'https://issuer.ct30.example.org', $2, 'CT30 Player') \
         RETURNING identity_id",
    )
    .bind(account_id as i32)
    .bind(format!("ct30-{n}"))
    .fetch_one(pool)
    .await
    .expect("insert identity");
    id
}

/// Whether the identity row `identity_id` still exists.
async fn identity_exists(pool: &PgPool, identity_id: i64) -> bool {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE identity_id = $1")
            .bind(identity_id)
            .fetch_one(pool)
            .await
            .expect("count identity");
    count == 1
}

/// The number of sign-in methods `account_id` has.
async fn methods(pool: &PgPool, account_id: u32) -> i64 {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(pool)
            .await
            .expect("count methods");
    count
}

/// Mints an access token for `account_id` issued at `issued`; `via_code` says
/// whether it came from a code sign-in.
fn token(env: &Env, account_id: u32, via_code: bool, issued: Instant) -> String {
    env.stores
        .access
        .mint(account_id, "ct30-family", via_code, issued)
}

/// A fresh code token for `account_id`.
fn code_token(env: &Env, account_id: u32) -> String {
    token(env, account_id, true, Instant::now())
}

/// Sends `method` to `uri` from the loopback peer, with an optional bearer token
/// and JSON body. Returns the status, the headers and the parsed body.
async fn send(
    router: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, HeaderMap, Value) {
    let peer: SocketAddr = PEER.parse().expect("peer address");
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .extension(ConnectInfo(peer));
    if let Some(t) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body = match body {
        Some(value) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let request = builder.body(body).expect("request");
    let response = router.clone().oneshot(request).await.expect("served");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 1 << 20).await.expect("body");
    (
        status,
        headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// `POST /launch-ticket` for `launch_id` with `bearer`.
async fn mint(env: &Env, bearer: &str, launch_id: &str) -> (StatusCode, HeaderMap, Value) {
    send(
        &env.router,
        "POST",
        "/launch-ticket",
        Some(bearer),
        Some(json!({"launch_id": launch_id})),
    )
    .await
}

/// Sends a form `POST` of `body` to `uri` from the loopback peer; returns the
/// status and headers.
async fn post_form(router: &Router, uri: &str, body: &str) -> (StatusCode, HeaderMap) {
    let peer: SocketAddr = PEER.parse().expect("peer address");
    let request = Request::post(uri)
        .extension(ConnectInfo(peer))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(body.to_owned()))
        .expect("request");
    let response = router.clone().oneshot(request).await.expect("served");
    (response.status(), response.headers().clone())
}

/// `target` (absolute or relative) parsed as a URL.
fn redirect(target: &str) -> Url {
    let base = Url::parse("http://ct30.test").expect("a base");
    base.join(target).expect("a redirect target")
}

/// The value of `key` in the query of `target`.
fn query_of(target: &str, key: &str) -> String {
    let url = redirect(target);
    let found = url.query_pairs().find(|(k, _)| k == key);
    found.expect("the query key").1.into_owned()
}

/// `target` as a router path: the path without `/auth/v1`, and its query.
fn route_of(target: &str) -> String {
    let url = redirect(target);
    let path = url
        .path()
        .strip_prefix("/auth/v1")
        .expect("an /auth/v1 path");
    match url.query() {
        Some(query) => format!("{path}?{query}"),
        None => path.to_owned(),
    }
}

/// A valid `POST /link/start` body for `provider` with challenge `method`.
fn link_body(provider: &str, method: &str) -> Value {
    json!({
        "provider": provider,
        "redirect_uri": CB,
        "state": "ct30StateAAAAAAAAAAAAAAAA",
        "code_challenge": pkce_challenge(VERIFIER),
        "code_challenge_method": method,
    })
}

/// The number of `unlink` events recorded for `account_id`.
async fn unlink_events(pool: &PgPool, account_id: u32) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'unlink'",
    )
    .bind(account_id as i32)
    .fetch_one(pool)
    .await
    .expect("count unlink events");
    count
}

/// `POST /link/start` with `bearer` and `body`; returns the status and body.
async fn link_start(env: &Env, bearer: &str, body: Value) -> (StatusCode, Value) {
    let (status, _, json) =
        send(&env.router, "POST", "/link/start", Some(bearer), Some(body)).await;
    (status, json)
}

/// `DELETE /identities/{id}` with `bearer`; returns the status only.
async fn unlink(env: &Env, bearer: &str, identity_id: i64) -> StatusCode {
    send(
        &env.router,
        "DELETE",
        &format!("/identities/{identity_id}"),
        Some(bearer),
        None,
    )
    .await
    .0
}

#[tokio::test]
async fn live_db_account_lists_identities_without_subjects() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_MAIN, true).await;
    let first = insert_identity(&pool, ACC_MAIN, 1).await;
    insert_identity(&pool, ACC_MAIN, 2).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_MAIN);
    let (status, _, body) = send(&env.router, "GET", "/account", Some(&t), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["account_name"], json!(format!("CT30-{ACC_MAIN}")));
    let rows = body["identities"].as_array().expect("identities array");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["identity_id"] == first));
    for row in rows {
        assert!(row.get("subject").is_none(), "no subject in {row}");
        assert!(row.get("email").is_none(), "no email in {row}");
    }
    assert!(!body.to_string().contains("ct30-"));
}

#[tokio::test]
async fn live_db_link_start_names_the_account_in_an_intent() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_LINK, true).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_LINK);
    let (status, _, body) = send(
        &env.router,
        "POST",
        "/link/start",
        Some(&t),
        Some(json!({
            "provider": "dev",
            "redirect_uri": "http://127.0.0.1:49152/cb",
            "state": "ct30StateAAAAAAAAAAAAAAAA",
            "code_challenge": "A".repeat(43),
            "code_challenge_method": "S256",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let url = url::Url::parse(body["authorize_url"].as_str().expect("authorize_url"))
        .expect("an absolute URL");
    assert_eq!(url.path(), "/auth/v1/authorize");
    let intent = url
        .query_pairs()
        .find(|(key, _)| key == "intent")
        .map(|(_, value)| value.into_owned())
        .expect("an intent parameter");
    let spent = env.stores.intents.take(&intent, Instant::now());
    assert_eq!(spent.map(|i| i.account_id), Some(ACC_LINK));
    assert!(env.stores.intents.take(&intent, Instant::now()).is_none());
}

#[tokio::test]
async fn live_db_link_start_attaches_a_second_identity() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_LINK, true).await;
    insert_identity(&pool, ACC_LINK, 20).await;
    let env = harness(&pool);
    // The attach needs the family of the session that started it to be live
    // (C16.5), so the bearer token carries a real family row.
    let (_, family) = crate::repo::refresh::issue(&pool, ACC_LINK, None, chrono::Utc::now())
        .await
        .expect("a live family");
    let t = env
        .stores
        .access
        .mint(ACC_LINK, &family, true, Instant::now());
    let (status, body) = link_start(&env, &t, link_body("dev", "S256")).await;
    assert_eq!(status, StatusCode::OK);
    let authorize_url = body["authorize_url"].as_str().expect("authorize_url");
    let intent = query_of(authorize_url, "intent");
    let authorize = route_of(authorize_url);
    // `/authorize` spends the intent and redirects to the dev-mock form.
    let (status, headers, _) = send(&env.router, "GET", &authorize, None, None).await;
    assert_eq!(status, StatusCode::FOUND);
    let form = headers[header::LOCATION].to_str().expect("text").to_owned();
    assert!(form.contains("/dev-mock/authorize?state="), "{form}");
    let server_state = query_of(&form, "state");
    // The form posts the identity; the mock redirects to its callback.
    let posted = format!("state={server_state}&subject=ct30-21&display_name=CT30+Player");
    let (status, headers) = post_form(&env.router, "/dev-mock/authorize", &posted).await;
    assert_eq!(status, StatusCode::FOUND);
    let callback = route_of(headers[header::LOCATION].to_str().expect("text"));
    // The callback hands back the launcher's code.
    let (status, headers, _) = send(&env.router, "GET", &callback, None, None).await;
    assert_eq!(status, StatusCode::FOUND);
    let handoff = query_of(headers[header::LOCATION].to_str().expect("text"), "code");
    let (status, _, _) = send(
        &env.router,
        "POST",
        "/token",
        None,
        Some(json!({
            "grant_type": "authorization_code",
            "code": handoff,
            "code_verifier": VERIFIER,
            "redirect_uri": CB,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(methods(&pool, ACC_LINK).await, 2);
    let (status, _, body) = send(&env.router, "GET", "/account", Some(&t), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["identities"].as_array().map(Vec::len), Some(2));
    // An unknown intent, and the spent one (the same URL again), are refused.
    let nope = authorize.replace(&intent, "nope");
    for uri in [nope, authorize] {
        let (status, _, body) = send(&env.router, "GET", &uri, None, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid_request");
    }
}

#[tokio::test]
async fn live_db_launch_ticket_is_no_store_and_names_the_account() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_TICKET, true).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_TICKET);
    let (status, headers, body) = mint(&env, &t, LAUNCH).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-store");
    assert_eq!(body["account_name"], json!(format!("CT30-{ACC_TICKET}")));
    assert!(body["ticket"]
        .as_str()
        .is_some_and(|ticket| !ticket.is_empty()));
    assert!(body["expires_in"].as_u64().is_some_and(|secs| secs > 0));
}

#[tokio::test]
async fn live_db_heartbeat_and_end() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_TICKET, true).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_TICKET);
    assert_eq!(mint(&env, &t, LAUNCH).await.0, StatusCode::OK);
    let beat = |id: &'static str| {
        send(
            &env.router,
            "POST",
            "/launch-ticket/heartbeat",
            Some(&t),
            Some(json!({"launch_id": id})),
        )
    };
    let end = |id: &'static str| {
        send(
            &env.router,
            "POST",
            "/launch-ticket/end",
            Some(&t),
            Some(json!({"launch_id": id})),
        )
    };
    assert_eq!(beat(LAUNCH).await.0, StatusCode::NO_CONTENT);
    assert_eq!(beat(OTHER_LAUNCH).await.0, StatusCode::NOT_FOUND);
    assert_eq!(end(LAUNCH).await.0, StatusCode::NO_CONTENT);
    assert_eq!(beat(LAUNCH).await.0, StatusCode::NOT_FOUND);
    assert_eq!(end(LAUNCH).await.0, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn live_db_launch_ticket_refused_for_disabled_account() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_DISABLED, false).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_DISABLED);
    let (status, _, body) = mint(&env, &t, LAUNCH).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "account_disabled");
}

#[tokio::test]
async fn live_db_launch_ticket_refuses_a_bad_launch_id() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_TICKET, true).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_TICKET);
    for bad in ["short", "CT30_LAUNCH_UNDERSCORE_0001", &"x".repeat(65)] {
        let (status, _, body) = mint(&env, &t, bad).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "mint {bad}");
        assert_eq!(body["error"], "invalid_request");
        let (end_status, _, _) = send(
            &env.router,
            "POST",
            "/launch-ticket/end",
            Some(&t),
            Some(json!({"launch_id": bad})),
        )
        .await;
        assert_eq!(end_status, StatusCode::BAD_REQUEST, "end {bad}");
    }
}

#[tokio::test]
async fn live_db_launch_ticket_mint_is_rate_limited_per_ip() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_RATE, true).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_RATE);
    // The token bucket allows 30 mints per 10 minutes from one address.
    for _ in 0..30 {
        assert_eq!(mint(&env, &t, LAUNCH).await.0, StatusCode::OK);
    }
    let (status, _, body) = mint(&env, &t, LAUNCH).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"], "rate_limited");
}

#[tokio::test]
async fn live_db_routes_refuse_a_missing_or_unknown_bearer() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_TICKET, true).await;
    let env = harness(&pool);
    let ticket = json!({"launch_id": LAUNCH});
    let routes = [
        ("GET", "/account", None),
        ("DELETE", "/identities/1", None),
        ("POST", "/link/start", None),
        ("POST", "/launch-ticket", Some(ticket.clone())),
        ("POST", "/launch-ticket/heartbeat", Some(ticket.clone())),
        ("POST", "/launch-ticket/end", Some(ticket)),
    ];
    for (method, uri, body) in routes {
        for bearer in [None, Some("not-a-token")] {
            let (status, _, json) = send(&env.router, method, uri, bearer, body.clone()).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
            assert_eq!(json["error"], "invalid_token", "{method} {uri}");
        }
    }
    // A token past its 15 minute access life is refused too.
    let old = Instant::now()
        .checked_sub(Duration::from_secs(16 * 60))
        .expect("an instant 16 minutes back");
    let expired = token(&env, ACC_TICKET, true, old);
    let (status, _, json) = send(&env.router, "GET", "/account", Some(&expired), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(json["error"], "invalid_token");
    // Another scheme than `Bearer` is refused.
    let peer: SocketAddr = PEER.parse().expect("peer address");
    let basic = Request::get("/account")
        .extension(ConnectInfo(peer))
        .header(header::AUTHORIZATION, "Basic abc")
        .body(Body::empty())
        .expect("request");
    let response = env.router.clone().oneshot(basic).await.expect("served");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_db_link_start_refusals() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_DISABLED, false).await;
    insert_account(&pool, ACC_LINK, true).await;
    let env = harness(&pool);
    let disabled = code_token(&env, ACC_DISABLED);
    let (status, body) = link_start(&env, &disabled, link_body("dev", "S256")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "account_disabled");
    let t = code_token(&env, ACC_LINK);
    let (status, body) = link_start(&env, &t, link_body("nope", "S256")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "unknown_provider");
    let (status, body) = link_start(&env, &t, link_body("dev", "plain")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_request");
}

#[path = "account_gate_tests_live_db.rs"]
mod gate;

#[path = "account_unlink_tests_live_db.rs"]
mod unlink;

#[path = "account_unlink_tx_tests_live_db.rs"]
mod unlink_tx;
