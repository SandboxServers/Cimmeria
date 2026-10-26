//! Live-DB checks for `POST /link/new` and `POST /link/existing` (CT-26): the
//! sign-up gates and account shape, the password and link-code forms, the
//! per-account limit, the refusals of staff and test accounts, and the password
//! clearing of a link (C15). Sentinel account ids `0x7000_E900..=0x7000_E909`;
//! subjects `ct26-<n>`. Cleanup deletes exactly the ids in `OWNED` and the
//! accounts holding a `signup_identities()` identity (exact provider and subject),
//! before and after each test, so the accounts that `link/new` creates go too.

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
use crate::names;
use crate::provider::{IdentityProvider, ProviderId, VerifiedIdentity};
use crate::providers::dev_mock::DevMockProvider;
use crate::providers::Providers;
use crate::routes::ctx::IdentityCtx;
use crate::routes::router;
use crate::stores::Stores;

const ID_WRONG: u32 = 0x7000_E900;
const ID_LEGACY: u32 = 0x7000_E901;
const ID_CODE: u32 = 0x7000_E902;
const ID_LIMITED: u32 = 0x7000_E903;
const ID_STAFF: u32 = 0x7000_E904;
const ID_TEST: u32 = 0x7000_E905;
const ID_CODE_ONCE: u32 = 0x7000_E906;
const ID_ACK: u32 = 0x7000_E907;
const ID_REFUSED: u32 = 0x7000_E908;
const ID_LINK_DISABLED: u32 = 0x7000_E909;
/// Every sentinel id the link-route test files insert; `clean` deletes exactly these.
const OWNED: &[u32] = &[
    ID_WRONG,
    ID_LEGACY,
    ID_CODE,
    ID_LIMITED,
    ID_STAFF,
    ID_TEST,
    ID_CODE_ONCE,
    ID_ACK,
    ID_REFUSED,
    ID_LINK_DISABLED,
];
/// The `(provider, subject)` identities the link-route tests sign up with, including
/// `link_tests_guards_live_db.rs` (`ct26-cap-0..=49` and `ct26-cap-51`). An account
/// holding one of these is deleted; no other account matches.
fn signup_identities() -> Vec<(String, String)> {
    let dev = |s: String| ("dev".to_owned(), s);
    let mut list: Vec<(String, String)> = (1..=21)
        .filter(|n| *n != 3)
        .map(|n| dev(format!("ct26-{n}")))
        .collect();
    list.push(("untrusted".to_owned(), "ct26-3".to_owned()));
    list.extend(["ct26-30", "ct26-disabled", "ct26-cap-51"].map(|s| dev(s.to_owned())));
    list.extend((0..50).map(|i| dev(format!("ct26-cap-{i}"))));
    list
}

/// The SHA-1 (uppercase hex) of the plaintext `password`: the stored form of a
/// legacy (algo 1) row whose plaintext is `password`.
const LEGACY_HASH: &str = "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8";
const LEGACY_PASSWORD: &str = "password";
const PEER: &str = "127.0.0.1:41000";
const ONE_DAY_MS: i64 = 86_400_000;

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

/// A router under test, with the stores it shares so a test can issue grants
/// and link codes.
pub(super) struct Env {
    router: Router,
    stores: Arc<Stores>,
    codes: &'static LinkCodeStore,
}

/// Builds the routes with the default config.
pub(super) fn harness(pool: &PgPool) -> Env {
    harness_with(pool, TABLE)
}

/// Builds the routes from `table` in developer mode, the database routes reach
/// through `pool`.
fn harness_with(pool: &PgPool, table: &str) -> Env {
    let env =
        |name: &str| (name == "CIMMERIA_IDP_TEST_SECRET").then(|| "s3cr3t-test-value".to_owned());
    let (cfg, _) = config::load(table, &env, true).expect("the fixture loads");
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

/// Deletes the sentinel accounts in `OWNED` and the accounts holding a
/// `signup_identities()` identity; identities and events cascade.
pub(in crate::routes) async fn clean(pool: &PgPool) {
    let ids: Vec<i32> = OWNED.iter().map(|&id| id as i32).collect();
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await;
    let (providers, subjects): (Vec<String>, Vec<String>) = signup_identities().into_iter().unzip();
    let _ = sqlx::query(
        "DELETE FROM account WHERE account_id IN (SELECT i.account_id FROM account_identity i \
         JOIN UNNEST($1::text[], $2::text[]) AS s(provider, subject) \
         ON i.provider = s.provider AND i.subject = s.subject)",
    )
    .bind(providers)
    .bind(subjects)
    .execute(pool)
    .await;
}

/// Inserts a sentinel account. A legacy row holds `password` (the SHA-1 hex),
/// or nothing when `password` is `None`.
async fn insert_row(
    pool: &PgPool,
    account_id: u32,
    name: &str,
    password: Option<&str>,
    accesslevel: i32,
    test_account: bool,
) {
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, $3, 1, $4, true, $5)",
    )
    .bind(account_id as i32)
    .bind(name)
    .bind(password)
    .bind(accesslevel)
    .bind(test_account)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// A verified identity under `provider`, with the given subject and email.
pub(super) fn verified(
    provider: &str,
    subject: &str,
    email: Option<&str>,
    verified: bool,
) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new(provider).expect("provider id"),
        issuer: "https://issuer.ct26.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT26 Player".to_owned(),
        email: email.map(str::to_owned),
        email_verified: verified,
        provider_account_created_ms: None,
    }
}

/// Stores `identity` under a fresh link grant and returns the grant.
pub(super) fn grant(env: &Env, identity: VerifiedIdentity) -> String {
    env.stores.grants.issue(identity, Instant::now())
}

/// Sends a JSON `POST` to `uri` from `peer` and returns the status and the
/// parsed body (`Null` when there is none).
pub(super) async fn post_from(
    env: &Env,
    peer: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let peer: SocketAddr = peer.parse().expect("peer address");
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

/// Sends a JSON `POST` to `uri` from the loopback peer.
pub(super) async fn post_json(env: &Env, uri: &str, body: Value) -> (StatusCode, Value) {
    post_from(env, PEER, uri, body).await
}

/// The password form of `POST /link/existing`, from `peer`.
async fn link_password(
    env: &Env,
    peer: &str,
    grant: &str,
    name: &str,
    password: &str,
    ack: bool,
) -> (StatusCode, Value) {
    post_from(
        env,
        peer,
        "/link/existing",
        json!({
            "link_grant": grant,
            "account_name": name,
            "password": password,
            "password_removal_ack": ack,
        }),
    )
    .await
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
async fn live_db_signup_creates_a_passwordless_account() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-1", None, false));
    let (status, body) = post_json(&env, "/link/new", json!({"link_grant": g})).await;
    let id = body["account"]["account_id"].as_u64().unwrap_or(0) as u32;
    let row: Option<(String, Option<String>, Option<String>)> = if status == StatusCode::OK {
        sqlx::query_as(
            "SELECT account_name, password, password_hash_v2 FROM account WHERE account_id = $1",
        )
        .bind(id as i32)
        .fetch_one(&pool)
        .await
        .ok()
    } else {
        None
    };
    let signups = events(&pool, id, "signup").await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], "signed_in");
    let (name, password, hash) = row.expect("the new account");
    assert!(names::is_valid_account_name(&name), "{name}");
    assert_eq!(password, None);
    assert_eq!(hash, None);
    assert_eq!(signups, 1);
}

#[tokio::test]
async fn live_db_signup_stores_email_only_when_trusted_and_verified() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let trusted = grant(
        &env,
        verified("dev", "ct26-2", Some("player@example.org"), true),
    );
    let untrusted = grant(
        &env,
        verified("untrusted", "ct26-3", Some("player@example.org"), true),
    );
    let unverified = grant(
        &env,
        verified("dev", "ct26-4", Some("player3@example.org"), false),
    );
    let (s1, b1) = post_json(&env, "/link/new", json!({"link_grant": trusted})).await;
    let (s2, b2) = post_json(&env, "/link/new", json!({"link_grant": untrusted})).await;
    let (s3, b3) = post_json(&env, "/link/new", json!({"link_grant": unverified})).await;
    let id1 = b1["account"]["account_id"].as_u64().unwrap_or(0) as u32;
    let id2 = b2["account"]["account_id"].as_u64().unwrap_or(0) as u32;
    let id3 = b3["account"]["account_id"].as_u64().unwrap_or(0) as u32;
    let email_of = |id: u32| {
        let pool = pool.clone();
        async move {
            let (null,): (bool,) =
                sqlx::query_as("SELECT email IS NULL FROM account_identity WHERE account_id = $1")
                    .bind(id as i32)
                    .fetch_one(&pool)
                    .await
                    .expect("read the email");
            null
        }
    };
    let trusted_null = email_of(id1).await;
    let untrusted_null = email_of(id2).await;
    let unverified_null = email_of(id3).await;
    clean(&pool).await;
    assert_eq!(s1, StatusCode::OK, "{b1}");
    assert_eq!(s2, StatusCode::OK, "{b2}");
    assert_eq!(s3, StatusCode::OK, "{b3}");
    assert!(!trusted_null, "a trusted, verified email is stored");
    assert!(
        untrusted_null,
        "an untrusted provider's email is not stored"
    );
    assert!(
        unverified_null,
        "a trusted provider's unverified email is not stored"
    );
}

#[tokio::test]
async fn live_db_signup_closed_and_ip_limit() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let closed = harness_with(&pool, &format!("signup_open = false\n{TABLE}"));
    let g = grant(&closed, verified("dev", "ct26-4", None, false));
    let (closed_status, closed_body) =
        post_json(&closed, "/link/new", json!({"link_grant": g})).await;
    let open = harness(&pool);
    let mut statuses = Vec::new();
    for n in 5..=8 {
        let g = grant(&open, verified("dev", &format!("ct26-{n}"), None, false));
        let (status, _) = post_json(&open, "/link/new", json!({"link_grant": g})).await;
        statuses.push(status);
    }
    clean(&pool).await;
    assert_eq!(closed_status, StatusCode::FORBIDDEN, "{closed_body}");
    assert_eq!(closed_body["error"], "signup_closed");
    assert_eq!(
        statuses,
        vec![
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::TOO_MANY_REQUESTS,
        ]
    );
}

#[tokio::test]
async fn live_db_signup_refuses_young_provider_accounts() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let young = harness_with(
        &pool,
        &format!("{TABLE}\n[limits]\nmin_provider_account_age_days = 7\n"),
    );
    let mut id = verified("dev", "ct26-9", None, false);
    id.provider_account_created_ms = Some(Utc::now().timestamp_millis() - ONE_DAY_MS);
    let g = grant(&young, id);
    let (status, body) = post_json(&young, "/link/new", json!({"link_grant": g.clone()})).await;
    // The refused sign-up keeps the grant, so a second try is refused the same way.
    let (again_status, again_body) = post_json(&young, "/link/new", json!({"link_grant": g})).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "signup_closed");
    assert_eq!(again_status, StatusCode::FORBIDDEN, "{again_body}");
    assert_eq!(again_body["error"], "signup_closed");
}

#[tokio::test]
async fn live_db_signup_conflict_leaves_no_orphan_account() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let (before,): (i64,) = sqlx::query_as("SELECT last_value FROM accounts_account_id_seq")
        .fetch_one(&pool)
        .await
        .expect("sequence position");
    let first = grant(&env, verified("dev", "ct26-20", None, false));
    let (first_status, first_body) =
        post_json(&env, "/link/new", json!({"link_grant": first})).await;
    // The same identity again: the account is created, then the identity insert conflicts.
    let second = grant(&env, verified("dev", "ct26-20", None, false));
    let (second_status, second_body) =
        post_json(&env, "/link/new", json!({"link_grant": second})).await;
    // An orphan has no identity, so the exact-identity cleanup cannot reach it: the
    // test lists the accounts created after the sequence position with the sign-up
    // name shape (its own signups) and deletes those ids itself.
    let orphan_rows: Vec<(i32,)> = sqlx::query_as(
        "SELECT a.account_id FROM account a WHERE a.account_id > $1::bigint \
         AND a.account_name LIKE 'CT26Player%' \
         AND NOT EXISTS (SELECT 1 FROM account_identity i WHERE i.account_id = a.account_id)",
    )
    .bind(before)
    .fetch_all(&pool)
    .await
    .expect("list orphan accounts");
    let orphans = orphan_rows.len();
    let orphan_ids: Vec<i32> = orphan_rows.into_iter().map(|(id,)| id).collect();
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(orphan_ids)
        .execute(&pool)
        .await;
    clean(&pool).await;
    assert_eq!(first_status, StatusCode::OK, "{first_body}");
    assert_eq!(second_status, StatusCode::BAD_REQUEST, "{second_body}");
    assert_eq!(second_body["error"], "invalid_grant");
    assert_eq!(orphans, 0);
}

#[tokio::test]
async fn live_db_link_existing_wrong_password_keeps_the_grant() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(
        &pool,
        ID_WRONG,
        "ct26_wrong_516",
        Some(LEGACY_HASH),
        0,
        false,
    )
    .await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-10", None, false));
    let (wrong_status, wrong) =
        link_password(&env, PEER, &g, "ct26_wrong_516", "not-the-password", true).await;
    let (right_status, right) =
        link_password(&env, PEER, &g, "ct26_wrong_516", LEGACY_PASSWORD, true).await;
    // The successful link spent the grant, so a replay is refused before the credential.
    let (replay_status, replay) =
        link_password(&env, PEER, &g, "ct26_wrong_516", LEGACY_PASSWORD, true).await;
    clean(&pool).await;
    assert_eq!(wrong_status, StatusCode::UNAUTHORIZED, "{wrong}");
    assert_eq!(wrong["error"], "invalid_credentials");
    assert_eq!(right_status, StatusCode::OK, "{right}");
    assert_eq!(right["result"], "signed_in");
    assert_eq!(right["account"]["account_id"], ID_WRONG);
    assert_eq!(replay_status, StatusCode::BAD_REQUEST, "{replay}");
    assert_eq!(replay["error"], "invalid_grant");
}

#[tokio::test]
async fn live_db_link_existing_is_rate_limited_per_account() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(
        &pool,
        ID_LIMITED,
        "ct26_limited_519",
        Some(LEGACY_HASH),
        0,
        false,
    )
    .await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-11", None, false));
    let mut statuses = Vec::new();
    for i in 1..=6 {
        let peer = format!("203.0.113.{i}:41000");
        let (status, _) = link_password(&env, &peer, &g, "ct26_limited_519", "wrong", true).await;
        statuses.push(status);
    }
    clean(&pool).await;
    let mut expected = vec![StatusCode::UNAUTHORIZED; 5];
    expected.push(StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(statuses, expected);
}

#[tokio::test]
async fn live_db_link_existing_is_rate_limited_per_ip() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-21", None, false));
    let mut statuses = Vec::new();
    for _ in 1..=6 {
        let (status, _) = post_json(
            &env,
            "/link/existing",
            json!({
                "link_grant": g,
                "link_code": "AAAAAAAAAA",
                "password_removal_ack": true,
            }),
        )
        .await;
        statuses.push(status);
    }
    clean(&pool).await;
    let mut expected = vec![StatusCode::UNAUTHORIZED; 5];
    expected.push(StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(statuses, expected);
}

#[tokio::test]
async fn live_db_staff_and_test_accounts_are_not_linkable() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(
        &pool,
        ID_STAFF,
        "ct26_staff_520",
        Some(LEGACY_HASH),
        1,
        false,
    )
    .await;
    insert_row(&pool, ID_TEST, "ct26_test_521", Some(LEGACY_HASH), 0, true).await;
    let env = harness(&pool);
    let staff_grant = grant(&env, verified("dev", "ct26-12", None, false));
    let test_grant = grant(&env, verified("dev", "ct26-13", None, false));
    let (staff_status, staff) = link_password(
        &env,
        PEER,
        &staff_grant,
        "ct26_staff_520",
        LEGACY_PASSWORD,
        true,
    )
    .await;
    let (test_status, test) = link_password(
        &env,
        PEER,
        &test_grant,
        "ct26_test_521",
        LEGACY_PASSWORD,
        true,
    )
    .await;
    let staff_refused = events(&pool, ID_STAFF, "link_refused").await;
    let test_refused = events(&pool, ID_TEST, "link_refused").await;
    clean(&pool).await;
    assert_eq!(staff_status, StatusCode::FORBIDDEN, "{staff}");
    assert_eq!(staff["error"], "not_linkable");
    assert_eq!(test_status, StatusCode::FORBIDDEN, "{test}");
    assert_eq!(test["error"], "not_linkable");
    assert_eq!(staff_refused, 1);
    assert_eq!(test_refused, 1);
}

#[tokio::test]
async fn live_db_link_code_links_once() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(&pool, ID_CODE_ONCE, "ct26_code_522", None, 0, false).await;
    let env = harness(&pool);
    let code = env.codes.issue(ID_CODE_ONCE, Instant::now());
    let first = grant(&env, verified("dev", "ct26-14", None, false));
    let (first_status, first_body) = post_json(
        &env,
        "/link/existing",
        json!({"link_grant": first, "link_code": code, "password_removal_ack": true}),
    )
    .await;
    let second = grant(&env, verified("dev", "ct26-15", None, false));
    let (second_status, second_body) = post_json(
        &env,
        "/link/existing",
        json!({"link_grant": second, "link_code": code, "password_removal_ack": true}),
    )
    .await;
    clean(&pool).await;
    assert_eq!(first_status, StatusCode::OK, "{first_body}");
    assert_eq!(first_body["account"]["account_id"], ID_CODE_ONCE);
    assert_eq!(second_status, StatusCode::UNAUTHORIZED, "{second_body}");
    assert_eq!(second_body["error"], "invalid_credentials");
}

/// A disabled account refuses a link by code with `account_disabled`, the same
/// answer the password path gives (C6), not `not_linkable`.
#[tokio::test]
async fn live_db_link_code_to_disabled_account_is_account_disabled() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(&pool, ID_LINK_DISABLED, "ct26_disabled_525", None, 0, false).await;
    sqlx::query("UPDATE account SET enabled = false WHERE account_id = $1")
        .bind(ID_LINK_DISABLED as i32)
        .execute(&pool)
        .await
        .expect("disable sentinel account");
    let env = harness(&pool);
    let code = env.codes.issue(ID_LINK_DISABLED, Instant::now());
    let link_grant = grant(&env, verified("dev", "ct26-disabled", None, false));
    let (status, body) = post_json(
        &env,
        "/link/existing",
        json!({"link_grant": link_grant, "link_code": code, "password_removal_ack": true}),
    )
    .await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "account_disabled");
}

#[path = "link_password_tests_live_db.rs"]
mod password;
