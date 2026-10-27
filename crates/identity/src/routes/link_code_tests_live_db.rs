//! Live-database tests for the link code's order (C02): the code is peeked
//! before the link is checked and spent only when the link is allowed, so a
//! refused link keeps its code and the same code links once the refusal is
//! gone. Sentinel ids `0x7000_EB00..=0x7000_EB01`; cleanup deletes exactly these
//! by id. The harness is copied from `link_tests_live_db.rs`, whose store is
//! private, so this file can issue the code it redeems.

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

use super::tests_live_db::verified;
use crate::config;
use crate::limits::RateLimiter;
use crate::link_code::LinkCodeStore;
use crate::provider::{IdentityProvider, VerifiedIdentity};
use crate::providers::dev_mock::DevMockProvider;
use crate::providers::Providers;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::Stores;

const ID_KEPT: u32 = 0x7000_EB00;
const ID_STAFF: u32 = 0x7000_EB01;
/// Every sentinel id this file inserts; `clean` deletes exactly these.
const SENTINELS: &[u32] = &[ID_KEPT, ID_STAFF];
const PEER: &str = "127.0.0.1:41000";

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

/// The routes under test, with the stores a test issues grants and link codes
/// into.
struct Env {
    router: Router,
    stores: Arc<Stores>,
    codes: &'static LinkCodeStore,
}

/// Builds the routes in developer mode, the database routes reach through `pool`.
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
    let deps = AuthRouteDeps {
        db: Some(Arc::new(pool.clone())),
        launch_tickets: Arc::new(LaunchTicketStore::default()),
        developer_mode: true,
    };
    Env {
        router: router(deps, ctx),
        stores,
        codes,
    }
}

/// Stores `identity` under a fresh link grant and returns the grant.
fn grant(env: &Env, identity: VerifiedIdentity) -> String {
    env.stores.grants.issue(identity, Instant::now())
}

/// Sends a JSON `POST` to `uri` from the loopback peer and returns the status
/// and the parsed body (`Null` when there is none).
async fn post_json(env: &Env, uri: &str, body: Value) -> (StatusCode, Value) {
    let peer: SocketAddr = PEER.parse().expect("peer address");
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

/// Deletes exactly the sentinel accounts in `SENTINELS`; identities and events
/// cascade.
async fn clean(pool: &PgPool) {
    let ids: Vec<i32> = SENTINELS.iter().map(|&id| id as i32).collect();
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await;
}

/// Inserts a sentinel legacy account with no password, at `accesslevel`.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str, accesslevel: i32) {
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, NULL, 1, $3, true, false)",
    )
    .bind(account_id as i32)
    .bind(name)
    .bind(accesslevel)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Sets `enabled` on the sentinel account `account_id` by exact id.
async fn set_enabled(pool: &PgPool, account_id: u32, enabled: bool) {
    sqlx::query("UPDATE account SET enabled = $2 WHERE account_id = $1")
        .bind(account_id as i32)
        .bind(enabled)
        .execute(pool)
        .await
        .expect("set enabled on sentinel account");
}

/// The number of identity rows linked to `account_id`.
async fn identity_rows(pool: &PgPool, account_id: u32) -> i64 {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account_identity WHERE account_id = $1")
            .bind(account_id as i32)
            .fetch_one(pool)
            .await
            .expect("count identities");
    count
}

/// A disabled account refuses the link with `account_disabled` and keeps the
/// code and the grant. Once the account is enabled, the same code and grant
/// link it. (Before the peek-then-redeem order, the first request burned the
/// code and the retry answered `invalid_credentials`.)
#[tokio::test]
async fn live_db_refused_link_code_keeps_the_code() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_KEPT, "ct26_kept_96", 0).await;
    set_enabled(&pool, ID_KEPT, false).await;
    let env = harness(&pool);
    let code = env.codes.issue(ID_KEPT, Instant::now());
    let link_grant = grant(&env, verified("dev", "ct26-96-kept", None, false));
    let body = json!({"link_grant": link_grant, "link_code": code, "password_removal_ack": true});
    let (refused_status, refused) = post_json(&env, "/link/existing", body.clone()).await;
    set_enabled(&pool, ID_KEPT, true).await;
    let (status, linked) = post_json(&env, "/link/existing", body).await;
    let identities = identity_rows(&pool, ID_KEPT).await;
    clean(&pool).await;
    assert_eq!(refused_status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"], "account_disabled");
    assert_eq!(status, StatusCode::OK, "{linked}");
    assert_eq!(linked["result"], "signed_in");
    assert_eq!(linked["account"]["account_id"], ID_KEPT);
    assert_eq!(identities, 1, "the kept code linked one identity");
}

/// A staff account refuses the link with `not_linkable` and keeps the code;
/// once its access level is 0, the same code links it.
#[tokio::test]
async fn live_db_staff_link_code_target_keeps_the_code() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ID_STAFF, "ct26_staff_96", 1).await;
    let env = harness(&pool);
    let code = env.codes.issue(ID_STAFF, Instant::now());
    let link_grant = grant(&env, verified("dev", "ct26-96-staff", None, false));
    let body = json!({"link_grant": link_grant, "link_code": code, "password_removal_ack": true});
    let (refused_status, refused) = post_json(&env, "/link/existing", body.clone()).await;
    sqlx::query("UPDATE account SET accesslevel = 0 WHERE account_id = $1")
        .bind(ID_STAFF as i32)
        .execute(&pool)
        .await
        .expect("lower sentinel access level");
    let (status, linked) = post_json(&env, "/link/existing", body).await;
    clean(&pool).await;
    assert_eq!(refused_status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"], "not_linkable");
    assert_eq!(status, StatusCode::OK, "{linked}");
    assert_eq!(linked["account"]["account_id"], ID_STAFF);
}
