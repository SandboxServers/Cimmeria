//! Local accounts as the identity flows see them (`account`, contract C5 and
//! C15): their facts for linking, creation under a free name, and whether a
//! password is set.

use sqlx::PgPool;

use super::RepoError;
use crate::link::AccountFacts;

/// The columns every facts query selects, in order: `account_id`, `enabled`,
/// `accesslevel`, `test_account`.
pub(super) type FactsRow = (i32, bool, i32, bool);

/// Builds the linking facts from one row of [`FactsRow`]'s columns.
pub(super) fn facts_of(
    account_id: i32,
    enabled: bool,
    accesslevel: i32,
    test_account: bool,
) -> AccountFacts {
    AccountFacts {
        account_id: account_id as u32,
        enabled,
        accesslevel,
        test_account,
    }
}

/// The linking facts of one account by id, or `None` when it does not exist.
pub async fn facts(pool: &PgPool, account_id: u32) -> Result<Option<AccountFacts>, RepoError> {
    let row: Option<FactsRow> = sqlx::query_as(
        "SELECT account_id, enabled, accesslevel, test_account FROM account WHERE account_id = $1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, enabled, level, test)| facts_of(id, enabled, level, test)))
}

/// The linking facts of the account named `name`, compared without case, or
/// `None`. The oldest match wins when more than one account has the name.
pub async fn facts_by_name(pool: &PgPool, name: &str) -> Result<Option<AccountFacts>, RepoError> {
    let row: Option<FactsRow> = sqlx::query_as(
        "SELECT account_id, enabled, accesslevel, test_account FROM account \
         WHERE lower(account_name) = lower($1) ORDER BY account_id LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, enabled, level, test)| facts_of(id, enabled, level, test)))
}

/// Creates a player account under the first candidate name nobody holds, with
/// no password, access level 0, enabled and not a test account. Returns the
/// new id and the name it got.
///
/// The `account` table has no unique index on `lower(account_name)`, so the
/// free check is a `NOT EXISTS` in the same statement. Creates are serialized
/// by a transaction-scoped advisory lock, so two racing creates never get one
/// name.
///
/// Returns [`RepoError::Conflict`] when every candidate is taken.
pub async fn create(pool: &PgPool, candidates: &[String]) -> Result<(u32, String), RepoError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(2000201)")
        .execute(&mut *tx)
        .await?;
    for name in candidates {
        let row: Option<(i32,)> = sqlx::query_as(
            "INSERT INTO account (account_name, accesslevel, enabled, test_account) \
             SELECT $1::VARCHAR, 0, true, false \
             WHERE NOT EXISTS (SELECT 1 FROM account WHERE lower(account_name) = lower($1::VARCHAR)) \
             RETURNING account_id",
        )
        .bind(name)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id,)) = row {
            tx.commit().await?;
            return Ok((id as u32, name.clone()));
        }
    }
    tx.commit().await?;
    Err(RepoError::Conflict)
}

/// Deletes `account_id` when it holds no identity row. Returns whether a row was removed.
/// `link/new` calls it when the identity insert fails, so no orphan account is left.
pub async fn remove_unlinked(pool: &PgPool, account_id: u32) -> Result<bool, RepoError> {
    let done = sqlx::query(
        "DELETE FROM account a WHERE a.account_id = $1 \
         AND NOT EXISTS (SELECT 1 FROM account_identity i WHERE i.account_id = a.account_id)",
    )
    .bind(account_id as i32)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Whether the account holds a password (`password` or `password_hash_v2`).
/// A missing account has none. Read by CT-45 only.
pub async fn has_password(pool: &PgPool, account_id: u32) -> Result<bool, RepoError> {
    let row: Option<(bool,)> = sqlx::query_as(
        "SELECT password IS NOT NULL OR password_hash_v2 IS NOT NULL \
         FROM account WHERE account_id = $1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some_and(|(has,)| has))
}
