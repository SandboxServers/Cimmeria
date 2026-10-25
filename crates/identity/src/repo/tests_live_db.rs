//! Live-DB checks for the identity repositories (CT-15): the find-and-link
//! path, the verified-email candidates, the account name pick, the method
//! count, the password clear on link, the list shape and the event kinds.
//! Sentinel account ids `2_000_201..=2_000_207`; cleanup deletes by exact id,
//! before and after each test.

use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::account;
use super::event::{self, EventKind};
use super::identity::{self, IdentityRow, LIST_SQL};
use crate::provider::{ProviderId, VerifiedIdentity};

const ID_BIND: u32 = 2_000_201;
const ID_EMAIL: u32 = 2_000_202;
const ID_NAMES: u32 = 2_000_203;
const ID_COUNT: u32 = 2_000_204;
const ID_CLEAR: u32 = 2_000_205;
const ID_LIST: u32 = 2_000_206;
const ID_EVENTS: u32 = 2_000_207;

/// Delete the sentinel account `account_id`. Its identity and event rows
/// cascade with it. A failed delete is ignored so it never hides an assertion.
async fn cleanup(pool: &PgPool, account_id: u32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
}

/// Insert a sentinel account with the given name and legacy password, or none.
async fn insert_account(pool: &PgPool, account_id: u32, name: &str, password: Option<&str>) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, 1, 0, true)",
    )
    .bind(account_id as i32)
    .bind(name)
    .bind(password)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

/// A verified identity under the `ct15-dev` provider, with the given subject
/// and email.
fn verified(subject: &str, email: Option<&str>, email_verified: bool) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new("ct15-dev").expect("provider id"),
        issuer: "https://issuer.ct15.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT15 Player".to_owned(),
        email: email.map(str::to_owned),
        email_verified,
        provider_account_created_ms: None,
    }
}

#[tokio::test]
async fn live_db_find_bound_and_insert_conflict() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_BIND, "ct15_bind_201", None).await;
    let id = verified("ct15-subject-201", None, false);
    let first = identity::insert(&pool, ID_BIND, &id, false).await;
    let bound = identity::find_bound(&pool, "ct15-dev", &id.issuer, &id.subject).await;
    let second = identity::insert(&pool, ID_BIND, &id, false).await;
    cleanup(&pool, ID_BIND).await;
    let linked = first.expect("first link");
    let (identity_id, facts) = bound.expect("find_bound").expect("the identity is bound");
    assert_eq!(identity_id, linked.identity_id);
    assert_eq!(facts.account_id, ID_BIND);
    assert_eq!(second, Err(super::RepoError::Conflict));
}

#[tokio::test]
async fn live_db_email_candidates_only_verified() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_EMAIL, "ct15_email_202", None).await;
    let email = "ct15-202@example.org";
    let unverified = verified("ct15-subject-202a", Some(email), false);
    let linked = identity::insert(&pool, ID_EMAIL, &unverified, true).await;
    let before = identity::email_candidates(&pool, email).await;
    let proven = verified("ct15-subject-202b", Some(email), true);
    let verified_link = identity::insert(&pool, ID_EMAIL, &proven, true).await;
    let after = identity::email_candidates(&pool, email).await;
    cleanup(&pool, ID_EMAIL).await;
    linked.expect("unverified link");
    verified_link.expect("verified link");
    let before: Vec<u32> = before
        .expect("candidates")
        .iter()
        .map(|f| f.account_id)
        .collect();
    let after: Vec<u32> = after
        .expect("candidates")
        .iter()
        .map(|f| f.account_id)
        .collect();
    assert!(
        before.is_empty(),
        "an unverified email made a candidate: {before:?}"
    );
    assert_eq!(after, vec![ID_EMAIL]);
}

#[tokio::test]
async fn live_db_create_account_skips_taken_names() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_NAMES, "CT15_Taken_203", None).await;
    let candidates = vec!["ct15_taken_203".to_owned(), "ct15_free_203".to_owned()];
    let created = account::create(&pool, &candidates).await;
    cleanup(&pool, ID_NAMES).await;
    if let Ok((new_id, _)) = &created {
        cleanup(&pool, *new_id).await;
    }
    let (new_id, name) = created.expect("create");
    assert_ne!(new_id, ID_NAMES);
    assert_eq!(name, "ct15_free_203");
}

#[tokio::test]
async fn live_db_count_methods_counts_identities() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_COUNT, "ct15_count_204", Some("legacy-hash-204")).await;
    let before = identity::count_methods(&pool, ID_COUNT).await;
    let first = identity::insert(
        &pool,
        ID_COUNT,
        &verified("ct15-subject-204a", None, false),
        false,
    )
    .await;
    let second = identity::insert(
        &pool,
        ID_COUNT,
        &verified("ct15-subject-204b", None, false),
        false,
    )
    .await;
    let after = identity::count_methods(&pool, ID_COUNT).await;
    cleanup(&pool, ID_COUNT).await;
    first.expect("first link");
    second.expect("second link");
    assert_eq!(before, Ok(0), "a password must not count as a method");
    assert_eq!(after, Ok(2));
}

#[tokio::test]
async fn live_db_insert_clears_the_password() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_CLEAR, "ct15_clear_205", Some("legacy-hash-205")).await;
    let first = identity::insert(
        &pool,
        ID_CLEAR,
        &verified("ct15-subject-205a", None, false),
        false,
    )
    .await;
    let second = identity::insert(
        &pool,
        ID_CLEAR,
        &verified("ct15-subject-205b", None, false),
        false,
    )
    .await;
    let cleared: Result<(bool, bool, bool), sqlx::Error> = sqlx::query_as(
        "SELECT password IS NULL, password_hash_v2 IS NULL, password_algo = 1 \
         FROM account WHERE account_id = $1",
    )
    .bind(ID_CLEAR as i32)
    .fetch_one(&pool)
    .await;
    let events: Result<(i64,), sqlx::Error> = sqlx::query_as(
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'password_cleared'",
    )
    .bind(ID_CLEAR as i32)
    .fetch_one(&pool)
    .await;
    cleanup(&pool, ID_CLEAR).await;
    let first = first.expect("first link");
    let second = second.expect("second link");
    let (no_password, no_hash_v2, algo_default) = cleared.expect("read the account");
    assert!(
        first.password_cleared,
        "the first link did not report the clear"
    );
    assert!(!second.password_cleared, "the second link reported a clear");
    assert!(
        no_password && no_hash_v2,
        "a password column still holds a value"
    );
    assert!(algo_default, "password_algo was not reset to 1");
    assert_eq!(events.expect("count events").0, 1);
}

#[tokio::test]
async fn live_db_list_never_returns_subject_or_email() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_LIST, "ct15_list_206", None).await;
    let linked = identity::insert(
        &pool,
        ID_LIST,
        &verified("ct15-subject-206", Some("ct15-206@example.org"), true),
        true,
    )
    .await;
    let rows = identity::list_for_account(&pool, ID_LIST).await;
    cleanup(&pool, ID_LIST).await;
    let linked = linked.expect("link");
    let rows = rows.expect("list");
    assert_eq!(rows.len(), 1);
    // The full field list, with no `..`: a field added to the row would stop
    // this compiling until the test is read again.
    let IdentityRow {
        identity_id,
        provider,
        display_name,
        created_at: _,
        last_used_at: _,
    } = &rows[0];
    assert_eq!(*identity_id, linked.identity_id);
    assert_eq!(provider.as_str(), "ct15-dev");
    assert_eq!(display_name.as_str(), "CT15 Player");
    assert!(
        !LIST_SQL.contains("subject"),
        "the list query selects the subject"
    );
    assert!(
        !LIST_SQL.contains("email"),
        "the list query selects the email"
    );
}

#[tokio::test]
async fn live_db_events_record_every_kind() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_EVENTS, "ct15_events_207", None).await;
    let kinds = [
        EventKind::Signup,
        EventKind::Link,
        EventKind::AutoLink,
        EventKind::Unlink,
        EventKind::LinkRefused,
        EventKind::RefreshReuse,
        EventKind::PasswordCleared,
        EventKind::LinkCodeIssued,
    ];
    let mut results = Vec::new();
    for kind in kinds {
        results.push(event::record(&pool, ID_EVENTS, None, kind, None).await);
    }
    cleanup(&pool, ID_EVENTS).await;
    for (kind, result) in kinds.iter().zip(results) {
        assert!(result.is_ok(), "{kind:?} was rejected: {result:?}");
    }
}
