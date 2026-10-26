//! Live-DB checks for the stored email authority (C01, D-CT23 = a): a sign-in
//! refreshes the stored email, and only an identity whose provider is trusted
//! in the current config is an auto-link candidate. Sentinel account ids
//! `0x7000_EBC0..=0x7000_EBC3`; cleanup deletes by exact id, before and after
//! each test.

use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{email_candidates, insert, touch};
use crate::provider::{ProviderId, VerifiedIdentity};

const ID_STALE: u32 = 0x7000_EBC0;
const ID_CLEARED: u32 = 0x7000_EBC1;
const ID_UNTRUSTED: u32 = 0x7000_EBC2;
const ID_TRUSTED: u32 = 0x7000_EBC3;

/// Delete the sentinel account `account_id`. Its identity and event rows
/// cascade with it. A failed delete is ignored so it never hides an assertion.
async fn cleanup(pool: &PgPool, account_id: u32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
}

/// Insert a sentinel account with the given name and no password.
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

/// A verified identity under `provider`, with the given subject and email.
fn verified(
    provider: &str,
    subject: &str,
    email: Option<&str>,
    email_verified: bool,
) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new(provider).expect("provider id"),
        issuer: "https://issuer.ct94.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT94 Player".to_owned(),
        email: email.map(str::to_owned),
        email_verified,
        provider_account_created_ms: None,
    }
}

#[tokio::test]
async fn live_db_stale_email_does_not_auto_link() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_STALE, "ct94_stale_0").await;
    let old = "ct94-old@example.org";
    let new = "ct94-new@example.org";
    let linked = insert(
        &pool,
        ID_STALE,
        &verified("ct94-dev", "ct94-subject-stale", Some(old), true),
        true,
    )
    .await;
    let identity_id = linked.as_ref().map_or(0, |l| l.identity_id);
    let refreshed = touch(
        &pool,
        identity_id,
        1_700_000_000_000,
        &verified("ct94-dev", "ct94-subject-stale", Some(new), true),
        true,
    )
    .await;
    let old_candidates = email_candidates(&pool, old, &["ct94-dev"]).await;
    let new_candidates = email_candidates(&pool, new, &["ct94-dev"]).await;
    cleanup(&pool, ID_STALE).await;
    linked.expect("link");
    refreshed.expect("touch");
    assert!(
        old_candidates.expect("candidates").is_empty(),
        "the stale email still made a candidate"
    );
    let new_ids: Vec<u32> = new_candidates
        .expect("candidates")
        .iter()
        .map(|f| f.account_id)
        .collect();
    assert_eq!(
        new_ids,
        vec![ID_STALE],
        "the refreshed email is not a candidate"
    );
}

#[tokio::test]
async fn live_db_signin_without_a_verified_email_clears_the_stored_one() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_CLEARED, "ct94_cleared_1").await;
    let email = "ct94-cleared@example.org";
    let linked = insert(
        &pool,
        ID_CLEARED,
        &verified("ct94-dev", "ct94-subject-cleared", Some(email), true),
        true,
    )
    .await;
    let identity_id = linked.as_ref().map_or(0, |l| l.identity_id);
    let touched = touch(
        &pool,
        identity_id,
        1_700_000_000_000,
        &verified("ct94-dev", "ct94-subject-cleared", None, false),
        true,
    )
    .await;
    let stored: Result<(bool, bool), sqlx::Error> = sqlx::query_as(
        "SELECT email IS NULL, email_verified_at IS NULL FROM account_identity WHERE identity_id = $1",
    )
    .bind(identity_id)
    .fetch_one(&pool)
    .await;
    let candidates = email_candidates(&pool, email, &["ct94-dev"]).await;
    cleanup(&pool, ID_CLEARED).await;
    linked.expect("link");
    touched.expect("touch");
    assert_eq!(stored.expect("read the identity"), (true, true));
    assert!(candidates.expect("candidates").is_empty());
}

#[tokio::test]
async fn live_db_identity_of_an_untrusted_provider_is_not_a_candidate() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_UNTRUSTED, "ct94_untrusted_2").await;
    let email = "ct94-untrusted@example.org";
    let linked = insert(
        &pool,
        ID_UNTRUSTED,
        &verified("ct94-dev", "ct94-subject-untrusted", Some(email), true),
        true,
    )
    .await;
    let omitted = email_candidates(&pool, email, &["ct94-other"]).await;
    let empty = email_candidates(&pool, email, &[]).await;
    cleanup(&pool, ID_UNTRUSTED).await;
    linked.expect("link");
    assert!(
        omitted.expect("candidates").is_empty(),
        "an untrusted provider made a candidate"
    );
    assert!(
        empty.expect("candidates").is_empty(),
        "an empty trusted list made a candidate"
    );
}

#[tokio::test]
async fn live_db_trusted_provider_identity_is_still_a_candidate() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_TRUSTED, "ct94_trusted_3").await;
    let email = "ct94-trusted@example.org";
    let linked = insert(
        &pool,
        ID_TRUSTED,
        &verified("ct94-dev", "ct94-subject-trusted", Some(email), true),
        true,
    )
    .await;
    let candidates = email_candidates(&pool, email, &["ct94-other", "ct94-dev"]).await;
    cleanup(&pool, ID_TRUSTED).await;
    linked.expect("link");
    let ids: Vec<u32> = candidates
        .expect("candidates")
        .iter()
        .map(|f| f.account_id)
        .collect();
    assert_eq!(ids, vec![ID_TRUSTED]);
}
