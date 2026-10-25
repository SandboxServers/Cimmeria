//! Live-DB tests of `DELETE /identities/{id}`: the reauth window, the last-method
//! and cross-account refusals, the concurrent unlink and the D1 session ending.

use chrono::Utc;

use super::*;
use crate::repo::refresh;

#[tokio::test]
async fn live_db_unlink_needs_a_fresh_code_token() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_MAIN, true).await;
    let first = insert_identity(&pool, ACC_MAIN, 3).await;
    insert_identity(&pool, ACC_MAIN, 4).await;
    let env = harness(&pool);
    let refresh = token(&env, ACC_MAIN, false, Instant::now());
    let (status, _, body) = send(
        &env.router,
        "DELETE",
        &format!("/identities/{first}"),
        Some(&refresh),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "reauth_required");
    assert!(identity_exists(&pool, first).await);
    assert_eq!(unlink_events(&pool, ACC_MAIN).await, 0);
    let code = code_token(&env, ACC_MAIN);
    assert_eq!(unlink(&env, &code, first).await, StatusCode::NO_CONTENT);
    assert!(!identity_exists(&pool, first).await);
    assert_eq!(unlink_events(&pool, ACC_MAIN).await, 1);
}

#[tokio::test]
async fn live_db_unlink_refuses_a_stale_code_token() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_MAIN, true).await;
    let first = insert_identity(&pool, ACC_MAIN, 5).await;
    insert_identity(&pool, ACC_MAIN, 6).await;
    let env = harness(&pool);
    // Issued 11 minutes ago: still a valid access token, past the 10 minute reauth window.
    let old = Instant::now()
        .checked_sub(Duration::from_secs(11 * 60))
        .expect("an instant 11 minutes back");
    let stale = token(&env, ACC_MAIN, true, old);
    let (status, _, body) = send(
        &env.router,
        "DELETE",
        &format!("/identities/{first}"),
        Some(&stale),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "reauth_required");
    assert!(identity_exists(&pool, first).await);
}

#[tokio::test]
async fn live_db_unlink_refuses_the_last_method() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_LAST, true).await;
    let only = insert_identity(&pool, ACC_LAST, 7).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_LAST);
    let (status, _, body) = send(
        &env.router,
        "DELETE",
        &format!("/identities/{only}"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "last_method");
    assert_eq!(methods(&pool, ACC_LAST).await, 1);
    let second = insert_identity(&pool, ACC_LAST, 8).await;
    assert_eq!(unlink(&env, &t, only).await, StatusCode::NO_CONTENT);
    assert_eq!(unlink(&env, &t, second).await, StatusCode::CONFLICT);
    assert_eq!(methods(&pool, ACC_LAST).await, 1);
}

#[tokio::test]
async fn live_db_unlink_of_another_accounts_identity_is_404() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_MAIN, true).await;
    insert_account(&pool, ACC_OTHER, true).await;
    insert_identity(&pool, ACC_MAIN, 9).await;
    insert_identity(&pool, ACC_MAIN, 10).await;
    let theirs = insert_identity(&pool, ACC_OTHER, 11).await;
    insert_identity(&pool, ACC_OTHER, 12).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_MAIN);
    assert_eq!(unlink(&env, &t, theirs).await, StatusCode::NOT_FOUND);
    assert!(identity_exists(&pool, theirs).await);
    assert_eq!(methods(&pool, ACC_OTHER).await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn live_db_unlink_concurrent_leaves_one_method() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_CONCURRENT, true).await;
    let a = insert_identity(&pool, ACC_CONCURRENT, 13).await;
    let b = insert_identity(&pool, ACC_CONCURRENT, 14).await;
    let env = harness(&pool);
    let t = code_token(&env, ACC_CONCURRENT);
    let (first, second) = tokio::join!(unlink(&env, &t, a), unlink(&env, &t, b));
    let mut statuses = [first, second];
    statuses.sort_by_key(|status| status.as_u16());
    assert_eq!(statuses, [StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
    assert_eq!(methods(&pool, ACC_CONCURRENT).await, 1);
}

/// D1: an unlink ends the account's other sessions (the other refresh family,
/// its access tokens and the launch tickets) and keeps the caller's own family.
#[tokio::test]
async fn live_db_unlink_ends_the_other_sessions_and_keeps_the_callers() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool, ACC_UNLINK, true).await;
    let first = insert_identity(&pool, ACC_UNLINK, 1).await;
    insert_identity(&pool, ACC_UNLINK, 2).await;
    let now = Utc::now();
    let own = refresh::issue(&pool, ACC_UNLINK, None, now)
        .await
        .expect("own family");
    let other = refresh::issue(&pool, ACC_UNLINK, None, now)
        .await
        .expect("other family");
    let env = harness(&pool);
    let caller = env
        .stores
        .access
        .mint(ACC_UNLINK, &own.1, true, Instant::now());
    let other_token = env
        .stores
        .access
        .mint(ACC_UNLINK, &other.1, false, Instant::now());
    env.tickets.mint(
        ACC_UNLINK,
        &format!("CT30-{ACC_UNLINK}"),
        LAUNCH,
        Instant::now(),
    );
    assert_eq!(unlink(&env, &caller, first).await, StatusCode::NO_CONTENT);
    assert!(env
        .stores
        .access
        .check(&other_token, Instant::now())
        .is_none());
    assert!(env.stores.access.check(&caller, Instant::now()).is_some());
    assert_eq!(env.tickets.live_count(), 0);
    let families: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT family_id FROM auth_refresh_token \
         WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(ACC_UNLINK as i32)
    .fetch_all(&pool)
    .await
    .expect("live families");
    assert_eq!(families, vec![(own.1.clone(),)]);
}
