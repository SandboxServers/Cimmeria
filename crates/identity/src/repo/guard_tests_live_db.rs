//! Live-DB tests of the identity guard's account row lock (CT-81, F14). An
//! identity insert and a password write on one passwordless account serialize,
//! and moving an identity to a passworded account is refused. Each race holds
//! an open transaction on a second connection, then commits it. Sentinel ids
//! `0x7000_E600..=0x7000_E605`; cleanup deletes by exact id.

use cimmeria_test_support::{database_url, require_db_or_skip};
use sqlx::{Connection, PgConnection, PgExecutor, PgPool};

/// The sentinel accounts, one per test so the tests can run side by side.
const ACCOUNT_SET_WAITS: i32 = 0x7000_E600;
const ACCOUNT_MOVE_FROM: i32 = 0x7000_E601;
const ACCOUNT_MOVE_PASSWORDED: i32 = 0x7000_E602;
const ACCOUNT_MOVE_SOURCE: i32 = 0x7000_E603;
const ACCOUNT_MOVE_TARGET: i32 = 0x7000_E604;
const ACCOUNT_INSERT_WAITS: i32 = 0x7000_E605;

/// A 40-hex legacy password, the shape the password writes use.
const PASSWORD: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678";

/// Delete the sentinel accounts by exact id. Their identity and event rows
/// cascade with them. A failed delete is ignored so it never hides an assertion.
async fn cleanup(pool: &PgPool, account_ids: &[i32]) {
    for account_id in account_ids {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }
}

/// Insert a sentinel account, with the given password or none, after clearing
/// any old one.
async fn insert_account(pool: &PgPool, account_id: i32, password: Option<&str>) {
    cleanup(pool, &[account_id]).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, 1, 0, true)",
    )
    .bind(account_id)
    .bind(format!("ct81_guard_{account_id}"))
    .bind(password)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Insert one `discord` identity for the account.
async fn insert_identity<'c, E: PgExecutor<'c>>(
    executor: E,
    account_id: i32,
    subject: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_identity (account_id, provider, issuer, subject) \
         VALUES ($1, 'discord', 'https://issuer.example.org', $2)",
    )
    .bind(account_id)
    .bind(subject)
    .execute(executor)
    .await
    .map(|_| ())
}

/// Set the password of the account, returning the rows changed.
async fn set_password<'c, E: PgExecutor<'c>>(
    executor: E,
    account_id: i32,
) -> Result<u64, sqlx::Error> {
    sqlx::query("UPDATE account SET password = $2 WHERE account_id = $1")
        .bind(account_id)
        .bind(PASSWORD)
        .execute(executor)
        .await
        .map(|result| result.rows_affected())
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

/// Asserts that the write was refused with SQLSTATE `23514` and `message`.
fn assert_refused<T: std::fmt::Debug>(outcome: Result<T, sqlx::Error>, message: &str) {
    let error = outcome.expect_err("the write is refused");
    let db = error.as_database_error().expect("a database error");
    assert_eq!(db.code().as_deref(), Some("23514"));
    assert_eq!(db.message(), message);
}

#[tokio::test]
async fn live_db_password_set_waits_for_an_uncommitted_identity_insert() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_SET_WAITS, None).await;
    let url = database_url().expect("the live-DB gate passed");
    let mut holder = PgConnection::connect(&url)
        .await
        .expect("connect the second connection");
    let mut tx = holder.begin().await.expect("begin");
    insert_identity(&mut *tx, ACCOUNT_SET_WAITS, "ct81-set-waits")
        .await
        .expect("insert the identity on connection 1");
    let contender = tokio::spawn({
        let pool = pool.clone();
        async move { set_password(&pool, ACCOUNT_SET_WAITS).await }
    });
    wait_for_a_waiting_lock(&pool).await;
    tx.commit().await.expect("commit the identity");
    let outcome = contender.await.expect("join the password write");
    cleanup(&pool, &[ACCOUNT_SET_WAITS]).await;
    assert_refused(outcome, "account_has_identity");
}

#[tokio::test]
async fn live_db_identity_insert_waits_for_an_uncommitted_password_set() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_INSERT_WAITS, None).await;
    let url = database_url().expect("the live-DB gate passed");
    let mut holder = PgConnection::connect(&url)
        .await
        .expect("connect the second connection");
    let mut tx = holder.begin().await.expect("begin");
    set_password(&mut *tx, ACCOUNT_INSERT_WAITS)
        .await
        .expect("set the password on connection 1");
    let contender = tokio::spawn({
        let pool = pool.clone();
        async move { insert_identity(&pool, ACCOUNT_INSERT_WAITS, "ct81-insert-waits").await }
    });
    wait_for_a_waiting_lock(&pool).await;
    tx.commit().await.expect("commit the password");
    let outcome = contender.await.expect("join the identity insert");
    cleanup(&pool, &[ACCOUNT_INSERT_WAITS]).await;
    assert_refused(outcome, "account_has_password");
}

#[tokio::test]
async fn live_db_moving_an_identity_to_a_passworded_account_is_refused() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_MOVE_FROM, None).await;
    insert_account(&pool, ACCOUNT_MOVE_PASSWORDED, None).await;
    insert_identity(&pool, ACCOUNT_MOVE_FROM, "ct81-move-refused")
        .await
        .expect("insert the identity");
    set_password(&pool, ACCOUNT_MOVE_PASSWORDED)
        .await
        .expect("set the password of the target");
    let outcome = sqlx::query("UPDATE account_identity SET account_id = $2 WHERE account_id = $1")
        .bind(ACCOUNT_MOVE_FROM)
        .bind(ACCOUNT_MOVE_PASSWORDED)
        .execute(&pool)
        .await
        .map(|result| result.rows_affected());
    cleanup(&pool, &[ACCOUNT_MOVE_FROM, ACCOUNT_MOVE_PASSWORDED]).await;
    assert_refused(outcome, "account_has_password");
}

#[tokio::test]
async fn live_db_moving_an_identity_to_a_passwordless_account_succeeds() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_MOVE_SOURCE, None).await;
    insert_account(&pool, ACCOUNT_MOVE_TARGET, None).await;
    insert_identity(&pool, ACCOUNT_MOVE_SOURCE, "ct81-move-allowed")
        .await
        .expect("insert the identity");
    let moved = sqlx::query("UPDATE account_identity SET account_id = $2 WHERE account_id = $1")
        .bind(ACCOUNT_MOVE_SOURCE)
        .bind(ACCOUNT_MOVE_TARGET)
        .execute(&pool)
        .await
        .map(|result| result.rows_affected());
    cleanup(&pool, &[ACCOUNT_MOVE_SOURCE, ACCOUNT_MOVE_TARGET]).await;
    assert_eq!(moved.expect("move the identity"), 1, "one identity moves");
}
