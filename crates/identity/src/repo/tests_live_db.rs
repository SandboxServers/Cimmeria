//! Live-DB checks for the identity repositories (CT-15): the find-and-link
//! path, the verified-email candidates, the account name pick, the method
//! count, the password clear on link, the list shape and the event kinds.
//! Sentinel account ids `2_000_201..=2_000_213`; cleanup deletes by exact id,
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
const ID_ROLLBACK: u32 = 2_000_208;
const ID_ROLLBACK_OTHER: u32 = 2_000_209;
const ID_EMAIL_OFF: u32 = 2_000_210;
const ID_LOCK: u32 = 2_000_211;
const ID_MISC: u32 = 2_000_212;
const ID_MISC_OTHER: u32 = 2_000_213;

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
    let before = identity::email_candidates(&pool, email, &["ct15-dev"]).await;
    let proven = verified("ct15-subject-202b", Some(email), true);
    let verified_link = identity::insert(&pool, ID_EMAIL, &proven, true).await;
    let after = identity::email_candidates(&pool, email, &["ct15-dev"]).await;
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
async fn live_db_create_account_waits_for_the_advisory_lock() {
    let pool = require_db_or_skip!();
    let name = "ct15_lock_a".to_owned();
    let _ = sqlx::query("DELETE FROM account WHERE lower(account_name) = $1")
        .bind(&name)
        .execute(&pool)
        .await;
    let mut holder = pool.begin().await.expect("begin the lock holder");
    sqlx::query("SELECT pg_advisory_xact_lock(2000201)")
        .execute(&mut *holder)
        .await
        .expect("take the create lock");
    let creator_pool = pool.clone();
    let candidates = [name.clone()];
    let task = tokio::spawn(async move { account::create(&creator_pool, &candidates).await });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let ran_early = task.is_finished();
    holder.commit().await.expect("release the lock");
    let (id, got) = task.await.expect("join").expect("create after the lock");
    cleanup(&pool, id).await;
    assert!(
        !ran_early,
        "create ran while another transaction held the lock"
    );
    assert_eq!(got, name);
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
    sqlx::query("UPDATE account SET password_hash_v2 = 'ct15-hash-205', password_algo = 2 WHERE account_id = $1")
        .bind(ID_CLEAR as i32)
        .execute(&pool)
        .await
        .expect("seed an algo-2 hash");
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
    let links: Result<(i64,), sqlx::Error> = sqlx::query_as(
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'link'",
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
    assert_eq!(links.expect("count link events").0, 2);
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

#[tokio::test]
async fn live_db_failed_link_keeps_the_password() {
    let pool = require_db_or_skip!();
    insert_account(
        &pool,
        ID_ROLLBACK,
        "ct15_rollback_208",
        Some("legacy-hash-208"),
    )
    .await;
    insert_account(&pool, ID_ROLLBACK_OTHER, "ct15_rollback_209", None).await;
    let id = verified("ct15-subject-208", None, false);
    let held = identity::insert(&pool, ID_ROLLBACK_OTHER, &id, false).await;
    let refused = identity::insert(&pool, ID_ROLLBACK, &id, false).await;
    let state: Result<(bool, i64), sqlx::Error> = sqlx::query_as(
        "SELECT password IS NOT NULL, \
         (SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1) \
         FROM account WHERE account_id = $1",
    )
    .bind(ID_ROLLBACK as i32)
    .fetch_one(&pool)
    .await;
    cleanup(&pool, ID_ROLLBACK).await;
    cleanup(&pool, ID_ROLLBACK_OTHER).await;
    held.expect("the first link");
    assert_eq!(refused, Err(super::RepoError::Conflict));
    let (has_password, events) = state.expect("read the account");
    assert!(has_password, "a refused link cleared the password");
    assert_eq!(events, 0, "a refused link left an event behind");
}

#[tokio::test]
async fn live_db_email_not_stored_when_store_email_is_false() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_EMAIL_OFF, "ct15_emailoff_210", None).await;
    let email = "ct15-210@example.org";
    let linked = identity::insert(
        &pool,
        ID_EMAIL_OFF,
        &verified("ct15-subject-210", Some(email), true),
        false,
    )
    .await;
    let stored: Result<(bool, bool), sqlx::Error> = sqlx::query_as(
        "SELECT email IS NULL, email_verified_at IS NULL FROM account_identity WHERE account_id = $1",
    )
    .bind(ID_EMAIL_OFF as i32)
    .fetch_one(&pool)
    .await;
    let candidates = identity::email_candidates(&pool, email, &["ct15-dev"]).await;
    cleanup(&pool, ID_EMAIL_OFF).await;
    linked.expect("link");
    assert_eq!(stored.expect("read the identity"), (true, true));
    assert!(candidates.expect("candidates").is_empty());
}

#[tokio::test]
async fn live_db_insert_waits_for_the_account_row() {
    let pool = require_db_or_skip!();
    // No password, so the guard trigger takes no lock here: only the insert's
    // own `FOR UPDATE` on the account row can make it wait.
    insert_account(&pool, ID_LOCK, "ct15_lock_211", None).await;
    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("SELECT 1 FROM account WHERE account_id = $1 FOR NO KEY UPDATE")
        .bind(ID_LOCK as i32)
        .execute(&mut *holder)
        .await
        .expect("hold the row");
    let task = tokio::spawn({
        let pool = pool.clone();
        async move {
            identity::insert(
                &pool,
                ID_LOCK,
                &verified("ct15-subject-211", None, false),
                false,
            )
            .await
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let blocked = !task.is_finished();
    holder.commit().await.expect("release");
    let linked = task.await.expect("join");
    cleanup(&pool, ID_LOCK).await;
    assert!(blocked, "insert did not wait for the account row lock");
    linked.expect("link after release");
}

#[tokio::test]
async fn live_db_touch_delete_facts_and_has_password() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_MISC, "ct15_misc_212", Some("legacy-hash-212")).await;
    insert_account(&pool, ID_MISC_OTHER, "CT15_Other_213", None).await;
    let has_password = account::has_password(&pool, ID_MISC).await;
    let has_password_other = account::has_password(&pool, ID_MISC_OTHER).await;
    let facts = account::facts(&pool, ID_MISC).await;
    let missing = account::facts(&pool, 2_000_219).await;
    let by_name = account::match_name(&pool, "ct15_other_213").await;
    let linked = identity::insert(
        &pool,
        ID_MISC,
        &verified("ct15-subject-212", None, false),
        false,
    )
    .await;
    let identity_id = linked.as_ref().map_or(0, |l| l.identity_id);
    let touched = identity::touch(
        &pool,
        identity_id,
        1_700_000_000_000,
        &verified("ct15-subject-212", None, false),
        false,
    )
    .await;
    let stamp: Result<(i64,), sqlx::Error> = sqlx::query_as(
        "SELECT (EXTRACT(EPOCH FROM last_used_at) * 1000)::BIGINT \
         FROM account_identity WHERE identity_id = $1",
    )
    .bind(identity_id)
    .fetch_one(&pool)
    .await;
    let listed = identity::list_for_account(&pool, ID_MISC).await;
    let wrong_delete = identity::delete(&pool, ID_MISC_OTHER, identity_id).await;
    let methods_kept = identity::count_methods(&pool, ID_MISC).await;
    let deleted = identity::delete(&pool, ID_MISC, identity_id).await;
    let repeat = identity::delete(&pool, ID_MISC, identity_id).await;
    let methods_after = identity::count_methods(&pool, ID_MISC).await;
    cleanup(&pool, ID_MISC).await;
    cleanup(&pool, ID_MISC_OTHER).await;

    assert_eq!(has_password, Ok(true));
    assert_eq!(has_password_other, Ok(false));
    let facts = facts.expect("facts").expect("the account exists");
    assert_eq!(facts.account_id, ID_MISC);
    assert!(facts.enabled && facts.accesslevel == 0 && !facts.test_account);
    assert!(missing.expect("facts of a missing id").is_none());
    assert!(
        matches!(by_name, Ok(account::NameMatch::One(ref f)) if f.account_id == ID_MISC_OTHER),
        "the single folded name is found, got {by_name:?}"
    );
    let linked = linked.expect("link");
    touched.expect("touch");
    assert_eq!(stamp.expect("read last_used_at").0, 1_700_000_000_000);
    let rows = listed.expect("list");
    assert_eq!(rows[0].last_used_at, 1_700_000_000_000);
    assert_eq!(linked.identity_id, identity_id);
    assert_eq!(wrong_delete, Ok(false), "a delete crossed accounts");
    assert_eq!(methods_kept, Ok(1));
    assert_eq!(deleted, Ok(true));
    assert_eq!(repeat, Ok(false));
    assert_eq!(methods_after, Ok(0));
}
