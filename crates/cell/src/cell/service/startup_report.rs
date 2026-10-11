//! Startup cache-load failures: one WARN per failed cache, then one INFO
//! summary per boot.
//!
//! Each startup cache that fails to load logs `event = "cache_load_failed"`
//! with the cache name, the error and what the failure costs. The caller keeps
//! the names [`cache_failed`] returns, and [`startup_summary`] runs once after
//! the loads, so an operator reads the boot's failures in one row.

use std::fmt::Display;

/// Log one failed startup cache load and return its name, so the caller can
/// collect it for [`startup_summary`]. `consequence` says what the process
/// runs without; it is a constant per call site.
pub(super) fn cache_failed(
    cache: &'static str,
    error: impl Display,
    consequence: &'static str,
) -> &'static str {
    tracing::warn!(
        target: "startup",
        event = "cache_load_failed",
        cache,
        error = %error,
        consequence,
        "startup cache failed to load"
    );
    cache
}

/// Log the boot's summary row: how many startup caches failed and which.
/// Runs once per boot, after every cache load, whatever the count.
pub(super) fn startup_summary(failed: &[&'static str]) {
    tracing::info!(
        target: "startup",
        event = "startup_caches",
        failed_count = failed.len(),
        failed_caches = ?failed,
        "startup caches loaded"
    );
}

#[cfg(test)]
mod tests {
    use super::{cache_failed, startup_summary};
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// One failed cache: one WARN with the event, the cache name, the error
    /// as a field (not in the message) and the consequence.
    #[test]
    fn cache_failed_logs_structured_warn() {
        let capture = LogCapture::install();
        let name = cache_failed("spawn_records", "db timeout", "using the fallback");
        assert_eq!(name, "spawn_records");
        let row = capture
            .find_message(Level::WARN, "startup cache failed to load")
            .expect("cache_load_failed WARN must be captured");
        assert!(row.has_field("event", "cache_load_failed"), "{row:#?}");
        assert!(row.has_field("cache", "spawn_records"), "{row:#?}");
        assert!(row.has_field("error", "db timeout"), "{row:#?}");
        assert!(
            row.has_field("consequence", "using the fallback"),
            "{row:#?}"
        );
    }

    /// Two failed caches give one summary row with `failed_count = 2` and
    /// both names. Dropping a failure from the list shows up here.
    #[test]
    fn startup_summary_counts_two_failures() {
        let capture = LogCapture::install();
        let failed = vec![
            cache_failed("spawn_records", "a", "x"),
            cache_failed("stargates", "b", "y"),
        ];
        startup_summary(&failed);
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.level == Level::INFO && c.has_field("event", "startup_caches"))
            .collect();
        assert_eq!(rows.len(), 1, "{rows:#?}");
        assert!(rows[0].has_field("failed_count", "2"), "{rows:#?}");
        assert!(rows[0].has_field("failed_caches", "[\"spawn_records\", \"stargates\"]"));
    }

    /// A boot with no failed caches still logs the summary, with a count of
    /// zero: the row is the boot's record, not only a failure report.
    #[test]
    fn startup_summary_logs_zero_when_nothing_failed() {
        let capture = LogCapture::install();
        startup_summary(&[]);
        let row = capture
            .find_message(Level::INFO, "startup caches loaded")
            .expect("summary must be captured");
        assert!(row.has_field("failed_count", "0"), "{row:#?}");
    }

    /// `start()` on a bare service (no database) logs exactly one summary
    /// row for the boot. Removing the call from `start()` fails this test.
    #[tokio::test]
    async fn start_logs_one_startup_summary_per_boot() {
        let config = cimmeria_common::ServerConfig::loopback();
        let mut svc = crate::cell::service::CellService::new(&config);
        let capture = LogCapture::install();
        svc.start().await.unwrap();
        let rows = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "startup_caches"))
            .count();
        assert_eq!(rows, 1, "one summary per boot, not per cache or row");
    }
}
