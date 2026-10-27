//! Build the served cooked-data catalog at base-service start (LP-11).
//!
//! The build runs on a blocking thread once the resource cache has loaded, and
//! boot never waits on it. Its outcome is published once through
//! [`served_catalog::publish`]: a launcher that asks before the build ends
//! waits, and one that asks after a failure gets `catalog_failed` at once
//! instead of at its deadline (D-LP13).

use std::sync::Arc;
use std::time::Instant;

use cimmeria_resources::base::resources::ResourceCache;
use cimmeria_resources::base::served_catalog::{self, failure_code, Published, ServedCatalog};

/// The reason code for a build that never ran to an outcome: the resource
/// cache failed to load, or the build task panicked or was cancelled.
const BUILD_FAILED: &str = "build_failed";

/// Start the presync build once the resource cache load has a result. A cache
/// that loaded gets the background build (which publishes); one that failed
/// publishes `catalog_failed` at once, so no launcher waits out its deadline
/// on a catalog that will never come (D-LP13). Nothing is logged for the
/// outcome when `publish` refuses.
pub(super) fn presync_after_cache_load(
    loaded: Result<ResourceCache, String>,
    data_dir: &str,
    spawn: impl FnOnce(Arc<ResourceCache>, String),
    publish: impl FnOnce(Published) -> bool,
) -> Option<Arc<ResourceCache>> {
    match loaded {
        Ok(cache) => {
            let cache = Arc::new(cache);
            spawn(Arc::clone(&cache), data_dir.to_owned());
            Some(cache)
        }
        Err(e) => {
            // Same row the cell's `startup_report::cache_failed` writes;
            // base does not depend on the cell crate.
            tracing::warn!(
                event = "cache_load_failed",
                cache = "resource_cache",
                error = %e,
                consequence = "the base runs without the resource cache",
                "startup cache failed to load"
            );
            if publish(Published::Failed(BUILD_FAILED)) {
                tracing::warn!(
                    event = "cooked_presync.catalog_unavailable",
                    reason = BUILD_FAILED,
                    error = %e,
                    "the resource cache failed to load; launchers get catalog_failed"
                );
            }
            None
        }
    }
}

/// Publish `build_failed` when the build task ended in a `JoinError` (panic or
/// cancellation); a task that returned normally already published.
fn on_build_task_ended(
    ended: Result<(), tokio::task::JoinError>,
    publish: impl FnOnce(Published) -> bool,
) {
    if let Err(join_error) = ended {
        if publish(Published::Failed(BUILD_FAILED)) {
            tracing::warn!(
                event = "cooked_presync.catalog_unavailable",
                reason = BUILD_FAILED,
                error = %join_error,
                "the presync catalog build task ended without a result"
            );
        }
    }
}

/// Build the catalog for `cache` on a blocking thread and publish the outcome.
/// Returns at once: the caller never waits on the build.
pub(super) fn spawn_presync_catalog(cache: Arc<ResourceCache>, data_dir: String) {
    tokio::spawn(async move {
        let built = tokio::task::spawn_blocking(move || {
            build_and_publish(&cache, &data_dir, served_catalog::publish);
        })
        .await;
        on_build_task_ended(built, served_catalog::publish);
    });
}

/// Build the catalog from `cache` under `data_dir`, hand the outcome to
/// `publish`, and log it: INFO `cooked_presync.catalog_ready` on success, WARN
/// `cooked_presync.catalog_unavailable` with the closed reason code on failure.
/// Nothing is logged when `publish` refuses (an outcome was already published;
/// `served_catalog::publish` WARNs for that).
fn build_and_publish(
    cache: &ResourceCache,
    data_dir: &str,
    publish: impl FnOnce(Published) -> bool,
) {
    let started = Instant::now();
    match ServedCatalog::build(cache, data_dir) {
        Ok(catalog) => {
            let (categories, body_bytes) = catalog_totals(&catalog);
            let duration_ms = started.elapsed().as_millis() as u64;
            if publish(Published::Ready(Arc::new(catalog))) {
                tracing::info!(
                    event = "cooked_presync.catalog_ready",
                    categories,
                    body_bytes,
                    duration_ms,
                    "the presync catalog is built and published"
                );
            }
        }
        Err(error) => {
            let reason = failure_code(&error);
            let category_id = category_named_by(&error);
            if publish(Published::Failed(reason)) {
                tracing::warn!(
                    event = "cooked_presync.catalog_unavailable",
                    reason,
                    category_id, // nt:id-only the one category that failed the build; reason names the failure
                    "the presync catalog build failed; launchers get catalog_failed"
                );
            }
        }
    }
}

/// The category count and total body bytes, read back from the index JSON.
fn catalog_totals(catalog: &ServedCatalog) -> (usize, u64) {
    let index: serde_json::Value =
        serde_json::from_slice(&catalog.index_json()).unwrap_or(serde_json::Value::Null);
    let rows: &[serde_json::Value] = index["categories"].as_array().map_or(&[], Vec::as_slice);
    let body_bytes = rows
        .iter()
        .filter_map(|row| row["body_bytes"].as_u64())
        .sum();
    (rows.len(), body_bytes)
}

/// The category id a build error names (`category <id>: <code>`), if any.
fn category_named_by(error: &str) -> Option<u32> {
    error
        .strip_prefix("category ")?
        .split(':')
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use tracing::Level;

    use super::*;
    use crate::test_support::LogCapture;

    const ITEMS: u32 = 4;
    const ITEMS_PAK: &str = "CookedDataItems.pak";
    const PLAIN_ITEM: u32 = 900_001;

    /// A plain item XML: no registered override, so the build patches nothing.
    fn item_xml(item_id: u32) -> Vec<u8> {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_ITEM ID=\"{item_id}\" Name=\"Item {item_id}\" Tier=\"1\">\
             <InventorySet IsDeletable=\"true\" MaxStackSize=\"1\" />\
             </COOKED_ITEM>"
        )
        .into_bytes()
    }

    /// Write one made-up items PAK into `dir` and load a cache over it.
    fn items_cache(dir: &Path) -> ResourceCache {
        let entries = BTreeMap::from([(PLAIN_ITEM, item_xml(PLAIN_ITEM))]);
        let file = std::fs::File::create(dir.join(ITEMS_PAK)).unwrap();
        cimmeria_cooked_pak::pak::write_pak(file, 7542, &entries).unwrap();
        ResourceCache::load_all(dir.to_str().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn a_built_catalog_is_published_and_logged() {
        let dir = tempfile::tempdir().unwrap();
        let cache = items_cache(dir.path());
        let capture = LogCapture::install();

        let mut published: Option<Published> = None;
        build_and_publish(&cache, dir.path().to_str().unwrap(), |state| {
            published = Some(state);
            true
        });

        let Some(Published::Ready(catalog)) = published else {
            panic!("a clean build must publish Ready");
        };
        assert!(
            catalog.category(ITEMS).is_some(),
            "the items category is served"
        );
        let ready = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "cooked_presync.catalog_ready"))
            .expect("catalog_ready must be logged");
        assert_eq!(ready.level, Level::INFO);
        assert!(ready.has_field("categories", "1"), "{ready:#?}");
    }

    #[tokio::test]
    async fn a_failed_build_publishes_failed_and_warns() {
        let dir = tempfile::tempdir().unwrap();
        let cache = items_cache(dir.path());
        // The cache loaded; now the raw PAK the build re-reads goes missing.
        std::fs::remove_file(dir.path().join(ITEMS_PAK)).unwrap();
        let capture = LogCapture::install();

        let mut published: Option<Published> = None;
        build_and_publish(&cache, dir.path().to_str().unwrap(), |state| {
            published = Some(state);
            true
        });

        assert!(
            matches!(published, Some(Published::Failed("raw_pak_unreadable"))),
            "a failed build must publish its reason code"
        );
        let warn = capture
            .find_event(
                Level::WARN,
                "presync catalog build failed",
                "raw_pak_unreadable",
            )
            .expect("catalog_unavailable must be logged at WARN");
        assert!(
            warn.has_field("category_id", &ITEMS.to_string()),
            "{warn:#?}"
        );
    }

    #[test]
    fn a_cache_that_failed_to_load_publishes_build_failed() {
        let capture = LogCapture::install();
        let mut published: Option<Published> = None;
        let returned = presync_after_cache_load(
            Err("simulated load failure".to_owned()),
            "unused",
            |_, _| panic!("must not spawn"),
            |state| {
                published = Some(state);
                true
            },
        );
        assert!(returned.is_none());
        assert!(matches!(published, Some(Published::Failed("build_failed"))));
        let unavailable = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "cooked_presync.catalog_unavailable"))
            .expect("catalog_unavailable must be logged");
        assert_eq!(unavailable.level, Level::WARN);
        assert!(
            unavailable.has_field("reason", "build_failed"),
            "{unavailable:#?}"
        );
        let load = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "cache_load_failed"))
            .expect("cache_load_failed must be logged");
        assert_eq!(load.level, Level::WARN);
    }

    #[test]
    fn a_loaded_cache_spawns_the_build_and_publishes_nothing_yet() {
        let dir = tempfile::tempdir().unwrap();
        let cache = items_cache(dir.path());
        let mut spawned: Option<(Arc<ResourceCache>, String)> = None;
        let returned = presync_after_cache_load(
            Ok(cache),
            "the-data-dir",
            |c, d| spawned = Some((c, d)),
            |_| panic!("must not publish"),
        );
        let Some(c) = returned else {
            panic!("a loaded cache is returned");
        };
        let (spawned_cache, spawned_dir) = spawned.expect("the build must be spawned");
        assert!(Arc::ptr_eq(&c, &spawned_cache));
        assert_eq!(spawned_dir, "the-data-dir");
    }

    fn simulated_build_panic() {
        panic!("simulated build panic");
    }

    #[tokio::test]
    async fn a_panicked_build_task_publishes_build_failed() {
        let ended = tokio::task::spawn_blocking(simulated_build_panic).await;
        assert!(ended.is_err());
        let capture = LogCapture::install();
        let mut published: Option<Published> = None;
        on_build_task_ended(ended, |state| {
            published = Some(state);
            true
        });
        assert!(matches!(published, Some(Published::Failed("build_failed"))));
        let warn = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "cooked_presync.catalog_unavailable"))
            .expect("catalog_unavailable must be logged");
        assert_eq!(warn.level, Level::WARN);
        assert!(warn.has_field("reason", "build_failed"), "{warn:#?}");
    }

    #[test]
    fn a_finished_build_task_publishes_nothing_more() {
        on_build_task_ended(Ok(()), |_| panic!("must not publish"));
    }

    #[tokio::test]
    async fn a_refused_publish_logs_no_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let cache = items_cache(dir.path());
        let capture = LogCapture::install();
        build_and_publish(&cache, dir.path().to_str().unwrap(), |_| false);
        assert!(
            !capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "cooked_presync.catalog_ready")),
            "a refused publish must not log catalog_ready"
        );

        std::fs::remove_file(dir.path().join(ITEMS_PAK)).unwrap();
        build_and_publish(&cache, dir.path().to_str().unwrap(), |_| false);
        assert!(
            !capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "cooked_presync.catalog_unavailable")),
            "a refused publish must not log catalog_unavailable"
        );
    }

    #[test]
    fn the_category_named_by_an_error_is_read_back() {
        assert_eq!(category_named_by("category 4: raw_pak_unreadable"), Some(4));
        assert_eq!(
            category_named_by("category 12: category_too_large (99 bytes)"),
            Some(12)
        );
        assert_eq!(category_named_by("something else went wrong"), None);
    }
}
