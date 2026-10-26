//! Live-DB checks for the refresh reuse arm of `POST /token` (CT-74, C16.6):
//! a presented token that was already rotated revokes the launch ticket and
//! the family's access token, and still does so when the audit event cannot
//! be inserted. Sentinel account ids `0x7000_E200..=0x7000_E201`; cleanup
//! deletes by those exact ids, before and after each test.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::ConnectInfo;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use cimmeria_auth::auth::{AuthRouteDeps, LaunchTicketStore, TicketCheck};
use cimmeria_test_support::require_db_or_skip;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
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
use crate::stores::{pkce_challenge, HandoffOutcome, Stores};

/// The account whose reuse revokes a minted launch ticket.
const ID_TICKET: u32 = 0x7000_E200;
/// The account whose audit insert the trigger fails.
const ID_AUDIT: u32 = 0x7000_E201;
/// The audit trigger: `ID_AUDIT` is `0x7000_E201`, written in decimal because
/// DDL takes no bind parameter.
const AUDIT_TRIGGER: &str =
    "CREATE TRIGGER ct74_fail_event BEFORE INSERT ON account_identity_event \
     FOR EACH ROW WHEN (NEW.account_id = 1879106049) EXECUTE FUNCTION ct74_fail_event()";
const _: () = assert!(ID_AUDIT == 1_879_106_049);
/// Drops the audit trigger before and after each test.
const TRIGGER: &str = "DROP TRIGGER IF EXISTS ct74_fail_event ON account_identity_event";

/// The launcher's PKCE verifier and redirect for every code grant here.
const VERIFIER: &str = "launcher-verifier-0123456789abcdefghijklmnopqrstuvw";
const REDIRECT: &str = "http://127.0.0.1:5000/cb";

/// The config: one trusted dev-mock provider `dev`.
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

/// A router under test, with the stores it shares so a test can read them.
struct Env {
    router: Router,
    stores: Arc<Stores>,
    tickets: Arc<LaunchTicketStore>,
}

/// Builds the routes in developer mode with the dev-mock provider, the
/// database routes reach through `pool`.
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

/// Deletes the two sentinel accounts by exact id; their identities and
/// events cascade.
async fn clean(pool: &PgPool) {
    for id in [ID_TICKET, ID_AUDIT] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(id as i32)
            .execute(pool)
            .await;
    }
}

/// Drops the audit trigger and its function, if present.
async fn drop_trigger(pool: &PgPool) {
    let _ = sqlx::query(TRIGGER).execute(pool).await;
    let _ = sqlx::query("DROP FUNCTION IF EXISTS ct74_fail_event()")
        .execute(pool)
        .await;
}

/// Makes every `account_identity_event` insert for `ID_AUDIT` raise. The
/// trigger is scoped to that account, so other tests' events are unaffected.
async fn fail_audit(pool: &PgPool) {
    drop_trigger(pool).await;
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ct74_fail_event() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'ct74'; END $$",
    )
    .execute(pool)
    .await
    .expect("create the trigger function");
    sqlx::query(AUDIT_TRIGGER)
        .execute(pool)
        .await
        .expect("create the trigger");
}

/// Inserts a sentinel account named `Player_1234`, enabled.
async fn insert_account(pool: &PgPool, account_id: u32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, 'Player_1234', NULL, 1, 0, true)",
    )
    .bind(account_id as i32)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// A verified identity under the dev provider, with the given subject.
fn verified(subject: &str) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new("dev").expect("provider id"),
        issuer: "https://issuer.ct74.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT74 Player".to_owned(),
        email: None,
        email_verified: false,
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

/// Presents `refresh_token` at `POST /token`.
async fn refresh(env: &Env, refresh_token: &str) -> (StatusCode, Value) {
    post_json(
        env,
        "/token",
        json!({"grant_type": "refresh_token", "refresh_token": refresh_token}),
    )
    .await
}

/// The string field `key` of a response body.
fn field(body: &Value, key: &str) -> String {
    body[key]
        .as_str()
        .unwrap_or_else(|| panic!("a `{key}` in {body}"))
        .to_owned()
}

/// The SHA-1 of a launch ticket as uppercase hex, the form the client sends.
fn sha1_hex(ticket: &str) -> String {
    Sha1::digest(ticket.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect()
}

#[tokio::test]
async fn live_db_reuse_revokes_a_minted_launch_ticket() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    drop_trigger(&pool).await;
    insert_account(&pool, ID_TICKET).await;
    identity::insert(&pool, ID_TICKET, &verified("ct74-1"), false)
        .await
        .expect("link");
    let env = harness(&pool);
    let (_, first) = code_grant(&env, verified("ct74-1")).await;
    let old = field(&first, "refresh_token");
    let (rotated_status, rotated) = refresh(&env, &old).await;
    let minted = env
        .tickets
        .mint(ID_TICKET, "Player_1234", "launch-1", Instant::now());
    let before = env
        .tickets
        .verify("Player_1234", &sha1_hex(&minted.ticket), Instant::now());
    let (reuse_status, reuse) = refresh(&env, &old).await;
    let after = env
        .tickets
        .verify("Player_1234", &sha1_hex(&minted.ticket), Instant::now());
    clean(&pool).await;
    assert_eq!(rotated_status, StatusCode::OK, "{rotated}");
    assert!(matches!(before, TicketCheck::Valid { .. }));
    assert_eq!(reuse_status, StatusCode::BAD_REQUEST, "{reuse}");
    assert!(
        matches!(after, TicketCheck::NoMatch),
        "a reused refresh token revokes the account's launch ticket"
    );
}

#[tokio::test]
async fn live_db_reuse_revokes_when_the_audit_insert_fails() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_AUDIT).await;
    identity::insert(&pool, ID_AUDIT, &verified("ct74-2"), false)
        .await
        .expect("link");
    let env = harness(&pool);
    let (_, first) = code_grant(&env, verified("ct74-2")).await;
    let old = field(&first, "refresh_token");
    let access = field(&first, "access_token");
    let (rotated_status, rotated) = refresh(&env, &old).await;
    let minted = env
        .tickets
        .mint(ID_AUDIT, "Player_1234", "launch-2", Instant::now());
    fail_audit(&pool).await;
    let (reuse_status, reuse) = refresh(&env, &old).await;
    let ticket = env
        .tickets
        .verify("Player_1234", &sha1_hex(&minted.ticket), Instant::now());
    let access_live = env.stores.access.check(&access, Instant::now()).is_some();
    drop_trigger(&pool).await;
    clean(&pool).await;
    assert_eq!(rotated_status, StatusCode::OK, "{rotated}");
    assert_eq!(reuse_status, StatusCode::BAD_REQUEST, "{reuse}");
    assert_eq!(reuse["error"], "invalid_grant");
    assert!(matches!(ticket, TicketCheck::NoMatch));
    assert!(!access_live, "the family's access token no longer resolves");
}
