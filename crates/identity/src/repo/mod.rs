//! Repositories for the identity tables (contract C5): the linked identities,
//! the local accounts they attach to, and the audit events.
//!
//! Every query is a runtime-checked `sqlx::query`. Database errors leave this
//! module as [`RepoError`] only, so no SQL text, subject or email reaches a
//! caller, a log line or a response body (R7).

pub mod account;
pub mod event;
pub mod identity;
pub mod refresh;

#[cfg(test)]
#[path = "tests_live_db.rs"]
mod tests_live_db;

#[cfg(test)]
#[path = "guard_tests_live_db.rs"]
mod guard_tests_live_db;

/// A repository failure, with no database text in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RepoError {
    /// The query failed for a reason the caller cannot act on.
    #[error("database error")]
    Db,
    /// A uniqueness rule or a C5 guard refused the write.
    #[error("conflict")]
    Conflict,
}

impl From<sqlx::Error> for RepoError {
    /// SQLSTATE `23505` (unique violation) and `23514` (check violation, which
    /// the C5 guard triggers raise) are [`RepoError::Conflict`]; anything else
    /// is [`RepoError::Db`].
    fn from(error: sqlx::Error) -> RepoError {
        match error.as_database_error().and_then(|e| e.code()).as_deref() {
            Some("23505" | "23514") => RepoError::Conflict,
            code => {
                tracing::warn!(
                    target: "cimmeria_identity",
                    reason = "db_error",
                    sqlstate = code.unwrap_or("none"),
                    "identity repository query failed"
                );
                RepoError::Db
            }
        }
    }
}
