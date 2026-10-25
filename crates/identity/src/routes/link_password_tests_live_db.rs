//! Live-DB tests of the password clearing on a link (C15): `link/existing` and a
//! link code clear the legacy password, the removal ack is required, and a refused
//! link keeps the password.

use cimmeria_auth::auth::{verify_account_password, PasswordCheck};

use super::*;

/// The `(password, password_hash_v2)` columns of `account_id`.
async fn password_of(pool: &PgPool, account_id: u32) -> (Option<String>, Option<String>) {
    sqlx::query_as("SELECT password, password_hash_v2 FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .fetch_one(pool)
        .await
        .expect("read the password columns")
}

#[tokio::test]
async fn live_db_link_existing_clears_the_password() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(
        &pool,
        ID_LEGACY,
        "ct26_legacy_517",
        Some(LEGACY_HASH),
        0,
        false,
    )
    .await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-16", None, false));
    let (status, body) =
        link_password(&env, PEER, &g, "ct26_legacy_517", LEGACY_PASSWORD, true).await;
    let cleared = password_of(&pool, ID_LEGACY).await;
    let cleared_events = events(&pool, ID_LEGACY, "password_cleared").await;
    let link_events = events(&pool, ID_LEGACY, "link").await;
    let old = verify_account_password(&pool, "ct26_legacy_517", LEGACY_PASSWORD).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["account"]["account_id"], ID_LEGACY);
    assert_eq!(cleared, (None, None));
    assert_eq!(cleared_events, 1);
    assert_eq!(link_events, 1);
    assert!(
        matches!(old, PasswordCheck::Invalid),
        "the old password still checks"
    );
}

#[tokio::test]
async fn live_db_link_code_clears_the_password() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(&pool, ID_CODE, "ct26_code_518", Some(LEGACY_HASH), 0, false).await;
    let env = harness(&pool);
    let code = env.codes.issue(ID_CODE, Instant::now());
    let g = grant(&env, verified("dev", "ct26-17", None, false));
    let (status, body) = post_json(
        &env,
        "/link/existing",
        json!({"link_grant": g, "link_code": code, "password_removal_ack": true}),
    )
    .await;
    let cleared = password_of(&pool, ID_CODE).await;
    let cleared_events = events(&pool, ID_CODE, "password_cleared").await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(cleared, (None, None));
    assert_eq!(cleared_events, 1);
}

#[tokio::test]
async fn live_db_link_existing_needs_the_removal_ack() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(&pool, ID_ACK, "ct26_ack_523", Some(LEGACY_HASH), 0, false).await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-18", None, false));
    let (absent_status, absent) = post_json(
        &env,
        "/link/existing",
        json!({"link_grant": g, "account_name": "ct26_ack_523", "password": LEGACY_PASSWORD}),
    )
    .await;
    let (false_status, false_body) =
        link_password(&env, PEER, &g, "ct26_ack_523", LEGACY_PASSWORD, false).await;
    let kept = password_of(&pool, ID_ACK).await;
    let (ok_status, ok_body) =
        link_password(&env, PEER, &g, "ct26_ack_523", LEGACY_PASSWORD, true).await;
    clean(&pool).await;
    assert_eq!(absent_status, StatusCode::BAD_REQUEST, "{absent}");
    assert_eq!(absent["error"], "invalid_request");
    assert_eq!(false_status, StatusCode::BAD_REQUEST, "{false_body}");
    assert_eq!(false_body["error"], "invalid_request");
    assert_eq!(kept, (Some(LEGACY_HASH.to_owned()), None));
    assert_eq!(ok_status, StatusCode::OK, "{ok_body}");
}

#[tokio::test]
async fn live_db_refused_link_keeps_the_password() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_row(
        &pool,
        ID_REFUSED,
        "ct26_refused_524",
        Some(LEGACY_HASH),
        0,
        true,
    )
    .await;
    let env = harness(&pool);
    let g = grant(&env, verified("dev", "ct26-19", None, false));
    let (status, body) =
        link_password(&env, PEER, &g, "ct26_refused_524", LEGACY_PASSWORD, true).await;
    let kept = password_of(&pool, ID_REFUSED).await;
    clean(&pool).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "not_linkable");
    assert_eq!(kept, (Some(LEGACY_HASH.to_owned()), None));
}
