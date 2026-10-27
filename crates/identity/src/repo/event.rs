//! The audit trail of identity changes (`account_identity_event`, contract C5).
//! A row holds no subject, email or IP address.

use sqlx::{PgConnection, PgPool};

use super::RepoError;

/// The audited changes. [`EventKind::as_str`] gives the text the table's
/// `kind` CHECK accepts; the test `live_db_events_record_every_kind` keeps the
/// two lists in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A new account was created from a provider sign-in.
    Signup,
    /// An identity was linked to an existing account by the player.
    Link,
    /// An identity was linked automatically to a verified email match.
    AutoLink,
    /// An identity was removed from an account.
    Unlink,
    /// A link attempt was refused.
    LinkRefused,
    /// A refresh token was presented after it had been rotated.
    RefreshReuse,
    /// A password was cleared from an account that got an identity (C15 a).
    PasswordCleared,
    /// A GM issued a link code (C15 b).
    LinkCodeIssued,
}

impl EventKind {
    /// The text stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Signup => "signup",
            EventKind::Link => "link",
            EventKind::AutoLink => "auto_link",
            EventKind::Unlink => "unlink",
            EventKind::LinkRefused => "link_refused",
            EventKind::RefreshReuse => "refresh_reuse",
            EventKind::PasswordCleared => "password_cleared",
            EventKind::LinkCodeIssued => "link_code_issued",
        }
    }
}

/// Writes one event row on a pool connection.
pub async fn record(
    pool: &PgPool,
    account_id: u32,
    identity_id: Option<i64>,
    kind: EventKind,
    provider: Option<&str>,
) -> Result<(), RepoError> {
    let mut conn = pool.acquire().await?;
    record_tx(&mut conn, account_id, identity_id, kind, provider).await
}

/// Writes one event row on the given connection or transaction, so the row
/// commits or rolls back with the change it records.
pub async fn record_tx(
    conn: &mut PgConnection,
    account_id: u32,
    identity_id: Option<i64>,
    kind: EventKind,
    provider: Option<&str>,
) -> Result<(), RepoError> {
    sqlx::query(
        "INSERT INTO account_identity_event (account_id, identity_id, kind, provider) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(account_id as i32)
    .bind(identity_id)
    .bind(kind.as_str())
    .bind(provider)
    .execute(conn)
    .await?;
    Ok(())
}
