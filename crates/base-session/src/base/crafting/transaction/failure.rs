//! Why a crafting transaction did not commit, and the `persist_failed`
//! event for every rollback that is not a game-rule refusal.

use sqlx::postgres::PgQueryResult;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::telemetry::{sql_error_class, sqlstate, JobIds};

/// Why a transaction did not commit. Every variant rolls back.
#[derive(Debug)]
pub enum CraftTxError {
    /// A game rule refused it; the player reads the reject's line, and the
    /// reject logs `rejected`.
    Rejected(CraftReject),
    /// The database failed during `phase`.
    Db {
        phase: &'static str,
        error: sqlx::Error,
    },
    /// A write under row locks changed a different number of rows than it
    /// had to: the inventory is not what the transaction locked.
    RowsAffected {
        phase: &'static str,
        rows_affected: u64,
        expected: u64,
    },
    /// The plan or the data cannot be applied (`reason`: a non-positive
    /// cost, an unknown product, a missing player row).
    Invalid {
        phase: &'static str,
        reason: &'static str,
    },
}

impl From<CraftReject> for CraftTxError {
    fn from(r: CraftReject) -> Self {
        CraftTxError::Rejected(r)
    }
}

/// Wrap a database error with the phase it happened in.
pub(super) fn at(phase: &'static str) -> impl FnOnce(sqlx::Error) -> CraftTxError {
    move |error| CraftTxError::Db { phase, error }
}

/// Check a write changed exactly `expected` rows. A mismatch is logged
/// here, with the paired `rows_affected` and `expected`, and rolls back.
pub(super) fn expect_rows(
    ids: &JobIds,
    phase: &'static str,
    done: PgQueryResult,
    expected: u64,
) -> Result<(), CraftTxError> {
    let rows_affected = done.rows_affected();
    if rows_affected == expected {
        return Ok(());
    }
    tracing::warn!(
        target: "crafting",
        event = "persist_failed",
        job_id = ids.job_id,
        account_id = ids.account_id,
        player_id = ids.player_id,
        entity_id = ids.entity_id,
        phase,
        reason = "rows_affected_mismatch",
        rows_affected,
        expected,
        "crafting transaction write changed an unexpected number of rows -- rolled back, nothing applied"
    );
    Err(CraftTxError::RowsAffected {
        phase,
        rows_affected,
        expected,
    })
}

/// Log `persist_failed` (WARN) for a rollback that is not a game-rule
/// refusal. A rows-affected mismatch was already logged where it
/// happened.
pub(super) fn log_persist_failed(ids: &JobIds, err: &CraftTxError) {
    match err {
        CraftTxError::Rejected(_) | CraftTxError::RowsAffected { .. } => {}
        CraftTxError::Db { phase, error } => tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            job_id = ids.job_id,
            account_id = ids.account_id,
            player_id = ids.player_id,
            entity_id = ids.entity_id,
            phase,
            reason = "db_error",
            error_class = sql_error_class(error),
            sqlstate = %sqlstate(error),
            error = %error,
            "crafting transaction failed -- rolled back, nothing applied"
        ),
        CraftTxError::Invalid { phase, reason } => tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            job_id = ids.job_id,
            account_id = ids.account_id,
            player_id = ids.player_id,
            entity_id = ids.entity_id,
            phase,
            reason,
            "crafting transaction refused its plan -- rolled back, nothing applied"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    const IDS: JobIds = JobIds {
        job_id: 41,
        verb: "test_plan",
        account_id: 42,
        player_id: 43,
        entity_id: 44,
    };

    fn assert_identity(e: &crate::test_support::Captured) {
        assert!(e.has_field("job_id", "41"), "{e:#?}");
        assert!(e.has_field("account_id", "42"), "{e:#?}");
        assert!(e.has_field("player_id", "43"), "{e:#?}");
        assert!(e.has_field("entity_id", "44"), "{e:#?}");
    }

    /// A write under lock that changed no row rolls back and logs the
    /// paired `rows_affected` and `expected` with its phase.
    #[test]
    fn a_write_that_changed_no_row_is_a_warning_with_the_counts() {
        let capture = LogCapture::install();
        let result = expect_rows(&IDS, "consume", PgQueryResult::default(), 1);
        assert!(matches!(
            result,
            Err(CraftTxError::RowsAffected {
                phase: "consume",
                rows_affected: 0,
                expected: 1
            })
        ));
        let e = capture
            .find_event(
                Level::WARN,
                "unexpected number of rows",
                "rows_affected_mismatch",
            )
            .expect("persist_failed WARN");
        assert_eq!(e.target, "crafting");
        assert!(e.has_field("event", "persist_failed"));
        assert!(e.has_field("phase", "consume"));
        assert!(e.has_field("rows_affected", "0"));
        assert!(e.has_field("expected", "1"));
        assert_identity(&e);
    }

    #[test]
    fn a_database_error_names_its_phase_and_class() {
        let capture = LogCapture::install();
        log_persist_failed(
            &IDS,
            &CraftTxError::Db {
                phase: "commit",
                error: sqlx::Error::PoolTimedOut,
            },
        );
        let e = capture
            .find_event(Level::WARN, "rolled back", "db_error")
            .expect("persist_failed WARN");
        assert!(e.has_field("event", "persist_failed"));
        assert!(e.has_field("phase", "commit"));
        assert!(e.has_field("error_class", "pool"));
        assert_identity(&e);
    }

    /// A game-rule refusal is `rejected`, not `persist_failed`.
    #[test]
    fn a_refusal_is_not_a_persist_failure() {
        let capture = LogCapture::install();
        log_persist_failed(&IDS, &CraftTxError::Rejected(CraftReject::InductionFailed));
        assert!(capture
            .all()
            .iter()
            .all(|e| !e.has_field("event", "persist_failed")));
    }
}
