//! Live-DB tests of the per-family refresh lock (contract C16.1). A rotation or
//! an account revoke that meets a successor inserted by another connection must
//! not leave a live token behind.

use std::future::Future;

use chrono::TimeZone;
use cimmeria_test_support::{database_url, require_db_or_skip};
use sqlx::{Connection, PgPool};

use super::*;

/// The sentinel accounts, one per test so the tests can run side by side.
const ACCOUNT_REUSE: i32 = 0x7000_E000;
const ACCOUNT_REVOKE: i32 = 0x7000_E001;
const ACCOUNT_KEEP: i32 = 0x7000_E002;

/// The fixed clock every test starts from.
fn base() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 26, 12, 0, 0)
        .single()
        .expect("a valid date")
}

/// The clock `days` after [`base`].
fn at(days: i64) -> DateTime<Utc> {
    base() + Duration::days(days)
}

/// Delete the token rows and the account row of `account_id`, by exact id.
async fn cleanup(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM auth_refresh_token WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert a sentinel account with no password, after clearing any old one.
async fn insert_account(pool: &PgPool, account_id: i32) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, 0, true)",
    )
    .bind(account_id)
    .bind(format!("ct71_family_lock_{account_id}"))
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// The number of live (unrevoked) token rows of one family.
async fn live_rows(pool: &PgPool, family_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_refresh_token WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .fetch_one(pool)
    .await
    .expect("count live rows")
}

/// The new token of a [`Rotation::Rotated`], or a panic naming the failure.
fn rotated_token(outcome: Result<Rotation, RepoError>) -> String {
    match outcome.expect("rotate") {
        Rotation::Rotated { token, .. } => token,
        _ => panic!("a live token rotates"),
    }
}

/// Polls `pg_locks` for up to 10 s until a lock request in the test database
/// is waiting. Gives up after that, so a contender that never blocks still
/// lets the test finish.
async fn wait_for_a_waiting_lock(pool: &PgPool) {
    for _ in 0..100 {
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks l \
             JOIN pg_database d ON d.oid = l.database \
             WHERE NOT l.granted AND d.datname = current_database())",
        )
        .fetch_one(pool)
        .await
        .expect("read pg_locks");
        if waiting {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// Runs `contender` while a second connection holds the family lock and has
/// replaced `b`, the family's live token, with a new live successor. Once the
/// contender is waiting on the lock, or after 10 s, the successor commits and
/// the contender's output is returned.
async fn against_a_successor<T>(
    pool: &PgPool,
    family_id: &str,
    b: &str,
    contender: impl Future<Output = T>,
) -> T {
    let url = database_url().expect("the live-DB gate passed");
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect the second connection");
    let mut tx = conn.begin().await.expect("begin");
    family_lock(&mut tx, family_id)
        .await
        .expect("take the family lock");
    let b_hash = hash_of(b);
    sqlx::query("SELECT 1 FROM auth_refresh_token WHERE token_hash = $1 FOR UPDATE")
        .bind(&b_hash[..])
        .fetch_all(&mut *tx)
        .await
        .expect("lock B");
    let (_, c_hash) = new_token();
    let now_ts = micros(at(2));
    sqlx::query(
        "UPDATE auth_refresh_token SET replaced_by = $2, revoked_at = $3::timestamptz, \
         last_used_at = $3::timestamptz WHERE token_hash = $1",
    )
    .bind(&b_hash[..])
    .bind(&c_hash[..])
    .bind(&now_ts)
    .execute(&mut *tx)
    .await
    .expect("replace B");
    sqlx::query(
        "INSERT INTO auth_refresh_token \
         (token_hash, family_id, account_id, issued_at, last_used_at, family_expires_at) \
         SELECT $1::bytea, family_id, account_id, $2::timestamptz, $2::timestamptz, family_expires_at \
         FROM auth_refresh_token WHERE token_hash = $3::bytea",
    )
    .bind(&c_hash[..])
    .bind(&now_ts)
    .bind(&b_hash[..])
    .execute(&mut *tx)
    .await
    .expect("insert the live successor C");
    let release = async {
        wait_for_a_waiting_lock(pool).await;
        tx.commit().await.expect("commit the successor");
    };
    let (out, ()) = tokio::join!(contender, release);
    out
}

#[tokio::test]
async fn live_db_reuse_during_a_successor_rotation_revokes_the_successor() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_REUSE).await;
    let (a, family) = issue(&pool, ACCOUNT_REUSE as u32, None, at(0))
        .await
        .expect("issue");
    let b = rotated_token(rotate(&pool, &a, at(1)).await);
    let outcome =
        against_a_successor(&pool, &family, &b, async { rotate(&pool, &a, at(3)).await }).await;
    let live = live_rows(&pool, &family).await;
    cleanup(&pool, ACCOUNT_REUSE).await;
    let Rotation::Reused {
        account_id,
        family_id,
    } = outcome.expect("rotate the spent token")
    else {
        panic!("a replayed token is reuse");
    };
    assert_eq!(account_id, ACCOUNT_REUSE as u32);
    assert_eq!(family_id, family);
    assert_eq!(live, 0, "the successor is revoked with its family");
}

#[tokio::test]
async fn live_db_account_revoke_waits_for_an_inflight_rotation() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_REVOKE).await;
    let (a, family) = issue(&pool, ACCOUNT_REVOKE as u32, None, at(0))
        .await
        .expect("issue");
    let b = rotated_token(rotate(&pool, &a, at(1)).await);
    let outcome = against_a_successor(&pool, &family, &b, async {
        revoke_account(&pool, ACCOUNT_REVOKE as u32, at(3)).await
    })
    .await;
    let live = live_rows(&pool, &family).await;
    cleanup(&pool, ACCOUNT_REVOKE).await;
    outcome.expect("revoke the account");
    assert_eq!(live, 0, "no live row is left for the account");
}

#[tokio::test]
async fn live_db_revoke_account_on_keeps_the_named_family() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_KEEP).await;
    let (_, keep) = issue(&pool, ACCOUNT_KEEP as u32, None, at(0))
        .await
        .expect("issue the kept family");
    let (_, other) = issue(&pool, ACCOUNT_KEEP as u32, None, at(0))
        .await
        .expect("issue the other family");
    let mut tx = pool.begin().await.expect("begin");
    let revoked = revoke_account_on(&mut tx, ACCOUNT_KEEP as u32, Some(keep.as_str()), at(3))
        .await
        .expect("revoke the account except the kept family");
    tx.commit().await.expect("commit");
    let keep_live = live_rows(&pool, &keep).await;
    let other_live = live_rows(&pool, &other).await;
    cleanup(&pool, ACCOUNT_KEEP).await;
    assert_eq!(
        revoked,
        vec![other.clone()],
        "only the other family is revoked"
    );
    assert_eq!(keep_live, 1, "the kept family stays live");
    assert_eq!(other_live, 0, "the other family is revoked");
}
