//! Live-DB check for the GM `.account linkcode` refusal on an ambiguous name
//! (CT-80, F13a): two accounts share a name when case is ignored, the GM types
//! it in capitals, and no link code is issued for either. Sentinel account ids
//! `0x7000_E580..=0x7000_E581`; cleanup deletes by exact id, before and after
//! the test.

use std::time::Instant;

use cimmeria_identity::link_code::LinkCodeStore;
use cimmeria_test_support::require_db_or_skip;
use sqlx::PgPool;

use super::{run, AdminAction, AdminCaller};

const ID_FIRST: u32 = 0x7000_E580;
const ID_SECOND: u32 = 0x7000_E581;

/// The GM who types the command in this check.
const GM: AdminCaller = AdminCaller {
    entity_id: 9_001,
    entity_name: String::new(),
    account_id: Some(1),
    gm_account_name: String::new(),
};

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
async fn live_db_gm_linkcode_refuses_an_ambiguous_name() {
    let pool = require_db_or_skip!();
    insert_account(&pool, ID_FIRST, "CT80Linked").await;
    insert_account(&pool, ID_SECOND, "ct80linked").await;
    let codes = LinkCodeStore::default();
    let lines = run(
        &GM,
        "CT80LINKED",
        AdminAction::LinkCode,
        Some(&pool),
        &codes,
        Instant::now(),
    )
    .await;
    let issued: Result<i64, sqlx::Error> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM account_identity_event \
         WHERE account_id IN ($1, $2) AND kind = 'link_code_issued'",
    )
    .bind(ID_FIRST as i32)
    .bind(ID_SECOND as i32)
    .fetch_one(&pool)
    .await;
    cleanup(&pool, ID_FIRST).await;
    cleanup(&pool, ID_SECOND).await;

    assert_eq!(
        lines,
        vec![
            "account: more than one account is named CT80LINKED when case is ignored; type the name exactly as it is stored"
                .to_owned()
        ]
    );
    assert_eq!(issued.expect("count link codes"), 0, "a code was issued");
}
