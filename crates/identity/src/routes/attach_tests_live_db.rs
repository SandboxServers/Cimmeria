//! Live-DB checks for the `/link/start` attach (C6, C16.5, C12): a link intent
//! attaches only while the session that started it is live and the account is
//! still linkable. The intent is built directly with the family `/link/start`
//! would copy; the handoff code and `POST /token` run as in production. Sentinel
//! account ids `0x7000_E300..=0x7000_E3FF`; cleanup deletes only the exact ids
//! it inserted, before and after each test.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use chrono::Utc;
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
use crate::repo::refresh;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::{pkce_challenge, HandoffOutcome, LinkIntent, Stores};

const ID_UNLINKED_AT_INTENT: u32 = 0x7000_E300;
const ID_REVOKED_AT_HANDOFF: u32 = 0x7000_E301;
const ID_LIVE_FAMILY: u32 = 0x7000_E302;
const ID_STAFF: u32 = 0x7000_E303;

/// The launcher's PKCE verifier and redirect for every code grant here.
const VERIFIER: &str = "launcher-verifier-0123456789abcdefghijklmnopqrstuvw";
const REDIRECT: &str = "http://127.0.0.1:5000/cb";

/// The config: a trusted dev-mock provider `dev`, the one the attach uses.
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

/// A router under test, with the stores it shares so a test can issue codes.
struct Env {
    router: Router,
    stores: Arc<Stores>,
}

/// Builds the routes in developer mode with the dev-mock provider, the database
/// routes reach through `pool`.
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
    let ctx = IdentityCtx {
        limiter: Arc::new(RateLimiter::with_limits(&cfg.limits)),
        cfg: Arc::new(cfg),
        providers: Arc::new(Providers::from_parts(list, Some(trusted))),
        stores: Arc::clone(&stores),
        link_codes: Box::leak(Box::new(LinkCodeStore::default())),
    };
    let deps = AuthRouteDeps {
        db: Some(Arc::new(pool.clone())),
        launch_tickets: Arc::new(LaunchTicketStore::default()),
        developer_mode: true,
    };
    Env {
        router: router(deps, ctx),
        stores,
    }
}

/// Deletes the four sentinel accounts by exact id: their refresh families first,
/// then the accounts (their identities and events cascade).
async fn clean(pool: &PgPool) {
    let ids: Vec<i32> = [
        ID_UNLINKED_AT_INTENT,
        ID_REVOKED_AT_HANDOFF,
        ID_LIVE_FAMILY,
        ID_STAFF,
    ]
    .iter()
    .map(|id| *id as i32)
    .collect();
    let _ = sqlx::query("DELETE FROM auth_refresh_token WHERE account_id = ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(&ids)
        .execute(pool)
        .await;
}

/// Inserts a sentinel account with the given access level, enabled.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, 0, true)",
    )
    .bind(account_id as i32)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// A verified identity under `dev`, with the given subject. Its email is
/// verified and trusted, so the attach stores it.
fn verified(subject: &str) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new("dev").expect("provider id"),
        issuer: "https://issuer.attach.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "Attach Player".to_owned(),
        email: Some("attach@example.org".to_owned()),
        email_verified: true,
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

/// Issues the handoff code of a link intent for `identity` and redeems it at
/// `POST /token`, as the callback and the launcher do.
async fn attach_grant(
    env: &Env,
    identity: VerifiedIdentity,
    account_id: u32,
    family_id: &str,
) -> (StatusCode, Value) {
    let code = env.stores.handoff.issue(
        HandoffOutcome {
            identity,
            intent: Some(LinkIntent {
                account_id,
                family_id: family_id.to_owned(),
            }),
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

/// The number of identities bound to `account_id`.
async fn identities(pool: &PgPool, account_id: u32) -> i64 {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(pool)
            .await
            .expect("count identities");
    count
}

/// Revoking the family after the intent is built and before the handoff code
/// is redeemed refuses the attach: no identity is added, no family is issued.
#[tokio::test]
async fn live_db_add_method_after_unlink_at_intent_is_refused() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_UNLINKED_AT_INTENT, "ct75_unlinked_300").await;
    let env = harness(&pool);
    let (_, family_id) = refresh::issue(&pool, ID_UNLINKED_AT_INTENT, None, Utc::now())
        .await
        .expect("a live family");
    refresh::revoke_family(&pool, &family_id, Utc::now())
        .await
        .expect("revoke the family");
    let (status, body) = attach_grant(
        &env,
        verified("ct75-300"),
        ID_UNLINKED_AT_INTENT,
        &family_id,
    )
    .await;
    let added = identities(&pool, ID_UNLINKED_AT_INTENT).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_grant");
    assert_eq!(added, 0, "a revoked intent must not add an identity");
}

/// A family revoked after the handoff code is issued and before `POST /token`
/// refuses the attach the same way.
#[tokio::test]
async fn live_db_add_method_after_revoke_at_handoff_is_refused() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_REVOKED_AT_HANDOFF, "ct75_handoff_301").await;
    let env = harness(&pool);
    let (_, family_id) = refresh::issue(&pool, ID_REVOKED_AT_HANDOFF, None, Utc::now())
        .await
        .expect("a live family");
    let code = env.stores.handoff.issue(
        HandoffOutcome {
            identity: verified("ct75-301"),
            intent: Some(LinkIntent {
                account_id: ID_REVOKED_AT_HANDOFF,
                family_id: family_id.clone(),
            }),
        },
        &pkce_challenge(VERIFIER),
        REDIRECT,
        Instant::now(),
    );
    refresh::revoke_family(&pool, &family_id, Utc::now())
        .await
        .expect("revoke the family");
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
    let added = identities(&pool, ID_REVOKED_AT_HANDOFF).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_grant");
    assert_eq!(added, 0, "a revoked handoff must not add an identity");
}

/// The happy path: a live family attaches the identity and answers `signed_in`.
#[tokio::test]
async fn live_db_add_method_with_a_live_family_attaches() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_LIVE_FAMILY, "ct75_live_302").await;
    let env = harness(&pool);
    let (_, family_id) = refresh::issue(&pool, ID_LIVE_FAMILY, None, Utc::now())
        .await
        .expect("a live family");
    let (status, body) = attach_grant(&env, verified("ct75-302"), ID_LIVE_FAMILY, &family_id).await;
    let added = identities(&pool, ID_LIVE_FAMILY).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "signed_in");
    assert_eq!(added, 1, "a live family attaches one identity");
}

/// An account made staff after `/link/start` refuses the attach with
/// `not_linkable` (C12), though its family is live and it is enabled.
#[tokio::test]
async fn live_db_add_method_to_a_staff_account_is_refused() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_STAFF, "ct75_staff_303").await;
    let env = harness(&pool);
    let (_, family_id) = refresh::issue(&pool, ID_STAFF, None, Utc::now())
        .await
        .expect("a live family");
    sqlx::query("UPDATE account SET accesslevel = 1 WHERE account_id = $1")
        .bind(ID_STAFF as i32)
        .execute(&pool)
        .await
        .expect("make the account staff");
    let (status, body) = attach_grant(&env, verified("ct75-303"), ID_STAFF, &family_id).await;
    let added = identities(&pool, ID_STAFF).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "not_linkable");
    assert_eq!(added, 0, "a staff account must not gain an identity");
}
