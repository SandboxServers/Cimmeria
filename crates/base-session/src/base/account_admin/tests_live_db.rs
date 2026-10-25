//! Live-DB checks for the GM `.account` handler (CT-45): the identity
//! listing, the unlink and its revocation, the last-method and other-account
//! refusals, the link code and its password warning, the staff refusal and
//! the unknown-account line. Sentinel account ids `2_000_701..=2_000_712`;
//! cleanup deletes by exact id, before and after each test.

use std::time::Instant;

use chrono::Utc;
use cimmeria_identity::link_code::LinkCodeStore;
use cimmeria_identity::provider::{ProviderId, VerifiedIdentity};
use cimmeria_identity::repo::{identity, refresh};
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{run, AdminAction, AdminCaller};

const ID_LIST: u32 = 2_000_701;
const ID_UNLINK: u32 = 2_000_702;
const ID_LAST: u32 = 2_000_703;
const ID_PW: u32 = 2_000_704;
const ID_NO_PW: u32 = 2_000_705;
const ID_OTHER_A: u32 = 2_000_706;
const ID_OTHER_B: u32 = 2_000_707;
const ID_STAFF: u32 = 2_000_708;
const ID_TEST: u32 = 2_000_709;
const ID_CODE: u32 = 2_000_710;
const ID_UNKNOWN: u32 = 2_000_711;

/// The GM who types the commands in these checks.
const GM: AdminCaller = AdminCaller {
    entity_id: 9_001,
    account_id: Some(1),
};

/// The alphabet a link code is drawn from (C6).
const CODE_ALPHABET: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Delete the sentinel account `account_id`. Its identity, refresh and event
/// rows cascade with it. A failed delete is ignored so it never hides an
/// assertion.
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

/// A verified identity under the `devmock` provider, with the given subject.
fn verified(subject: &str) -> VerifiedIdentity {
    VerifiedIdentity {
        provider: ProviderId::new("devmock").expect("provider id"),
        issuer: "https://issuer.ct45.example.org".to_owned(),
        subject: subject.to_owned(),
        display_name: "CT45 Player".to_owned(),
        email: None,
        email_verified: false,
        provider_account_created_ms: None,
    }
}

/// Link one identity to `account_id` and return its row id.
async fn link(pool: &PgPool, account_id: u32, subject: &str) -> i64 {
    identity::insert(pool, account_id, &verified(subject), false)
        .await
        .expect("link identity")
        .identity_id
}

/// Count the rows of `sql`, which takes the account id as `$1`.
async fn count(pool: &PgPool, sql: &'static str, account_id: u32) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(account_id as i32)
        .fetch_one(pool)
        .await
        .expect("count rows")
}

#[tokio::test]
async fn live_db_identities_lists_without_subjects_or_emails() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_LIST, "ct45_list_701", Some("legacy")).await;
    link(&pool, ID_LIST, "ct45-subject-701").await;
    let lines = run(
        &GM,
        "ct45_list_701",
        AdminAction::Identities,
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    cleanup(&pool, ID_LIST).await;

    assert!(
        lines[0].contains("1 sign-in methods; password set"),
        "{lines:?}"
    );
    assert!(lines[1].contains("devmock") && lines[1].contains("CT45 Player"));
    assert!(lines
        .iter()
        .all(|l| !l.contains("ct45-") && !l.contains("example.org")));
}

#[tokio::test]
async fn live_db_unlink_removes_and_revokes() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_UNLINK, "ct45_unlink_702", None).await;
    let first = link(&pool, ID_UNLINK, "ct45-subject-702a").await;
    link(&pool, ID_UNLINK, "ct45-subject-702b").await;
    for _ in 0..2 {
        refresh::issue(&pool, ID_UNLINK, None, Utc::now())
            .await
            .expect("issue refresh token");
    }
    let before = count(
        &pool,
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        ID_UNLINK,
    )
    .await;
    let lines = run(
        &GM,
        "ct45_unlink_702",
        AdminAction::Unlink { identity_id: first },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let methods = identity::count_methods(&pool, ID_UNLINK)
        .await
        .expect("count");
    let events = count(
        &pool,
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'unlink'",
        ID_UNLINK,
    )
    .await;
    let live = count(
        &pool,
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
        ID_UNLINK,
    )
    .await;
    cleanup(&pool, ID_UNLINK).await;

    assert_eq!(before, 2, "two live refresh rows before the unlink");
    assert!(
        lines[0].starts_with("account: removed identity #"),
        "{lines:?}"
    );
    assert_eq!(methods, 1, "one identity row is gone");
    assert_eq!(events, 1, "one unlink event row");
    assert_eq!(live, 0, "every refresh row of the account is revoked");
}

#[tokio::test]
async fn live_db_unlink_refuses_the_last_method() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_LAST, "ct45_last_703", None).await;
    let only = link(&pool, ID_LAST, "ct45-subject-703").await;
    let lines = run(
        &GM,
        "ct45_last_703",
        AdminAction::Unlink { identity_id: only },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let methods = identity::count_methods(&pool, ID_LAST)
        .await
        .expect("count");
    cleanup(&pool, ID_LAST).await;

    assert_eq!(
        lines,
        vec!["account: refusing to remove the last sign-in method of ct45_last_703".to_owned()]
    );
    assert_eq!(methods, 1, "the last row is kept");
}

#[tokio::test]
async fn live_db_linkcode_warns_when_the_account_has_a_password() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_PW, "ct45_pw_704", Some("legacy")).await;
    insert_account(&pool, ID_NO_PW, "ct45_nopw_705", None).await;
    let codes = LinkCodeStore::default();
    let with = run(
        &GM,
        "ct45_pw_704",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    let without = run(
        &GM,
        "ct45_nopw_705",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    cleanup(&pool, ID_PW).await;
    cleanup(&pool, ID_NO_PW).await;

    assert!(
        with.iter()
            .any(|l| l.contains("removes it for good (D-CT11)")),
        "{with:?}"
    );
    assert!(
        without.iter().all(|l| !l.contains("removes it for good")),
        "{without:?}"
    );
}

#[tokio::test]
async fn live_db_unlink_other_accounts_identity_refused() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_OTHER_A, "ct45_othera_706", None).await;
    insert_account(&pool, ID_OTHER_B, "ct45_otherb_707", None).await;
    link(&pool, ID_OTHER_A, "ct45-subject-706a").await;
    link(&pool, ID_OTHER_A, "ct45-subject-706b").await;
    let b_identity = link(&pool, ID_OTHER_B, "ct45-subject-707").await;
    let lines = run(
        &GM,
        "ct45_othera_706",
        AdminAction::Unlink {
            identity_id: b_identity,
        },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let b_methods = identity::count_methods(&pool, ID_OTHER_B)
        .await
        .expect("count");
    cleanup(&pool, ID_OTHER_A).await;
    cleanup(&pool, ID_OTHER_B).await;

    assert_eq!(
        lines,
        vec![format!(
            "account: identity #{b_identity} is not linked to ct45_othera_706"
        )]
    );
    assert_eq!(b_methods, 1, "the other account keeps its identity");
}

#[tokio::test]
async fn live_db_linkcode_refuses_staff_and_test_accounts() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_STAFF, "ct45_staff_708", None).await;
    insert_account(&pool, ID_TEST, "ct45_test_709", None).await;
    sqlx::query("UPDATE account SET accesslevel = 1 WHERE account_id = $1")
        .bind(ID_STAFF as i32)
        .execute(&pool)
        .await
        .expect("set staff");
    sqlx::query("UPDATE account SET test_account = true WHERE account_id = $1")
        .bind(ID_TEST as i32)
        .execute(&pool)
        .await
        .expect("set test");
    let codes = LinkCodeStore::default();
    let staff = run(
        &GM,
        "ct45_staff_708",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    let test = run(
        &GM,
        "ct45_test_709",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    cleanup(&pool, ID_STAFF).await;
    cleanup(&pool, ID_TEST).await;

    assert!(staff[0].contains("(not_linkable)"), "{staff:?}");
    assert!(test[0].contains("(not_linkable)"), "{test:?}");
    assert!(codes.redeem("ABCDEFGHJK", Instant::now()).is_none());
}

#[tokio::test]
async fn live_db_linkcode_issues_a_redeemable_code() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_CODE, "ct45_code_710", None).await;
    let codes = LinkCodeStore::default();
    let lines = run(
        &GM,
        "ct45_code_710",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    let events = count(
        &pool,
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'link_code_issued'",
        ID_CODE,
    )
    .await;
    cleanup(&pool, ID_CODE).await;

    let code = lines[0]
        .strip_prefix("account: link code for ct45_code_710: ")
        .expect("the code line");
    assert_eq!(code.len(), 10);
    assert!(code.bytes().all(|b| CODE_ALPHABET.as_bytes().contains(&b)));
    assert_eq!(codes.redeem(code, Instant::now()), Some(ID_CODE));
    assert_eq!(events, 1, "one link_code_issued event row");
}

#[tokio::test]
async fn live_db_unknown_account_line() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ID_UNKNOWN).await;
    let lines = run(
        &GM,
        "Player_9999",
        AdminAction::Identities,
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    assert_eq!(
        lines,
        vec!["account: no account named Player_9999".to_owned()]
    );
}

#[tokio::test]
async fn no_pool_answers_unavailable() {
    let lines = run(
        &GM,
        "Player_9999",
        AdminAction::Identities,
        None,
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    assert_eq!(
        lines,
        vec!["account: the database is not available".to_owned()]
    );
}
