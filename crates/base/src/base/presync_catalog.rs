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

/// The reason code for a build task that panicked or was cancelled. It is one
/// of the closed codes [`failure_code`] returns.
const BUILD_TASK_FAILED: &str = "build_failed";

/// Build the catalog for `cache` on a blocking thread and publish the outcome.
/// Returns at once: the caller never waits on the build.
pub(super) fn spawn_presync_catalog(cache: Arc<ResourceCache>, data_dir: String) {
    tokio::spawn(async move {
        let built = tokio::task::spawn_blocking(move || {
            build_and_publish(&cache, &data_dir, served_catalog::publish);
        })
        .await;
        if let Err(join_error) = built {
            served_catalog::publish(Published::Failed(BUILD_TASK_FAILED));
            tracing::warn!(
                event = "cooked_presync.catalog_unavailable",
                reason = BUILD_TASK_FAILED,
                error = %join_error,
                "the presync catalog build task ended without a result"
            );
        }
    });
}

/// Build the catalog from `cache` under `data_dir`, hand the outcome to
/// `publish`, and log it: INFO `cooked_presync.catalog_ready` on success, WARN
/// `cooked_presync.catalog_unavailable` with the closed reason code on failure.
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
            publish(Published::Ready(Arc::new(catalog)));
            tracing::info!(
                event = "cooked_presync.catalog_ready",
                categories,
                body_bytes,
                duration_ms,
                "the presync catalog is built and published"
            );
        }
        Err(error) => {
            let reason = failure_code(&error);
            let category_id = category_named_by(&error);
            publish(Published::Failed(reason));
            tracing::warn!(
                event = "cooked_presync.catalog_unavailable",
                reason,
                category_id,
                "the presync catalog build failed; launchers get catalog_failed"
            );
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
    fn the_category_named_by_an_error_is_read_back() {
        assert_eq!(category_named_by("category 4: raw_pak_unreadable"), Some(4));
        assert_eq!(
            category_named_by("category 12: category_too_large (99 bytes)"),
            Some(12)
        );
        assert_eq!(category_named_by("something else went wrong"), None);
    }
}
