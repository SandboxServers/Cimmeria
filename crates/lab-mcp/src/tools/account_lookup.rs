//! An account name to its id, for the chaos tools' `account` argument
//! (lab-chaos CH-14, D-CH10). Compiled into tests as well as `lab-chaos`
//! builds, so the ordinary live-DB tier runs its query.

use sqlx::PgPool;

/// The id of the account named `name`, or `None` when no account has that
/// name. Matched exactly as login matches it (`account_name = $1` in the
/// auth credential lookup), so an arm waits for the account that name logs
/// in to, not one that differs only in case.
pub(super) async fn account_id_named(pool: &PgPool, name: &str) -> Result<Option<u32>, String> {
    let id: Option<i32> =
        sqlx::query_scalar("SELECT account_id FROM account WHERE account_name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("account lookup failed: {e}"))?;
    // Sessions carry the id as u32; a negative one cannot log in to a
    // session an arm could reach.
    Ok(id.and_then(|id| u32::try_from(id).ok()))
}

#[cfg(test)]
mod tests {
    use super::account_id_named;
    use crate::test_support::require_db_or_skip;

    /// Sentinel account for this test alone.
    const ACCOUNT: i32 = 0x7046_1400;

    async fn cleanup(pool: &sqlx::PgPool) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(ACCOUNT)
            .execute(pool)
            .await;
    }

    /// The name resolves to its id; another case and an unknown name do not.
    /// Fails if the match goes case-insensitive (an arm would wait for an
    /// account login never picks) or reads the wrong column.
    #[tokio::test]
    async fn live_db_account_id_named_matches_like_login() {
        let pool = require_db_or_skip!();
        cleanup(&pool).await;
        let name = format!("ch14-{ACCOUNT}");
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, accesslevel) \
             VALUES ($1, $2, '', 0)",
        )
        .bind(ACCOUNT)
        .bind(&name)
        .execute(&pool)
        .await
        .expect("insert account");
        let exact = account_id_named(&pool, &name).await;
        let upper = account_id_named(&pool, &name.to_uppercase()).await;
        let unknown = account_id_named(&pool, "ch14-no-such-account").await;
        cleanup(&pool).await;
        assert_eq!(exact, Ok(Some(ACCOUNT as u32)));
        assert_eq!(upper, Ok(None), "login matches the name exactly");
        assert_eq!(unknown, Ok(None));
    }
}
