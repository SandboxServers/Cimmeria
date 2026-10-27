//! Live-DB checks for the folded name lookup (CT-80, F13a): an exact match
//! wins, several case variants with no exact match are ambiguous, and a single
//! folded match is used. Sentinel account ids `0x7000_E500..=0x7000_E502` and
//! `0x7000_E503..=0x7000_E504`; cleanup deletes by exact id, before and after
//! each test.

use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{match_name, NameMatch};

const ID_EXACT_OLD: u32 = 0x7000_E500;
const ID_EXACT_NEW: u32 = 0x7000_E501;
const ID_SINGLE: u32 = 0x7000_E502;
const ID_AMBIG_A: u32 = 0x7000_E503;
const ID_AMBIG_B: u32 = 0x7000_E504;

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
        "INSERT INTO account (account_id, account_name, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, 1, 0, true)",
    )
    .bind(account_id as i32)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert sentinel account");
}

#[tokio::test]
async fn live_db_exact_name_wins_over_an_older_case_variant() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_EXACT_OLD, "ct80alice").await;
    insert_account(&pool, ID_EXACT_NEW, "CT80Alice").await;
    let found = match_name(&pool, "CT80Alice").await;
    cleanup(&pool, ID_EXACT_OLD).await;
    cleanup(&pool, ID_EXACT_NEW).await;

    match found.expect("match the exact name") {
        NameMatch::One(facts) => assert_eq!(facts.account_id, ID_EXACT_NEW),
        other => panic!("expected the exact account, got {other:?}"),
    }
}

#[tokio::test]
async fn live_db_two_case_variants_without_an_exact_match_are_ambiguous() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_AMBIG_A, "ct80alice").await;
    insert_account(&pool, ID_AMBIG_B, "Ct80Alice").await;
    let found = match_name(&pool, "CT80ALICE").await;
    cleanup(&pool, ID_AMBIG_A).await;
    cleanup(&pool, ID_AMBIG_B).await;

    assert!(
        matches!(found, Ok(NameMatch::Ambiguous)),
        "two folded matches must be refused, got {found:?}"
    );
}

#[tokio::test]
async fn live_db_single_folded_match_is_used() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_SINGLE, "CT80Bob").await;
    let found = match_name(&pool, "ct80bob").await;
    cleanup(&pool, ID_SINGLE).await;

    match found.expect("match the folded name") {
        NameMatch::One(facts) => assert_eq!(facts.account_id, ID_SINGLE),
        other => panic!("expected the single folded account, got {other:?}"),
    }
}
