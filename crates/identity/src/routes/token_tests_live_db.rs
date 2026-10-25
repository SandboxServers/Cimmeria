//! Live-DB checks for `POST /token` and `POST /logout` (CT-25): the code grant
//! for bound, new, auto-linked, untrusted and staff identities, and the refresh
//! grant for reuse, a disabled account and logout. Sentinel account ids
//! `2_000_501..=2_000_515`; cleanup deletes by that range, before and after
//! each test.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore};
use cimmeria_test_support::require_db_or_skip;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::config;
use crate::limits::RateLimiter;
use crate::link_code::LinkCodeStore;
use crate::provider::{IdentityProvider, ProviderId, VerifiedIdentity};
use crate::providers::dev_mock::DevMockProvider;
use crate::providers::Providers;
use crate::repo::identity;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::{pkce_challenge, HandoffOutcome, LinkIntent, Stores};

const FIRST_ID: u32 = 2_000_501;
const LAST_ID: u32 = 2_000_515;
const ID_BOUND: u32 = 2_000_501;
const ID_AUTO: u32 = 2_000_503;
const ID_UNTRUSTED: u32 = 2_000_504;
const ID_STAFF: u32 = 2_000_505;
const ID_REUSE: u32 = 2_000_506;
const ID_DISABLED: u32 = 2_000_507;
const ID_LOGOUT: u32 = 2_000_508;
const ID_ATTACH: u32 = 2_000_509;

/// The launcher's PKCE verifier and redirect for every code grant here.
const VERIFIER: &str = "launcher-verifier-0123456789abcdefghijklmnopqrstuvw";
const REDIRECT: &str = "http://127.0.0.1:5000/cb";

/// The config: a trusted dev-mock provider `dev` and an untrusted one `untrusted`.
const TABLE: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = true

[[provider]]
id = "untrusted"
kind = "dev_mock"
display_name = "Untrusted sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = false
"#;

/// A router under test, with the stores it shares so a test can issue codes.
struct Env {
    router: Router,
    stores: Arc<Stores>,
    tickets: Arc<LaunchTicketStore>,
}

/// Builds the routes in developer mode with the two dev-mock providers, the
/// database routes reach through `pool`.
fn harness(pool: &PgPool) -> Env {
    let env =
        |name: &str| (name == "CIMMERIA_IDP_TEST_SECRET").then(|| "s3cr3t-test-value".to_owned());
    let (cfg, _) = config::load(TABLE, &env, true).expect("the fixture loads");
    let trusted = Arc::new(DevMockProvider::new(
        &cfg.providers[0],
        &cfg.public_base_url,
    ));
    let untrusted = Arc::new(DevMockProvider::new(
        &cfg.providers[1],
        &cfg.public_base_url,
    ));
    let list: Vec<Arc<dyn IdentityProvider>> = vec![trusted.clone(), untrusted];
    let stores = Arc::new(Stores::default());
    let ctx = IdentityCtx {
        limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
        cfg: Arc::new(cfg),
        providers: Arc::new(Providers::from_parts(list, Some(trusted))),
        stores: Arc::clone(&stores),
        link_codes: Box::leak(Box::new(LinkCodeStore::default())),
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

/// Deletes every sentinel account; their identities and events cascade.
async fn clean(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id BETWEEN $1 AND $2")
        .bind(FIRST_ID as i32)
        .bind(LAST_ID as i32)
        .execute(pool)
        .await;
}

/// Inserts a sentinel account with the given access level, enabled.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str, accesslevel: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, $3, true)",
    )
    .bind(account_id as i32)
    .bind(name)
    .bind(accesslevel)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// A verified identity under `provider`, with the given subject and email.
fn verified(
    provider: &str,
    subject: &str,
    email: Option<&str>,
    verified: bool,
) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new(provider).expect("provider id"),
        issuer: "https://issuer.ct25.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT25 Player".to_owned(),
        email: email.map(str::to_owned),
        email_verified: verified,
        provider_account_created_ms: None,
    }
}

/// Sends a JSON `POST` to `uri` from the loopback peer and returns the status
/// and the parsed body (`Null` when there is none).
async fn post_json(env: &Env, uri: &str, body: Value) -> (StatusCode, Value) {
    let peer: SocketAddr = "127.0.0.1:41000".parse().expect("peer address");
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .extension(ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("request");
    let response = env.router.clone().oneshot(request).await.expect("served");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await.expect("body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Redeems a handoff code for `identity` at `POST /token`.
async fn code_grant(env: &Env, identity: VerifiedIdentity) -> (StatusCode, Value) {
    let code = env.stores.handoff.issue(
        HandoffOutcome {
            identity,
            intent: None,
        },
        &pkce_challenge(VERIFIER),
        REDIRECT,
        Instant::now(),
    );
    post_json(
        env,
        "/token",
        json!({
            "grant_type": "authorization_code",
            "code": code,
            "code_verifier": VERIFIER,
            "redirect_uri": REDIRECT,
        }),
    )
    .await
}

/// Rotates `refresh_token` at `POST /token`.
async fn refresh(env: &Env, refresh_token: &str) -> (StatusCode, Value) {
    post_json(
        env,
        "/token",
        json!({"grant_type": "refresh_token", "refresh_token": refresh_token}),
    )
    .await
}

/// The refresh token of a `signed_in` body.
fn refresh_of(body: &Value) -> String {
    body["refresh_token"]
        .as_str()
        .expect("a refresh token")
        .to_owned()
}

/// The number of `kind` events recorded for `account_id`.
async fn events(pool: &PgPool, account_id: u32, kind: &str) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = $2",
    )
    .bind(account_id as i32)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("count events");
    count
}

#[tokio::test]
async fn live_db_bound_identity_signs_in() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_BOUND, "ct25_bound_501", 0).await;
    let id = verified("dev", "ct25-1", None, false);
    identity::insert(&pool, ID_BOUND, &id, false)
        .await
        .expect("link");
    let env = harness(&pool);
    let (status, body) = code_grant(&env, id).await;
    let (refreshed_status, refreshed) = refresh(&env, &refresh_of(&body)).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "signed_in");
    assert_eq!(body["account"]["account_id"], ID_BOUND);
    assert_eq!(refreshed_status, StatusCode::OK, "{refreshed}");
    assert_eq!(refreshed["result"], "signed_in");
}

#[tokio::test]
async fn live_db_unknown_identity_needs_link() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let (status, body) = code_grant(&env, verified("dev", "ct25-2", None, false)).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "needs_link");
    assert!(body["link_grant"].is_string(), "{body}");
    assert!(body["signup_open"].is_boolean(), "{body}");
}

#[tokio::test]
async fn live_db_trusted_verified_unique_email_auto_links() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_AUTO, "ct25_auto_503", 0).await;
    let email = "ct25-3@example.org";
    identity::insert(
        &pool,
        ID_AUTO,
        &verified("dev", "ct25-3a", Some(email), true),
        true,
    )
    .await
    .expect("first link");
    let env = harness(&pool);
    let (status, body) = code_grant(&env, verified("dev", "ct25-3b", Some(email), true)).await;
    let autos = events(&pool, ID_AUTO, "auto_link").await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "signed_in");
    assert_eq!(body["account"]["account_id"], ID_AUTO);
    assert_eq!(autos, 1);
}

#[tokio::test]
async fn live_db_untrusted_provider_never_auto_links() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_UNTRUSTED, "ct25_untrusted_504", 0).await;
    let email = "ct25-4@example.org";
    identity::insert(
        &pool,
        ID_UNTRUSTED,
        &verified("dev", "ct25-4a", Some(email), true),
        true,
    )
    .await
    .expect("first link");
    let env = harness(&pool);
    let (status, body) =
        code_grant(&env, verified("untrusted", "ct25-4b", Some(email), true)).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "needs_link");
    assert_eq!(body["hint"], "none");
}

#[tokio::test]
async fn live_db_staff_account_is_never_auto_linked() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_STAFF, "ct25_staff_505", 1).await;
    let email = "ct25-5@example.org";
    identity::insert(
        &pool,
        ID_STAFF,
        &verified("dev", "ct25-5a", Some(email), true),
        true,
    )
    .await
    .expect("first link");
    let env = harness(&pool);
    let (status, body) = code_grant(&env, verified("dev", "ct25-5b", Some(email), true)).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "needs_link");
}

#[tokio::test]
async fn live_db_refresh_reuse_revokes_the_family() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_REUSE, "ct25_reuse_506", 0).await;
    identity::insert(
        &pool,
        ID_REUSE,
        &verified("dev", "ct25-6", None, false),
        false,
    )
    .await
    .expect("link");
    let env = harness(&pool);
    let (_, first) = code_grant(&env, verified("dev", "ct25-6", None, false)).await;
    let old = refresh_of(&first);
    let (rotated_status, rotated) = refresh(&env, &old).await;
    let new = refresh_of(&rotated);
    let (reuse_status, _) = refresh(&env, &old).await;
    let (new_status, _) = refresh(&env, &new).await;
    clean(&pool).await;
    assert_eq!(rotated_status, StatusCode::OK, "{rotated}");
    assert_eq!(reuse_status, StatusCode::BAD_REQUEST);
    assert_eq!(new_status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn live_db_disabled_account_refresh_is_403() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_DISABLED, "ct25_disabled_507", 0).await;
    identity::insert(
        &pool,
        ID_DISABLED,
        &verified("dev", "ct25-7", None, false),
        false,
    )
    .await
    .expect("link");
    let env = harness(&pool);
    let (_, first) = code_grant(&env, verified("dev", "ct25-7", None, false)).await;
    let old = refresh_of(&first);
    sqlx::query("UPDATE account SET enabled = false WHERE account_id = $1")
        .bind(ID_DISABLED as i32)
        .execute(&pool)
        .await
        .expect("disable");
    env.tickets
        .mint(ID_DISABLED, "ct25_disabled_507", "launch-1", Instant::now());
    let (status, body) = refresh(&env, &old).await;
    let tickets_left = env.tickets.live_count();
    let (after_status, _) = refresh(&env, &old).await;
    let (live,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(ID_DISABLED as i32)
    .fetch_one(&pool)
    .await
    .expect("count live tokens");
    clean(&pool).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(
        tickets_left, 0,
        "a disabled account's launch ticket is revoked"
    );
    assert_eq!(after_status, StatusCode::BAD_REQUEST);
    assert_eq!(live, 0);
}

/// An attach (`/link/start` intent) from a trusted provider whose email is
/// unverified links the identity but stores no email (D-CT9, C5). Dropping
/// the `email_verified` half of `attach` stores it.
#[tokio::test]
async fn live_db_attach_stores_no_unverified_email() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_ATTACH, "ct25_attach_509", 0).await;
    let env = harness(&pool);
    let identity = verified("dev", "ct25-10", Some("ct25-10@example.org"), false);
    let code = env.stores.handoff.issue(
        HandoffOutcome {
            identity,
            intent: Some(LinkIntent {
                account_id: ID_ATTACH,
            }),
        },
        &pkce_challenge(VERIFIER),
        REDIRECT,
        Instant::now(),
    );
    let (status, body) = post_json(
        &env,
        "/token",
        json!({
            "grant_type": "authorization_code",
            "code": code,
            "code_verifier": VERIFIER,
            "redirect_uri": REDIRECT,
        }),
    )
    .await;
    let (email_is_null,): (bool,) =
        sqlx::query_as("SELECT email IS NULL FROM account_identity WHERE account_id = $1")
            .bind(ID_ATTACH as i32)
            .fetch_one(&pool)
            .await
            .expect("attached identity");
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "signed_in");
    assert!(email_is_null, "an unverified email must not be stored");
}

#[tokio::test]
async fn live_db_logout_revokes() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_LOGOUT, "ct25_logout_508", 0).await;
    identity::insert(
        &pool,
        ID_LOGOUT,
        &verified("dev", "ct25-8", None, false),
        false,
    )
    .await
    .expect("link");
    let env = harness(&pool);
    let (_, first) = code_grant(&env, verified("dev", "ct25-8", None, false)).await;
    let token = refresh_of(&first);
    env.tickets
        .mint(ID_LOGOUT, "ct25_logout_508", "launch-1", Instant::now());
    let tickets_before = env.tickets.live_count();
    let (logout_status, _) = post_json(&env, "/logout", json!({"refresh_token": token})).await;
    let tickets_after = env.tickets.live_count();
    let (after_status, _) = refresh(&env, &token).await;
    clean(&pool).await;
    assert_eq!(tickets_before, 1);
    assert_eq!(
        tickets_after, 0,
        "logout revokes the account's launch ticket"
    );
    assert_eq!(logout_status, StatusCode::NO_CONTENT);
    assert_eq!(after_status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn live_db_token_limit_answers_429_before_redeeming() {
    let pool = require_db_or_skip!();
    let env = harness(&pool);
    for _ in 0..30 {
        let (status, _) = post_json(&env, "/token", json!({"grant_type": "bogus"})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, body) = code_grant(&env, verified("dev", "ct25-9", None, false)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["error"], "rate_limited");
}
