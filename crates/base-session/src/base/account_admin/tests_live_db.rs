//! Live-DB checks for the GM `.account` handler (CT-45): the identity
//! listing, the unlink and its revocation, the last-method and other-account
//! refusals, the link code and its password warning, the staff refusal and
//! the unknown-account line, and the live-token revocation on unlink. Sentinel
//! account ids `2_000_701..=2_000_714`; cleanup deletes by exact id, before and
//! after each test.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use cimmeria_identity::link_code::LinkCodeStore;
use cimmeria_identity::provider::{ProviderId, VerifiedIdentity};
use cimmeria_identity::repo::{identity, refresh};
use cimmeria_identity::revoke::{self, LaunchTicketStore};
use cimmeria_identity::stores::Stores;
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{date, run, sanitize, AdminAction, AdminCaller};

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
const ID_RACE: u32 = 2_000_712;
const ID_LIVE: u32 = 2_000_713;
const ID_NOLINK: u32 = 2_000_714;

/// The GM who types the commands in these checks.
const GM: AdminCaller = AdminCaller {
    entity_id: 9_001,
    entity_name: String::new(),
    account_id: Some(1),
    gm_account_name: String::new(),
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
    // Linking clears a legacy password (C15 a), so the account starts without one.
    insert_account(&pool, ID_LIST, "ct45_list_701", None).await;
    let mut with_email = verified("ct45-subject-701");
    with_email.email = Some("p701@example.org".to_owned());
    with_email.email_verified = true;
    identity::insert(&pool, ID_LIST, &with_email, true)
        .await
        .expect("link identity");
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
        lines[0].contains("1 sign-in methods; password none"),
        "{lines:?}"
    );
    assert!(lines[1].contains("devmock") && lines[1].contains("CT45 Player"));
    assert!(lines
        .iter()
        .all(|l| !l.contains("ct45-") && !l.contains("example.org") && !l.contains("p701")));
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

/// A password-only account has no identity row, so an unlink names nothing
/// and is not the last method.
#[tokio::test]
async fn live_db_unlink_on_password_only_account_is_not_linked() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_NOLINK, "ct45_nolink_714", Some("legacy")).await;
    let lines = run(
        &GM,
        "ct45_nolink_714",
        AdminAction::Unlink { identity_id: 1 },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    cleanup(&pool, ID_NOLINK).await;

    assert!(lines[0].contains("is not linked to"), "{lines:?}");
    assert!(!lines[0].contains("last sign-in method"), "{lines:?}");
}

/// An unlink ends the removed account's live access token and launch ticket,
/// which sit in the identity process, not in the refresh table (N1).
#[tokio::test]
async fn live_db_unlink_ends_live_access_tokens_and_tickets() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_LIVE, "ct45_live_713", None).await;
    let first = link(&pool, ID_LIVE, "ct45-subject-713a").await;
    link(&pool, ID_LIVE, "ct45-subject-713b").await;
    let stores = Arc::new(Stores::default());
    let tickets = Arc::new(LaunchTicketStore::default());
    revoke::register(Arc::clone(&stores), Arc::clone(&tickets));
    let now = Instant::now();
    let token = stores.access.mint(ID_LIVE, "ct45-family-713", false, now);
    tickets.mint(ID_LIVE, "player", "ct45-launch-713", now);
    let lines = run(
        &GM,
        "ct45_live_713",
        AdminAction::Unlink { identity_id: first },
        Some(&pool),
        &LinkCodeStore::default(),
        Instant::now(),
    )
    .await;
    let token_live = stores.access.check(&token, now).is_some();
    let tickets_live = tickets.live_count();
    cleanup(&pool, ID_LIVE).await;

    assert!(
        lines[0].ends_with(
            "its launcher sign-ins are revoked; a game session already running is not ended"
        ),
        "{lines:?}"
    );
    assert!(!token_live, "the access token is ended");
    assert_eq!(tickets_live, 0, "the launch ticket is ended");
}

/// Two unlinks of one two-method account run on two worker threads at once.
/// Without the account row lock both read two methods and both delete.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_db_concurrent_unlinks_keep_one_method() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_RACE, "ct45_race_712", None).await;
    let a = link(&pool, ID_RACE, "ct45-subject-712a").await;
    let b = link(&pool, ID_RACE, "ct45-subject-712b").await;
    let (pool_a, pool_b) = (pool.clone(), pool.clone());
    let first = tokio::spawn(async move {
        run(
            &GM,
            "ct45_race_712",
            AdminAction::Unlink { identity_id: a },
            Some(&pool_a),
            &LinkCodeStore::default(),
            Instant::now(),
        )
        .await
    });
    let second = tokio::spawn(async move {
        run(
            &GM,
            "ct45_race_712",
            AdminAction::Unlink { identity_id: b },
            Some(&pool_b),
            &LinkCodeStore::default(),
            Instant::now(),
        )
        .await
    });
    let (first, second) = (
        first.await.expect("first unlink task"),
        second.await.expect("second unlink task"),
    );
    let methods = identity::count_methods(&pool, ID_RACE)
        .await
        .expect("count");
    cleanup(&pool, ID_RACE).await;

    assert_eq!(methods, 1, "one identity survives: {first:?} {second:?}");
    let removed = [&first, &second]
        .iter()
        .filter(|l| l[0].starts_with("account: removed identity #"))
        .count();
    assert_eq!(removed, 1, "exactly one unlink wins: {first:?} {second:?}");
}

/// An unlink waits for the account row lock. The test holds `FOR UPDATE` on
/// the account row and drops the second method under that lock. The unlink
/// must wait, then count one method and refuse. Without the lock it counts
/// two before the drop commits and removes the first method too. This is
/// deterministic, unlike the race above, which only sometimes loses.
#[tokio::test]
async fn live_db_unlink_waits_for_the_account_row_lock() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_RACE, "ct45_race_712", None).await;
    let a = link(&pool, ID_RACE, "ct45-subject-712a").await;
    let b = link(&pool, ID_RACE, "ct45-subject-712b").await;
    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("SELECT account_id FROM account WHERE account_id = $1 FOR UPDATE")
        .bind(ID_RACE as i32)
        .fetch_one(&mut *holder)
        .await
        .expect("lock the account row");
    sqlx::query("DELETE FROM account_identity WHERE identity_id = $1")
        .bind(b)
        .execute(&mut *holder)
        .await
        .expect("drop the second method under the lock");
    let worker_pool = pool.clone();
    let task = tokio::spawn(async move {
        run(
            &GM,
            "ct45_race_712",
            AdminAction::Unlink { identity_id: a },
            Some(&worker_pool),
            &LinkCodeStore::default(),
            Instant::now(),
        )
        .await
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let waited = !task.is_finished();
    holder.commit().await.expect("release the lock");
    let lines = task.await.expect("unlink task");
    let methods = identity::count_methods(&pool, ID_RACE)
        .await
        .expect("count");
    cleanup(&pool, ID_RACE).await;

    assert!(waited, "the unlink must wait for the account row lock");
    assert!(
        lines[0].starts_with("account: refusing to remove the last sign-in method"),
        "{lines:?}"
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
    let issued = count(
        &pool,
        "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'link_code_issued'",
        ID_STAFF,
    )
    .await
        + count(
            &pool,
            "SELECT COUNT(*) FROM account_identity_event WHERE account_id = $1 AND kind = 'link_code_issued'",
            ID_TEST,
        )
        .await;
    cleanup(&pool, ID_STAFF).await;
    cleanup(&pool, ID_TEST).await;

    assert!(staff[0].contains("(not_linkable)"), "{staff:?}");
    assert!(test[0].contains("(not_linkable)"), "{test:?}");
    assert_eq!(issued, 0, "no code was issued or recorded");
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

#[tokio::test]
async fn live_db_closed_pool_answers_db_error() {
    let pool = require_db_or_skip!();
    pool.close().await;
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
        vec!["account: the identity tables could not be read or updated; check the result with .account identities <name> and see the server log".to_owned()]
    );
}

#[test]
fn sanitize_keeps_printable_ascii_drops_quotes_and_caps_length() {
    assert_eq!(sanitize("a\"b\u{7}c\u{e9}d"), "abcd");
    assert_eq!(sanitize(&"x".repeat(40)).len(), 32);
}

#[test]
fn date_formats_utc_days_and_survives_out_of_range() {
    assert_eq!(date(0), "1970-01-01");
    assert_eq!(date(1_700_000_000_000), "2023-11-14");
    assert_eq!(date(i64::MAX), "unknown");
}
