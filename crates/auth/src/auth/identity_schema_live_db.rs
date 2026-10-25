//! Live-DB checks for the identity tables and their guard triggers (CT-10):
//! the unique subject, the cascade on account delete, the event-kind CHECK,
//! the `login_audit` ticket outcomes, and the rule that an account with a
//! linked identity holds no password. Sentinel ids `2_000_101..=2_000_107`;
//! cleanup deletes by exact id, before and after each test.

use sqlx::PgPool;

use crate::test_support::require_db_or_skip;

const ID_SUBJECT: i32 = 2_000_101;
const ID_CASCADE: i32 = 2_000_102;
const ID_EVENT_KIND: i32 = 2_000_103;
const ID_AUDIT: i32 = 2_000_104;
const ID_GUARD_SET: i32 = 2_000_105;
const ID_GUARD_LINK: i32 = 2_000_106;
const ID_CLEAR_LINK: i32 = 2_000_107;

/// Delete the sentinel rows for `account_id`. The account delete cascades to
/// its identity, refresh-token and event rows; `login_audit` has no foreign
/// key, so its rows are deleted first.
async fn cleanup(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM login_audit WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert a sentinel account, with the given legacy password or none.
async fn insert_account(pool: &PgPool, account_id: i32, password: Option<&str>) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, 1, 0, true)",
    )
    .bind(account_id)
    .bind(format!("identity_test_{account_id}"))
    .bind(password)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Insert one identity for the account, under a `discord` sentinel issuer.
async fn insert_identity(pool: &PgPool, account_id: i32, subject: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO account_identity (account_id, provider, issuer, subject) \
         VALUES ($1, 'discord', 'https://issuer.example.org', $2)",
    )
    .bind(account_id)
    .bind(subject)
    .execute(pool)
    .await
    .map(|_| ())
}

/// SQLSTATE and message of a database error, or `None` for any other error.
fn db_error(err: &sqlx::Error) -> Option<(String, String)> {
    match err {
        sqlx::Error::Database(e) => Some((
            e.code().map(|c| c.to_string()).unwrap_or_default(),
            e.message().to_string(),
        )),
        _ => None,
    }
}

#[tokio::test]
async fn live_db_identity_subject_is_unique() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_SUBJECT, None).await;
    insert_identity(&pool, ID_SUBJECT, "subject-101")
        .await
        .expect("first identity for the subject");
    let duplicate = insert_identity(&pool, ID_SUBJECT, "subject-101").await;
    cleanup(&pool, ID_SUBJECT).await;
    let err = duplicate.expect_err("a second identity with the same subject was accepted");
    let (code, _) = db_error(&err).expect("database error");
    assert_eq!(code, "23505", "expected a unique violation");
}

#[tokio::test]
async fn live_db_identity_rows_cascade_with_the_account() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_CASCADE, None).await;
    insert_identity(&pool, ID_CASCADE, "subject-102")
        .await
        .expect("identity");
    sqlx::query(
        "INSERT INTO auth_refresh_token (token_hash, family_id, account_id, family_expires_at) \
         VALUES ($1, 'family-102', $2, now() + interval '1 day')",
    )
    .bind(vec![0x02u8; 32])
    .bind(ID_CASCADE)
    .execute(&pool)
    .await
    .expect("refresh token");
    sqlx::query("INSERT INTO account_identity_event (account_id, kind) VALUES ($1, 'link')")
        .bind(ID_CASCADE)
        .execute(&pool)
        .await
        .expect("identity event");

    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ID_CASCADE)
        .execute(&pool)
        .await
        .expect("delete the sentinel account");

    let mut remaining = 0i64;
    for sql in [
        "SELECT COUNT(*) FROM account_identity WHERE account_id = $1",
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1",
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1",
    ] {
        let (n,): (i64,) = sqlx::query_as(sql)
            .bind(ID_CASCADE)
            .fetch_one(&pool)
            .await
            .expect("count rows");
        remaining += n;
    }
    cleanup(&pool, ID_CASCADE).await;
    assert_eq!(
        remaining, 0,
        "rows were left behind after the account delete"
    );
}

#[tokio::test]
async fn live_db_identity_event_kind_is_checked() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_EVENT_KIND, None).await;
    let bogus =
        sqlx::query("INSERT INTO account_identity_event (account_id, kind) VALUES ($1, 'bogus')")
            .bind(ID_EVENT_KIND)
            .execute(&pool)
            .await;
    let accepted = sqlx::query(
        "INSERT INTO account_identity_event (account_id, kind) VALUES ($1, 'link_code_issued')",
    )
    .bind(ID_EVENT_KIND)
    .execute(&pool)
    .await;
    cleanup(&pool, ID_EVENT_KIND).await;
    let err = bogus.expect_err("event kind 'bogus' was accepted");
    let (code, _) = db_error(&err).expect("database error");
    assert_eq!(code, "23514", "expected a check violation");
    accepted.expect("event kind 'link_code_issued' was refused");
}

#[tokio::test]
async fn live_db_login_audit_accepts_ticket_outcomes() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ID_AUDIT).await;
    let mut refused = Vec::new();
    for outcome in ["launch_ticket", "ticket_requires_tls"] {
        let inserted = sqlx::query(
            "INSERT INTO login_audit (account_name, account_id, ip_address, phase, outcome) \
             VALUES ($1, $2, $3::INET, 'credential_check', $4)",
        )
        .bind(format!("identity_test_{ID_AUDIT}"))
        .bind(ID_AUDIT)
        .bind("203.0.113.7")
        .bind(outcome)
        .execute(&pool)
        .await;
        if let Err(e) = inserted {
            refused.push(format!("{outcome}: {e}"));
        }
    }
    cleanup(&pool, ID_AUDIT).await;
    assert!(refused.is_empty(), "login_audit refused: {refused:#?}");
}

#[tokio::test]
async fn live_db_password_refused_on_an_account_with_an_identity() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_GUARD_SET, None).await;
    insert_identity(&pool, ID_GUARD_SET, "subject-105")
        .await
        .expect("identity");
    let set = sqlx::query("UPDATE account SET password_hash_v2 = 'x' WHERE account_id = $1")
        .bind(ID_GUARD_SET)
        .execute(&pool)
        .await;
    let cleared = sqlx::query(
        "UPDATE account SET password = NULL, password_hash_v2 = NULL WHERE account_id = $1",
    )
    .bind(ID_GUARD_SET)
    .execute(&pool)
    .await;
    cleanup(&pool, ID_GUARD_SET).await;
    let err = set.expect_err("a password was set on an account with an identity");
    let (code, message) = db_error(&err).expect("database error");
    assert_eq!(code, "23514", "expected a check violation");
    assert!(
        message.contains("account_has_identity"),
        "unexpected message: {message}"
    );
    cleared.expect("clearing both password columns was refused");
}

#[tokio::test]
async fn live_db_identity_refused_while_the_account_has_a_password() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_GUARD_LINK, Some("legacy-test-hash")).await;
    let linked = insert_identity(&pool, ID_GUARD_LINK, "subject-106").await;
    cleanup(&pool, ID_GUARD_LINK).await;
    let err = linked.expect_err("an identity was linked while the account has a password");
    let (code, message) = db_error(&err).expect("database error");
    assert_eq!(code, "23514", "expected a check violation");
    assert!(
        message.contains("account_has_password"),
        "unexpected message: {message}"
    );
}

#[tokio::test]
async fn live_db_clear_then_link_in_one_transaction() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_CLEAR_LINK, Some("legacy-test-hash")).await;
    let mut tx = pool.begin().await.expect("begin");
    sqlx::query(
        "UPDATE account SET password = NULL, password_hash_v2 = NULL WHERE account_id = $1",
    )
    .bind(ID_CLEAR_LINK)
    .execute(&mut *tx)
    .await
    .expect("clear the password");
    sqlx::query(
        "INSERT INTO account_identity (account_id, provider, issuer, subject) \
         VALUES ($1, 'discord', 'https://issuer.example.org', 'subject-107')",
    )
    .bind(ID_CLEAR_LINK)
    .execute(&mut *tx)
    .await
    .expect("link under the guards");
    let committed = tx.commit().await;
    let row: Result<(Option<String>, Option<String>), sqlx::Error> =
        sqlx::query_as("SELECT password, password_hash_v2 FROM account WHERE account_id = $1")
            .bind(ID_CLEAR_LINK)
            .fetch_one(&pool)
            .await;
    cleanup(&pool, ID_CLEAR_LINK).await;
    committed.expect("commit refused");
    assert_eq!(row.expect("read the account back"), (None, None));
}
