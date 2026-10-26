//! Live-DB checks for [`verify_account_password`]: a disabled account answers
//! `Disabled` only to the right password, and the legacy and unknown-name paths
//! spend exactly one argon2id verify. Sentinel ids `0x7000_EB80..=0x7000_EBBF`
//! and names `ct97_player_N`; cleanup deletes by exact id, before and after.

use std::sync::atomic::Ordering;

use sqlx::PgPool;

use super::super::credentials::sha1_upper_hex;
use super::super::password_hash::hash_argon2id;
use super::{verify_account_password, PasswordCheck, ARGON2_VERIFIES};
use crate::test_support::require_db_or_skip;

const ID_DISABLED_WRONG: i32 = 0x7000_EB80;
const ID_DISABLED_RIGHT: i32 = 0x7000_EB81;
const ID_LEGACY_SPEND: i32 = 0x7000_EB82;

/// Delete the sentinel account `account_id` by exact id.
async fn cleanup(pool: &PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert a sentinel account with the given scheme and stored secret. The
/// legacy secret goes in `password`, the argon2id PHC string in
/// `password_hash_v2`.
async fn insert_account(
    pool: &PgPool,
    account_id: i32,
    name: &str,
    password: Option<&str>,
    password_hash_v2: Option<&str>,
    password_algo: i16,
    enabled: bool,
) {
    cleanup(pool, account_id).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_hash_v2, \
         password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, $4, $5, 0, $6)",
    )
    .bind(account_id)
    .bind(name)
    .bind(password)
    .bind(password_hash_v2)
    .bind(password_algo)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert ct97 fixture account");
}

#[tokio::test]
async fn live_db_disabled_account_with_a_wrong_password_is_invalid() {
    let pool = require_db_or_skip!();
    let id = ID_DISABLED_WRONG;
    let name = "ct97_player_1";
    let phc = hash_argon2id("right-pw").expect("hash must succeed");
    insert_account(&pool, id, name, None, Some(&phc), 2, false).await;

    assert_eq!(
        verify_account_password(&pool, name, "wrong-pw").await,
        PasswordCheck::Invalid
    );
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_legacy_row_spends_one_argon2_verify() {
    let pool = require_db_or_skip!();
    let id = ID_LEGACY_SPEND;
    let name = "ct97_player_2";
    let stored = sha1_upper_hex("right-pw");
    insert_account(&pool, id, name, Some(&stored), None, 1, true).await;

    let before = ARGON2_VERIFIES.load(Ordering::SeqCst);
    let answer = verify_account_password(&pool, name, "wrong-pw").await;
    let spent = ARGON2_VERIFIES.load(Ordering::SeqCst) - before;

    assert_eq!(answer, PasswordCheck::Invalid);
    assert_eq!(
        spent, 1,
        "a legacy row must spend exactly one argon2 verify"
    );
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_disabled_account_with_the_right_password_is_disabled() {
    let pool = require_db_or_skip!();
    let id = ID_DISABLED_RIGHT;
    let name = "ct97_player_3";
    let phc = hash_argon2id("right-pw").expect("hash must succeed");
    insert_account(&pool, id, name, None, Some(&phc), 2, false).await;

    assert_eq!(
        verify_account_password(&pool, name, "right-pw").await,
        PasswordCheck::Disabled
    );
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_unknown_name_spends_one_argon2_verify() {
    let pool = require_db_or_skip!();

    let before = ARGON2_VERIFIES.load(Ordering::SeqCst);
    let answer = verify_account_password(&pool, "ct97_player_unknown", "any-pw").await;
    let spent = ARGON2_VERIFIES.load(Ordering::SeqCst) - before;

    assert_eq!(answer, PasswordCheck::Invalid);
    assert_eq!(
        spent, 1,
        "an unknown name must spend exactly one argon2 verify"
    );
}
