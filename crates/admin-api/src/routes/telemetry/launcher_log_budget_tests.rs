//! The log ledgers of C-LF6: whole-chunk admission, the install-day window and
//! its `Retry-After`, the table caps.

use std::time::{Duration, Instant};

use super::dto::IngestError;
use super::launcher_log_budget::LogBudget;

const DAY: u64 = 86_400;

/// The refusal's `(reason, global, retry_after_secs)`.
fn refused(result: Result<(), IngestError>) -> (&'static str, bool, Option<u64>) {
    match result {
        Err(IngestError::LogBudget {
            reason,
            global,
            retry_after_secs,
        }) => (reason, global, retry_after_secs),
        other => panic!("expected a log budget refusal, got {other:?}"),
    }
}

/// **A chunk past the session's rows is refused whole.** 1,000 rows fit a
/// budget of 1,500; the next 1,000 do not, and the 500 left are not taken
/// piecemeal.
#[test]
fn a_chunk_over_the_session_budget_is_refused_whole() {
    let budget = LogBudget::new(1_500, 80_000, 40_000);
    let t0 = Instant::now();
    budget.admit("s", "i", 1_000, t0, 10 * DAY).unwrap();
    assert_eq!(
        refused(budget.admit("s", "i", 1_000, t0, 10 * DAY)),
        ("log_session_budget", false, None)
    );
    budget.admit("s", "i", 500, t0, 10 * DAY).unwrap();
}

/// **The install-day refusal waits for the next UTC midnight.** 90 s before
/// midnight the answer is `Retry-After: 90`; after midnight the install has
/// its rows again.
#[test]
fn the_install_day_refusal_retries_at_utc_midnight() {
    let budget = LogBudget::new(40_000, 1_000, 40_000);
    let t0 = Instant::now();
    let before_midnight = 10 * DAY + DAY - 90;
    budget.admit("a", "i", 1_000, t0, before_midnight).unwrap();
    assert_eq!(
        refused(budget.admit("b", "i", 1, t0, before_midnight)),
        ("log_install_budget", false, Some(90))
    );
    budget.admit("b", "i", 1_000, t0, 11 * DAY).unwrap();
}

/// **The install-day allowance spans sessions and is per install.** A second
/// session of the same install shares it; another install does not.
#[test]
fn the_install_day_allowance_is_per_install_not_per_session() {
    let budget = LogBudget::new(40_000, 1_000, 40_000);
    let t0 = Instant::now();
    budget.admit("a", "i1", 600, t0, 10 * DAY).unwrap();
    assert_eq!(
        refused(budget.admit("b", "i1", 600, t0, 10 * DAY)).0,
        "log_install_budget"
    );
    budget.admit("c", "i2", 600, t0, 10 * DAY).unwrap();
}

/// **The global ceiling answers 503 with the minute's remainder.**
#[test]
fn the_global_minute_ceiling_is_a_503_with_a_retry_after() {
    let budget = LogBudget::new(40_000, 80_000, 1_000);
    let t0 = Instant::now();
    budget.admit("a", "i1", 1_000, t0, 10 * DAY).unwrap();
    let (reason, global, retry) = refused(budget.admit("b", "i2", 1, t0, 10 * DAY));
    assert_eq!((reason, global), ("log_global_budget", true));
    assert!(matches!(retry, Some(1..=61)), "{retry:?}");
    budget
        .admit("b", "i2", 1, t0 + Duration::from_secs(61), 10 * DAY)
        .unwrap();
}

/// **A refused chunk charges nothing.** A 600-row chunk refused by the global
/// ceiling leaves the session's allowance whole: 1,000 rows still fit it in
/// later minutes.
#[test]
fn a_refused_chunk_charges_none_of_the_ledgers() {
    let budget = LogBudget::new(1_000, 80_000, 500);
    let t0 = Instant::now();
    assert_eq!(
        refused(budget.admit("s", "i", 600, t0, 10 * DAY)).0,
        "log_global_budget"
    );
    budget.admit("s", "i", 500, t0, 10 * DAY).unwrap();
    let later = t0 + Duration::from_secs(61);
    budget.admit("s", "i", 500, later, 10 * DAY).unwrap();
    assert_eq!(
        refused(budget.admit("s", "i", 1, later, 10 * DAY)).0,
        "log_session_budget"
    );
}

/// **A zero-row chunk takes no ledger slot.**
#[test]
fn a_zero_row_chunk_takes_no_slot() {
    let budget = LogBudget::new(10, 10, 10).with_capacity(1);
    let t0 = Instant::now();
    budget.admit("a", "i1", 0, t0, 10 * DAY).unwrap();
    assert_eq!(budget.tracked_installs(), 0);
    budget.admit("b", "i2", 1, t0, 10 * DAY).unwrap();
}

/// **The install-day table holds at most its capacity.** A third install
/// takes the place of the entry of the oldest day.
#[test]
fn the_install_day_table_evicts_the_oldest_day() {
    // One session throughout, so only the install table is under test.
    let budget = LogBudget::new(1_000, 10, 1_000).with_capacity(2);
    let t0 = Instant::now();
    budget.admit("s", "i1", 10, t0, 10 * DAY).unwrap();
    budget.admit("s", "i2", 10, t0, 11 * DAY).unwrap();
    budget.admit("s", "i3", 10, t0, 11 * DAY).unwrap();
    assert_eq!(budget.tracked_installs(), 2);
    // Both entries are on today now, so a fourth install evicts one of them
    // and the table still holds two.
    budget.admit("s", "i4", 10, t0, 11 * DAY).unwrap();
    assert_eq!(budget.tracked_installs(), 2);
}

/// **A full session table refuses a new session (fail closed).**
#[test]
fn a_full_session_table_refuses_a_new_session() {
    let budget = LogBudget::new(10, 10, 100).with_capacity(1);
    let t0 = Instant::now();
    budget.admit("a", "i1", 1, t0, 10 * DAY).unwrap();
    assert_eq!(
        refused(budget.admit("b", "i2", 1, t0, 10 * DAY)),
        ("log_session_budget", false, Some(60))
    );
}
