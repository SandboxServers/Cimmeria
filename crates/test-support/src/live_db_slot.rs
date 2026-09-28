//! Per-slot live-DB databases: the one place a test process decides which
//! database it talks to.
//!
//! `tools/test-live-db.sh` loads the schema into the database `DATABASE_URL`
//! names (`sgw`, or `sgw_<worktree>`), then clones it into one database per
//! slot of nextest's `live-db` test group: `sgw_0` .. `sgw_<N-1>`. nextest
//! runs at most N live-DB tests at once and gives each a slot number in
//! `NEXTEST_TEST_GROUP_SLOT` that no other running test in the group holds,
//! so a test that uses database `<name>_<slot>` has it to itself.
//!
//! [`database_url`] resolves that URL once per process and writes it back to
//! `DATABASE_URL`, so everything the test does afterwards (a second pool, a
//! helper that reads the variable, a child process) lands on the same
//! database. The live-DB gate calls it before it opens its pool. Outside the
//! group (plain `cargo test`, or another nextest profile) the URL is used
//! as given. The `database_url_is_only_resolved_by_the_gate` guard in this
//! crate fails if test code reads `DATABASE_URL` any other way.

use std::sync::OnceLock;

/// The nextest test group whose tests get a database per slot. Must match
/// `[test-groups]` in `.config/nextest.toml`.
pub const LIVE_DB_GROUP: &str = "live-db";

/// Set in the environment once [`database_url`] has rewritten
/// `DATABASE_URL`, so a child process that is itself a test does not add a
/// second slot suffix.
const RESOLVED_ENV: &str = "CIMMERIA_LIVE_DB_URL_RESOLVED";

/// The database URL this test process must use, or `None` when
/// `DATABASE_URL` is unset or empty. Inside nextest's `live-db` group it is
/// the slot's clone (`.../sgw` becomes `.../sgw_3` in slot 3).
///
/// # Panics
///
/// When a slot applies but the URL names no database to suffix.
pub fn database_url() -> Option<String> {
    static URL: OnceLock<Option<String>> = OnceLock::new();
    URL.get_or_init(|| {
        let base = std::env::var("DATABASE_URL")
            .ok()
            .filter(|u| !u.is_empty())?;
        if std::env::var_os(RESOLVED_ENV).is_some() {
            return Some(base);
        }
        let group = std::env::var("NEXTEST_TEST_GROUP").ok();
        let slot = std::env::var("NEXTEST_TEST_GROUP_SLOT").ok();
        let url = slot_url(&base, group.as_deref(), slot.as_deref())
            .unwrap_or_else(|e| panic!("live-DB slot: {e}"));
        // Written back so later readers and child processes inherit the
        // slot. This runs from the gate, before the test has opened a
        // connection or started any work that could read the environment
        // concurrently.
        std::env::set_var("DATABASE_URL", &url);
        std::env::set_var(RESOLVED_ENV, "1");
        Some(url)
    })
    .clone()
}

/// The URL for `slot` of `group`: `base` with `_<slot>` appended to its
/// database name when the test runs in the [`LIVE_DB_GROUP`], else `base`.
pub(crate) fn slot_url(
    base: &str,
    group: Option<&str>,
    slot: Option<&str>,
) -> Result<String, String> {
    let (Some(LIVE_DB_GROUP), Some(slot)) = (group, slot) else {
        return Ok(base.to_string());
    };
    let slot: u32 = slot
        .parse()
        .map_err(|_| format!("NEXTEST_TEST_GROUP_SLOT={slot:?} is not a number"))?;
    let (head, query) = match base.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (base, None),
    };
    let after_scheme = head.find("://").map_or(0, |i| i + 3);
    let names_a_database = head[after_scheme..]
        .find('/')
        .is_some_and(|i| after_scheme + i + 1 < head.len());
    if !names_a_database {
        return Err("DATABASE_URL names no database, so there is no per-slot clone to use".into());
    }
    let mut url = format!("{head}_{slot}");
    if let Some(q) = query {
        url.push('?');
        url.push_str(q);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End to end: under the `ci-live-db` profile this test runs in the
    /// `live-db` group, and the gate's pool, the URL it resolved and the
    /// `DATABASE_URL` a child process would inherit all name the slot's
    /// clone. Fails if nextest stops exporting the group variables, or the
    /// clones are not made.
    #[tokio::test]
    async fn live_db_each_slot_talks_to_its_own_clone() {
        let pool = crate::require_db_or_skip!();
        let db: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&pool)
            .await
            .unwrap();
        let url = database_url().expect("the gate resolved a URL");
        assert_eq!(
            url.split('?').next().unwrap().rsplit('/').next(),
            Some(db.as_str())
        );
        assert_eq!(
            std::env::var("DATABASE_URL").as_deref(),
            Ok(url.as_str()),
            "child processes inherit the slot's URL"
        );
        if std::env::var("NEXTEST_PROFILE").as_deref() == Ok("ci-live-db") {
            assert_eq!(
                std::env::var("NEXTEST_TEST_GROUP").as_deref(),
                Ok(LIVE_DB_GROUP)
            );
            let slot = std::env::var("NEXTEST_TEST_GROUP_SLOT").expect("nextest sets the slot");
            assert!(
                db.ends_with(&format!("_{slot}")),
                "{db} is not slot {slot}'s clone"
            );
        }
    }

    #[test]
    fn a_live_db_slot_suffixes_the_database_name() {
        let base = "postgres://w-testing:w-testing@localhost:5433/sgw_agent_x";
        assert_eq!(
            slot_url(base, Some("live-db"), Some("3")).unwrap(),
            "postgres://w-testing:w-testing@localhost:5433/sgw_agent_x_3"
        );
        assert_eq!(
            slot_url(
                "postgres://u@h/sgw?sslmode=disable",
                Some("live-db"),
                Some("0")
            )
            .unwrap(),
            "postgres://u@h/sgw_0?sslmode=disable"
        );
    }

    /// Outside the group (non-DB tests, other profiles, plain `cargo test`)
    /// the URL is left alone.
    #[test]
    fn outside_the_group_the_url_is_unchanged() {
        let base = "postgres://u@h:5432/sgw";
        assert_eq!(slot_url(base, None, None).unwrap(), base);
        assert_eq!(slot_url(base, Some("@global"), Some("2")).unwrap(), base);
        assert_eq!(slot_url(base, Some("live-db"), None).unwrap(), base);
    }

    #[test]
    fn a_url_without_a_database_name_is_an_error() {
        for base in ["postgres://u@h:5432", "postgres://u@h:5432/"] {
            assert!(
                slot_url(base, Some("live-db"), Some("1")).is_err(),
                "{base}"
            );
        }
        assert!(slot_url("postgres://u@h/sgw", Some("live-db"), Some("x")).is_err());
    }
}
