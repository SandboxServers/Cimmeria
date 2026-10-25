//! Account lookup for a launch-ticket login (CT-21). The ticket proves which
//! account is signing in; the account row still decides whether it may.

use sqlx::PgPool;

use super::credentials::AuthCredError;

/// Reads `enabled`, `accesslevel` and `test_account` for a ticket's account and
/// returns `(account_id, access_level, test_account)`, as `validate_credentials`
/// does for a password login.
///
/// A disabled account is `AccountDisabled`; a missing row is
/// `InvalidCredentials`. With no pool, only the developer no-database setup
/// (`developer_mode && !db_configured`) accepts the ticket, as access level 99.
/// Any other missing pool is `DbError`, the same fail-closed refusal as
/// `handle_user_auth`.
pub(super) async fn ticket_account(
    db: Option<&PgPool>,
    developer_mode: bool,
    db_configured: bool,
    account_id: u32,
) -> Result<(u32, u32, bool), AuthCredError> {
    let Some(db) = db else {
        return if developer_mode && !db_configured {
            Ok((account_id, 99, false))
        } else {
            Err(AuthCredError::DbError("no database connection".to_string()))
        };
    };
    let row: Option<(bool, i32, bool)> = sqlx::query_as(
        "SELECT enabled, accesslevel, test_account FROM account WHERE account_id = $1",
    )
    .bind(account_id as i32)
    .fetch_optional(db)
    .await
    .map_err(|e| AuthCredError::DbError(e.to_string()))?;
    match row {
        None => Err(AuthCredError::InvalidCredentials),
        Some((false, _, _)) => Err(AuthCredError::AccountDisabled),
        Some((true, access_level, test_account)) => {
            Ok((account_id, access_level.max(0) as u32, test_account))
        }
    }
}
