//! SS-C1, PR #893 review: `chatIgnore`'s duplicate and cap checks are made
//! in the database under the list row's lock, not on a snapshot. Type 5
//! (the forced race at 99) and a live-DB case-folded duplicate. Sentinels
//! `0x7300_C24x` / `C25x`, beside the rest of `chat_ignore`'s.

use std::sync::Arc;

use super::super::ignore::ignore_full_text;
use super::chat_ignore::{cleanup, insert_player, refused, Harness, TEST_BASE};
use crate::base::contact_list::ignore::MAX_IGNORE_LIST_MEMBERS;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Type 5 (PR #893 review): the owner's Ignore list holds 99 names and two
/// `chatIgnore` adds overlap. Exactly one gets the 100th slot: the add runs
/// under the list row's `FOR UPDATE` lock, so the second counts 100 after
/// the first commits and is refused as full. Without the lock both count 99
/// and the list ends at 101.
///
/// The race is forced as in SS-M1's `concurrent_sends_respect_mailbox_cap`:
/// a `SHARE` lock on `sgw_contact_list_member` lets both adds read and count
/// but blocks every insert, and is held until both are parked behind it.
#[tokio::test]
async fn concurrent_ignore_adds_respect_the_cap() {
    let pool = require_db_or_skip!();
    let owner = (TEST_BASE + 40, TEST_BASE + 41);
    let a = (TEST_BASE + 42, TEST_BASE + 43);
    let b = (TEST_BASE + 44, TEST_BASE + 45);
    cleanup(&pool, &[owner, a, b]).await;
    insert_player(&pool, owner.0, owner.1, "ssc1-owner-race").await;
    insert_player(&pool, a.0, a.1, "SsC1RaceA").await;
    insert_player(&pool, b.0, b.1, "SsC1RaceB").await;
    let list_id = crate::base::contact_list::ignore::ensure_ignore_list(&pool, owner.1)
        .await
        .unwrap();
    let filler: Vec<String> = (0..MAX_IGNORE_LIST_MEMBERS - 1)
        .map(|i| format!("race-filler-{i}"))
        .collect();
    sqlx::query(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) \
         SELECT $1, n FROM UNNEST($2::text[]) AS t(n)",
    )
    .bind(list_id)
    .bind(&filler)
    .execute(&pool)
    .await
    .unwrap();

    let h = Harness::new(owner.1, Some(Arc::new(pool.clone())));
    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE sgw_contact_list_member IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let release = async {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "WITH held AS (\
                     SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))\
                 ) \
                 SELECT COUNT(*) FROM pg_stat_activity a \
                 WHERE a.pid IN (SELECT pid FROM held) \
                    OR EXISTS (SELECT 1 FROM held h WHERE h.pid = ANY(pg_blocking_pids(a.pid)))",
            )
            .bind(gate_pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting >= 2 {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "both adds should be parked behind the gate (saw {waiting})"
            );
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        gate.commit().await.unwrap();
    };
    tokio::join!(h.ignore("SsC1RaceA", 1), h.ignore("SsC1RaceB", 1), release);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 100,
        "exactly one of the two racing adds may take the last slot"
    );
    let lines: Vec<String> = h
        .take()
        .into_iter()
        .filter(|(m, _)| *m == method_idx::ON_PLAYER_COMMUNICATION)
        .map(|p| Harness::last_feedback(&[p]))
        .collect();
    assert!(
        lines.contains(&ignore_full_text()),
        "the loser is told: {lines:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("You are now ignoring"))
            .count(),
        1,
        "{lines:?}"
    );

    cleanup(&pool, &[owner, a, b]).await;
}

/// A duplicate in a different case is refused by the database check, not a
/// snapshot: "ssc1pest" is on the list, "SsC1Pest" is the same entry.
#[tokio::test]
async fn chat_ignore_refuses_a_case_folded_duplicate() {
    let capture = LogCapture::install();
    let pool = require_db_or_skip!();
    let owner = (TEST_BASE + 50, TEST_BASE + 51);
    let target = (TEST_BASE + 52, TEST_BASE + 53);
    cleanup(&pool, &[owner, target]).await;
    insert_player(&pool, owner.0, owner.1, "ssc1-owner-dup").await;
    insert_player(&pool, target.0, target.1, "SsC1DupPest").await;
    let list_id = crate::base::contact_list::ignore::ensure_ignore_list(&pool, owner.1)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES ($1, 'ssc1duppest')",
    )
    .bind(list_id)
    .execute(&pool)
    .await
    .unwrap();
    let h = Harness::new(owner.1, Some(Arc::new(pool.clone())));
    h.ignore("SsC1DupPest", 1).await;
    assert_eq!(
        Harness::last_feedback(&h.take()),
        "ssc1duppest is already on your Ignore list."
    );
    assert!(refused(&capture, "already_ignored"));
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_contact_list_member WHERE list_id = $1")
            .bind(list_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
    cleanup(&pool, &[owner, target]).await;
}
