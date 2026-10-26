//! Live-DB check that each route test file's `clean` deletes only the rows it
//! inserted (F19). Three near-match rows look like test rows but are not theirs:
//! an account named `CT26PlayerKeep`, an account named `CT30-keep`, and an
//! account whose identities have provider `other` and subjects `ct26-keep`, `ct26-1`
//! and `ct30-21` (a subject of a `clean` list under a foreign provider). Every
//! `clean` must leave all three in place; they are deleted by exact id at the end.

use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::account::tests_live_db::clean as account_clean;
use super::link::tests_live_db::clean as link_clean;
use super::token::tests_live_db::clean as token_clean;

/// The `CT26PlayerKeep` near-match account.
const KEEP_CT26: u32 = 0x7000_E9F0;
/// The `CT30-keep` near-match account.
const KEEP_CT30: u32 = 0x7000_E9F1;
/// The account holding the `other` / `ct26-keep` near-match identity.
const KEEP_OTHER: u32 = 0x7000_E9F2;

/// Every near-match id; the test deletes exactly these.
const KEEP: &[u32] = &[KEEP_CT26, KEEP_CT30, KEEP_OTHER];

/// Deletes the near-match accounts by exact id; their identities cascade.
async fn delete_exact(pool: &PgPool) {
    let ids: Vec<i32> = KEEP.iter().map(|&id| id as i32).collect();
    let _ = sqlx::query("DELETE FROM account WHERE account_id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await;
}

/// Inserts a passwordless, enabled account with the given id and name.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str) {
    sqlx::query(
        "INSERT INTO account \
         (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
         VALUES ($1, $2, NULL, 1, 0, true, false)",
    )
    .bind(account_id as i32)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert near-match account");
}

/// Inserts the `other` identity with `subject` for `account_id`.
async fn insert_identity(pool: &PgPool, account_id: u32, subject: &str) {
    sqlx::query(
        "INSERT INTO account_identity (account_id, provider, issuer, subject, display_name) \
         VALUES ($1, 'other', 'https://issuer.example.org', $2, 'Keep')",
    )
    .bind(account_id as i32)
    .bind(subject)
    .execute(pool)
    .await
    .expect("insert near-match identity");
}

/// Counts the near-match accounts that still exist, by exact id.
async fn surviving(pool: &PgPool) -> i64 {
    let ids: Vec<i32> = KEEP.iter().map(|&id| id as i32).collect();
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM account WHERE account_id = ANY($1)")
            .bind(ids)
            .fetch_one(pool)
            .await
            .expect("count near-match accounts");
    count
}

#[tokio::test]
async fn live_db_cleanup_keeps_near_match_rows() {
    let pool = require_db_or_skip!();
    delete_exact(&pool).await;
    insert_account(&pool, KEEP_CT26, "CT26PlayerKeep").await;
    insert_account(&pool, KEEP_CT30, "CT30-keep").await;
    insert_account(&pool, KEEP_OTHER, "Keep_Other").await;
    for subject in ["ct26-keep", "ct26-1", "ct30-21"] {
        insert_identity(&pool, KEEP_OTHER, subject).await;
    }
    token_clean(&pool).await;
    account_clean(&pool).await;
    link_clean(&pool).await;
    let survived = surviving(&pool).await;
    delete_exact(&pool).await;
    assert_eq!(survived, 3, "each clean must keep every near-match row");
}
