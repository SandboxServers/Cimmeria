//! Live-DB checks for the player unlink's revoke inside its transaction (C16.6):
//! the unlink ends the account's other refresh families in the commit that
//! deletes the identity, and an attach that waits on the unlink's row lock is
//! refused once the unlink commits. Sentinel account ids
//! `0x7000_E400..=0x7000_E47F`; cleanup deletes only the exact ids it inserted.

use chrono::Utc;
use serde_json::json;

use super::*;
use crate::provider::{ProviderId, VerifiedIdentity};
use crate::repo::refresh;
use crate::stores::{pkce_challenge, HandoffOutcome, LinkIntent};

/// The account whose second family the unlink revokes.
const ACC_OTHER_FAMILY: u32 = 0x7000_E400;
/// The account an unlink and an attach race for.
const ACC_RACE: u32 = 0x7000_E401;

/// Deletes the sentinel account `account_id` by exact id: its refresh families
/// first, then the account (its identities and events cascade). A failed delete
/// is ignored so it never hides an assertion.
async fn clean_exact(pool: &PgPool, account_id: u32) {
    let _ = sqlx::query("DELETE FROM auth_refresh_token WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(pool)
        .await;
}

/// The number of unrevoked refresh rows of `family_id`.
async fn live_rows_of_family(pool: &PgPool, family_id: &str) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM auth_refresh_token WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .fetch_one(pool)
    .await
    .expect("count the family's live rows");
    count
}

/// The number of unrevoked refresh rows of `account_id`.
async fn live_rows_of_account(pool: &PgPool, account_id: u32) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM auth_refresh_token WHERE account_id = $1 AND revoked_at IS NULL",
    )
    .bind(account_id as i32)
    .fetch_one(pool)
    .await
    .expect("count the account's live rows");
    count
}

/// Polls `pg_stat_activity` for up to 10 s until a backend waits on a lock that
/// `holder_pid` holds. Gives up after that, so the caller can still finish.
async fn a_backend_waits_on(pool: &PgPool, holder_pid: i32) -> bool {
    for _ in 0..100 {
        let waiting: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)))",
        )
        .bind(holder_pid)
        .fetch_one(pool)
        .await
        .expect("read pg_stat_activity");
        if waiting {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// The player unlink ends the account's other refresh family in the commit that
/// deletes the identity: once the route answers, a second connection sees the
/// family revoked, and the caller's own family stays live.
#[tokio::test]
async fn live_db_player_unlink_revokes_other_families_in_its_transaction() {
    let pool = require_db_or_skip!();
    clean_exact(&pool, ACC_OTHER_FAMILY).await;
    insert_account(&pool, ACC_OTHER_FAMILY, true).await;
    let first = insert_identity(&pool, ACC_OTHER_FAMILY, 401).await;
    insert_identity(&pool, ACC_OTHER_FAMILY, 402).await;
    let now = Utc::now();
    let (_, own) = refresh::issue(&pool, ACC_OTHER_FAMILY, None, now)
        .await
        .expect("own family");
    let (_, other) = refresh::issue(&pool, ACC_OTHER_FAMILY, None, now)
        .await
        .expect("other family");
    let env = harness(&pool);
    let caller = env
        .stores
        .access
        .mint(ACC_OTHER_FAMILY, &own, true, Instant::now());
    let status = unlink(&env, &caller, first).await;
    let other_live = live_rows_of_family(&pool, &other).await;
    let own_live = live_rows_of_family(&pool, &own).await;
    clean_exact(&pool, ACC_OTHER_FAMILY).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(other_live, 0, "the other family is revoked by the commit");
    assert!(own_live > 0, "the caller's own family stays live");
}

/// An unlink holds the account row lock, deletes the identity and revokes every
/// family before it commits. An attach that started on a live family waits on
/// that lock and, once the unlink commits, is refused: no identity is added and
/// no family stays live.
#[tokio::test(flavor = "multi_thread")]
async fn live_db_unlink_and_attach_race_leaves_no_new_family() {
    let pool = require_db_or_skip!();
    clean_exact(&pool, ACC_RACE).await;
    insert_account(&pool, ACC_RACE, true).await;
    let only = insert_identity(&pool, ACC_RACE, 403).await;
    let (_, family) = refresh::issue(&pool, ACC_RACE, None, Utc::now())
        .await
        .expect("a live family");
    let env = harness(&pool);
    let code = env.stores.handoff.issue(
        HandoffOutcome {
            identity: VerifiedIdentity {
                provider: ProviderId::new("dev").expect("provider id"),
                issuer: "https://issuer.attach.example.org".to_owned(),
                subject: "ct30-404".to_owned(),
                display_name: "Race Player".to_owned(),
                email: None,
                email_verified: false,
                provider_account_created_ms: None,
            },
            intent: Some(LinkIntent {
                account_id: ACC_RACE,
                family_id: family.clone(),
            }),
        },
        &pkce_challenge(VERIFIER),
        CB,
        Instant::now(),
    );
    let mut unlink_tx = pool.begin().await.expect("begin the unlink");
    let holder_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *unlink_tx)
        .await
        .expect("the unlink's backend pid");
    sqlx::query("SELECT account_id FROM account WHERE account_id = $1 FOR UPDATE")
        .bind(ACC_RACE as i32)
        .execute(&mut *unlink_tx)
        .await
        .expect("lock the account row");
    sqlx::query("DELETE FROM account_identity WHERE identity_id = $1")
        .bind(only)
        .execute(&mut *unlink_tx)
        .await
        .expect("delete the identity");
    let router = env.router.clone();
    let attach = tokio::spawn(async move {
        send(
            &router,
            "POST",
            "/token",
            None,
            Some(json!({
                "grant_type": "authorization_code",
                "code": code,
                "code_verifier": VERIFIER,
                "redirect_uri": CB,
            })),
        )
        .await
    });
    let waited = a_backend_waits_on(&pool, holder_pid).await;
    refresh::revoke_account_on(&mut unlink_tx, ACC_RACE, None, Utc::now())
        .await
        .expect("revoke every family");
    unlink_tx.commit().await.expect("commit the unlink");
    let (status, _, body) = attach.await.expect("the attach task");
    let methods_after = methods(&pool, ACC_RACE).await;
    let live = live_rows_of_account(&pool, ACC_RACE).await;
    let old_live = live_rows_of_family(&pool, &family).await;
    clean_exact(&pool, ACC_RACE).await;
    assert!(waited, "the attach waits on the unlink's row lock");
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "invalid_grant");
    assert_eq!(methods_after, 0, "the attach adds no identity");
    assert_eq!(live, 0, "no family is live after the unlink");
    assert_eq!(old_live, 0, "the intent's family is revoked");
}
