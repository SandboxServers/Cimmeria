//! Live-DB tests of the identity-bound issue (contract C16.2). A refresh family
//! is issued only while its account is enabled and its identity still linked,
//! and an unlink in flight is waited for, not raced.

use chrono::TimeZone;
use cimmeria_test_support::{database_url, require_db_or_skip};
use sqlx::{Connection, PgConnection, PgPool};

use super::*;
use crate::provider::{ProviderId, VerifiedIdentity};
use crate::repo::identity;

/// The sentinel accounts, one per test so the tests can run side by side. The
/// range is `0x7000_E100..=0x7000_E1FF` (R6).
const ACCOUNT_UNLINKED: u32 = 0x7000_E100;
const ACCOUNT_LOCKED: u32 = 0x7000_E101;
const ACCOUNT_DISABLED: u32 = 0x7000_E102;
const ACCOUNT_LINKED: u32 = 0x7000_E103;

/// The fixed clock every test issues at.
fn base() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 26, 12, 0, 0)
        .single()
        .expect("a valid date")
}

/// Delete the token rows and the account row of `account_id`, by exact id. The
/// account's identities and events go with it (ON DELETE CASCADE).
async fn cleanup(pool: &PgPool, account_id: u32) {
    let _ = sqlx::query("DELETE FROM auth_refresh_token WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
}

/// Insert a sentinel account with no password, after clearing any old one.
async fn insert_account(pool: &PgPool, account_id: u32, enabled: bool) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, 0, $3)",
    )
    .bind(account_id as i32)
    .bind(format!("ct73_bound_{account_id}"))
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Links a dev-provider identity with the given subject to `account_id` and
/// returns its `identity_id`.
async fn link_identity(pool: &PgPool, account_id: u32, subject: &str) -> i64 {
    let verified = VerifiedIdentity {
        provider: ProviderId::new("dev").expect("provider id"),
        issuer: "https://issuer.ct73.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT73 Player".to_owned(),
        email: None,
        email_verified: false,
        provider_account_created_ms: None,
    };
    identity::insert(pool, account_id, &verified, false)
        .await
        .expect("link the identity")
        .identity_id
}

/// The number of token rows (live or spent) of one account.
async fn token_rows(pool: &PgPool, account_id: u32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1")
        .bind(account_id as i32)
        .fetch_one(pool)
        .await
        .expect("count token rows")
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

#[tokio::test]
async fn live_db_issue_bound_refuses_an_unlinked_identity() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_UNLINKED, true).await;
    let identity_id = link_identity(&pool, ACCOUNT_UNLINKED, "ct73-unlinked").await;
    sqlx::query("DELETE FROM account_identity WHERE identity_id = $1")
        .bind(identity_id)
        .execute(&pool)
        .await
        .expect("unlink the identity");

    let outcome = issue_bound(&pool, ACCOUNT_UNLINKED, identity_id, base())
        .await
        .expect("issue_bound");
    assert!(
        matches!(outcome, Issued::Unbound),
        "an unlinked identity must not earn a family"
    );
    assert_eq!(token_rows(&pool, ACCOUNT_UNLINKED).await, 0);
    cleanup(&pool, ACCOUNT_UNLINKED).await;
}

#[tokio::test]
async fn live_db_issue_bound_waits_for_an_unlink_in_flight() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_LOCKED, true).await;
    let identity_id = link_identity(&pool, ACCOUNT_LOCKED, "ct73-locked").await;

    // The unlink holds the account row `FOR UPDATE` across its delete, as the
    // player and GM unlinks do.
    let url = database_url().expect("the live-DB gate passed");
    let mut unlink = PgConnection::connect(&url)
        .await
        .expect("connect the unlink");
    sqlx::query("BEGIN")
        .execute(&mut unlink)
        .await
        .expect("begin the unlink");
    sqlx::query("SELECT account_id FROM account WHERE account_id = $1 FOR UPDATE")
        .bind(ACCOUNT_LOCKED as i32)
        .execute(&mut unlink)
        .await
        .expect("lock the account");
    sqlx::query("DELETE FROM account_identity WHERE identity_id = $1")
        .bind(identity_id)
        .execute(&mut unlink)
        .await
        .expect("unlink the identity");

    let task = tokio::spawn({
        let pool = pool.clone();
        async move { issue_bound(&pool, ACCOUNT_LOCKED, identity_id, base()).await }
    });
    wait_for_a_waiting_lock(&pool).await;
    sqlx::query("COMMIT")
        .execute(&mut unlink)
        .await
        .expect("commit the unlink");

    let outcome = task.await.expect("join").expect("issue_bound");
    assert!(
        matches!(outcome, Issued::Unbound),
        "an issue behind an unlink must see it and answer Unbound"
    );
    assert_eq!(token_rows(&pool, ACCOUNT_LOCKED).await, 0);
    cleanup(&pool, ACCOUNT_LOCKED).await;
}

#[tokio::test]
async fn live_db_issue_bound_refuses_a_disabled_account() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_DISABLED, false).await;
    let identity_id = link_identity(&pool, ACCOUNT_DISABLED, "ct73-disabled").await;

    let outcome = issue_bound(&pool, ACCOUNT_DISABLED, identity_id, base())
        .await
        .expect("issue_bound");
    assert!(
        matches!(outcome, Issued::Disabled),
        "a disabled account must not earn a family"
    );
    assert_eq!(token_rows(&pool, ACCOUNT_DISABLED).await, 0);
    cleanup(&pool, ACCOUNT_DISABLED).await;
}

#[tokio::test]
async fn live_db_issue_bound_issues_for_a_linked_identity() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACCOUNT_LINKED, true).await;
    let identity_id = link_identity(&pool, ACCOUNT_LINKED, "ct73-linked").await;

    let outcome = issue_bound(&pool, ACCOUNT_LINKED, identity_id, base())
        .await
        .expect("issue_bound");
    assert!(
        matches!(outcome, Issued::Pair(_, _)),
        "a linked identity on an enabled account earns a family"
    );
    assert_eq!(token_rows(&pool, ACCOUNT_LINKED).await, 1);
    cleanup(&pool, ACCOUNT_LINKED).await;
}
