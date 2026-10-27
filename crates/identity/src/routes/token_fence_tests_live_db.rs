//! Live-DB check of the access-token fence (CT-72, C16.3): a refresh whose
//! family was revoked after the request captured its fence mints no access
//! token. `session` is called directly with the fence captured before the
//! revoke, since the route only adds that capture on entry. Sentinel account id
//! `0x7000_E0C0`; cleanup deletes by that id, before and after the test.

use std::time::Instant;

use chrono::Utc;
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{session, span_tests::test_state};
use crate::repo::refresh;
use crate::routes::error::ApiError;

const ACC_FENCE: u32 = 0x7000_E0C0;

/// Removes the sentinel account and its refresh rows.
async fn clean(pool: &PgPool) {
    sqlx::query("DELETE FROM auth_refresh_token WHERE account_id = $1")
        .bind(ACC_FENCE as i32)
        .execute(pool)
        .await
        .expect("the sentinel refresh rows are removed");
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACC_FENCE as i32)
        .execute(pool)
        .await
        .expect("the sentinel account is removed");
}

/// Inserts an enabled sentinel account, so `session` finds its name.
async fn insert_account(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, NULL, 1, 0, true, false)",
    )
    .bind(ACC_FENCE as i32)
    .bind(format!("FENCE-{ACC_FENCE}"))
    .execute(pool)
    .await
    .expect("the sentinel account is inserted");
}

/// After a family revoke that ran after the fence, `session` answers
/// `invalid_grant`, mints no access token and revokes the family's refresh rows.
#[tokio::test]
async fn live_db_refresh_after_family_revoke_mints_no_access_token() {
    let pool = require_db_or_skip!();
    clean(&pool).await;
    insert_account(&pool).await;
    let state = test_state();
    let now = Utc::now();
    let (refresh_token, family_id) = refresh::issue(&pool, ACC_FENCE, None, now)
        .await
        .expect("a refresh token is issued");
    // The fence is captured as the request arrives, before the revoke.
    let fence = state.ctx.stores.access.fence();
    state.ctx.stores.access.revoke_family(&family_id);

    let answer = session(
        &state,
        &pool,
        ACC_FENCE,
        (refresh_token, family_id.clone()),
        false,
        fence,
        Instant::now(),
    )
    .await;
    let refused = matches!(answer, Err(ApiError::InvalidGrant));
    let (live_rows,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM auth_refresh_token WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(&family_id)
    .fetch_one(&pool)
    .await
    .expect("the family's rows are counted");
    clean(&pool).await;
    assert!(refused, "a stale fence is refused with invalid_grant");
    assert_eq!(live_rows, 0, "the refused family's rows are revoked");
}
