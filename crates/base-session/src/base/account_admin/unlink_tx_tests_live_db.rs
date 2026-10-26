//! Live-DB checks for the GM unlink's revoke inside its transaction (C16.6): a
//! revoke that fails rolls the delete back and answers the C16.6 text, and a
//! successful unlink revokes every family of the account. Sentinel account ids
//! `0x7000_E480..=0x7000_E4FF`; cleanup deletes only the exact ids it inserted.

use std::time::Instant;

use chrono::Utc;
use cimmeria_identity::link_code::LinkCodeStore;
use cimmeria_identity::repo::{identity, refresh};
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{run, AdminAction, AdminCaller};

/// The account whose unlink the trigger makes fail.
const ACC_REVOKE_FAILS: u32 = 0x7000_E480;
/// The account whose every family a successful unlink revokes.
const ACC_REVOKE_ALL: u32 = 0x7000_E481;
/// The trigger on `ACC_REVOKE_FAILS` only (the id is written out, sqlx wants a
/// literal statement).
const CREATE_TRIGGER: &str = "CREATE TRIGGER ct76_fail_revoke BEFORE UPDATE ON auth_refresh_token \
     FOR EACH ROW WHEN (OLD.account_id = 1879106688) EXECUTE FUNCTION ct76_fail_revoke()";
const _: () = assert!(ACC_REVOKE_FAILS == 1_879_106_688);

/// The GM who types the commands in these checks.
const GM: AdminCaller = AdminCaller {
    entity_id: 9_001,
    entity_name: String::new(),
    account_id: Some(1),
    gm_account_name: String::new(),
};

/// Deletes the sentinel account `account_id` by exact id. Its identity, refresh
/// and event rows cascade with it. A failed delete is ignored so it never hides
/// an assertion.
async fn cleanup(pool: &PgPool, account_id: u32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
}

/// Inserts a passwordless sentinel account named `name`, enabled.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, NULL, 1, 0, true)",
    )
    .bind(account_id as i32)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// Links one `devmock` identity with the given subject to `account_id`; returns
/// its row id.
async fn link(pool: &PgPool, account_id: u32, subject: &str) -> i64 {
    let verified = cimmeria_identity::provider::VerifiedIdentity {
        provider: cimmeria_identity::provider::ProviderId::new("devmock").expect("provider id"),
        issuer: "https://issuer.ct76.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT76 Player".to_owned(),
        email: None,
        email_verified: false,
        provider_account_created_ms: None,
    };
    identity::insert(pool, account_id, &verified, false)
        .await
        .expect("link identity")
        .identity_id
}

/// Counts the rows of `sql`, which takes the account id as `$1`.
async fn count(pool: &PgPool, sql: &'static str, account_id: u32) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(account_id as i32)
        .fetch_one(pool)
        .await
        .expect("count rows")
}

/// Installs the test-only trigger that makes every refresh row of
/// `ACC_REVOKE_FAILS` fail to update, so its revoke errors. Scoped to that id.
async fn install_revoke_failure(pool: &PgPool) {
    let _ = sqlx::query("DROP TRIGGER IF EXISTS ct76_fail_revoke ON auth_refresh_token")
        .execute(pool)
        .await;
    sqlx::query(
        "CREATE OR REPLACE FUNCTION ct76_fail_revoke() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'ct76'; END $$",
    )
    .execute(pool)
    .await
    .expect("create the revoke trigger function");
    sqlx::query(CREATE_TRIGGER)
        .execute(pool)
        .await
        .expect("create the revoke trigger");
}

/// Drops the revoke trigger, so no other check sees it.
async fn drop_revoke_failure(pool: &PgPool) {
    let _ = sqlx::query("DROP TRIGGER IF EXISTS ct76_fail_revoke ON auth_refresh_token")
        .execute(pool)
        .await;
}

#[tokio::test]
async fn live_db_gm_unlink_rolls_back_when_the_revoke_fails() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACC_REVOKE_FAILS, "ct76_fail_480").await;
    let first = link(&pool, ACC_REVOKE_FAILS, "ct76-subject-480a").await;
    link(&pool, ACC_REVOKE_FAILS, "ct76-subject-480b").await;
    refresh::issue(&pool, ACC_REVOKE_FAILS, None, Utc::now())
        .await
        .expect("issue refresh token");
    install_revoke_failure(&pool).await;
    let lines = run(
        &GM,
        "ct76_fail_480",
        AdminAction::Unlink { identity_id: first },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let methods = count(
        &pool,
        "SELECT COUNT(*) FROM account_identity WHERE account_id = $1",
        ACC_REVOKE_FAILS,
    )
    .await;
    let live = count(
        &pool,
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        ACC_REVOKE_FAILS,
    )
    .await;
    drop_revoke_failure(&pool).await;
    cleanup(&pool, ACC_REVOKE_FAILS).await;

    assert!(
        lines[0].starts_with("account: could not remove identity #")
            && lines[0].ends_with("nothing changed, see the server log"),
        "{lines:?}"
    );
    assert_eq!(methods, 2, "the delete rolled back: both identities remain");
    assert_eq!(live, 1, "the refresh row was not revoked");
}

#[tokio::test]
async fn live_db_gm_unlink_revokes_every_family() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ACC_REVOKE_ALL, "ct76_all_481").await;
    let first = link(&pool, ACC_REVOKE_ALL, "ct76-subject-481a").await;
    link(&pool, ACC_REVOKE_ALL, "ct76-subject-481b").await;
    for _ in 0..2 {
        refresh::issue(&pool, ACC_REVOKE_ALL, None, Utc::now())
            .await
            .expect("issue refresh token");
    }
    let lines = run(
        &GM,
        "ct76_all_481",
        AdminAction::Unlink { identity_id: first },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let methods = count(
        &pool,
        "SELECT COUNT(*) FROM account_identity WHERE account_id = $1",
        ACC_REVOKE_ALL,
    )
    .await;
    let live = count(
        &pool,
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        ACC_REVOKE_ALL,
    )
    .await;
    cleanup(&pool, ACC_REVOKE_ALL).await;

    assert!(
        lines[0].starts_with("account: removed identity #"),
        "{lines:?}"
    );
    assert_eq!(methods, 1, "one identity row is gone");
    assert_eq!(live, 0, "every refresh row of the account is revoked");
}
