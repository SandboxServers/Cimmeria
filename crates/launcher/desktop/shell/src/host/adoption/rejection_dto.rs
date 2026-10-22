//! Webview projections of the engine's refusal record and index fallback, and the
//! closed progress item a label maps to (C16). Each conversion copies fields one
//! by one, so engine labels and paths never reach the webview.
use super::contract::ProgressItem;
use cimmeria_launcher_engine::{adoption, install::Progress as Observed};
use serde::Serialize;

/// The check that refused a step, shown beside the error code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdoptionRejection {
    pub check: String,
    pub entry: Option<String>,
    pub expected: Option<String>,
    pub found: Option<String>,
    pub http_status: Option<u16>,
}
impl From<&adoption::Rejection> for AdoptionRejection {
    fn from(rejection: &adoption::Rejection) -> Self {
        Self {
            check: rejection.check.to_owned(),
            entry: rejection.entry.clone(),
            expected: rejection.expected.clone(),
            found: rejection.found.clone(),
            http_status: rejection.http_status,
        }
    }
}
/// Why the review uses the signed seed instead of the published file index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexFallbackView {
    pub reason: String,
    pub check: Option<String>,
    pub http_status: Option<u16>,
    pub seed_bytes: u64,
}
impl From<&adoption::IndexFallback> for IndexFallbackView {
    fn from(fallback: &adoption::IndexFallback) -> Self {
        Self {
            reason: fallback.reason.to_owned(),
            check: fallback.check.map(str::to_owned),
            http_status: fallback.http_status,
            seed_bytes: fallback.seed_bytes,
        }
    }
}
/// The closed item an engine progress label names. Anything outside the table,
/// and copy-phase progress, is `None`.
pub(super) fn progress_item(observed: &Observed) -> Option<ProgressItem> {
    match observed {
        Observed::Downloading { label, .. } => match label.as_str() {
            "seed" => Some(ProgressItem::Seed),
            "file index" => Some(ProgressItem::FileIndex),
            label if label.starts_with("patch ") => Some(ProgressItem::Patch),
            _ => None,
        },
        Observed::Extracting { label, .. } => match label.as_str() {
            "File index" => Some(ProgressItem::IndexBuild),
            "Authenticated adoption reference" => Some(ProgressItem::Reference),
            "Adoption replay" => Some(ProgressItem::Patch),
            "Scanning" => Some(ProgressItem::Scan),
            _ => None,
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_items_map_the_engine_labels() {
        let downloading = |label: &str| Observed::Downloading {
            label: label.into(),
            downloaded: 1,
            total: 2,
        };
        let extracting = |label: &str| Observed::Extracting {
            label: label.into(),
            current: 1,
            total: 2,
            filename: "file".into(),
        };
        assert_eq!(
            progress_item(&downloading("seed")),
            Some(ProgressItem::Seed)
        );
        assert_eq!(
            progress_item(&downloading("patch 2024-01")),
            Some(ProgressItem::Patch)
        );
        assert_eq!(
            progress_item(&downloading("file index")),
            Some(ProgressItem::FileIndex)
        );
        assert_eq!(
            progress_item(&extracting("File index")),
            Some(ProgressItem::IndexBuild)
        );
        assert_eq!(
            progress_item(&extracting("Authenticated adoption reference")),
            Some(ProgressItem::Reference)
        );
        assert_eq!(
            progress_item(&extracting("Adoption replay")),
            Some(ProgressItem::Patch)
        );
        assert_eq!(
            progress_item(&extracting("Scanning")),
            Some(ProgressItem::Scan)
        );
        assert_eq!(progress_item(&extracting("Verified copy")), None);
        assert_eq!(progress_item(&downloading("something else")), None);
        assert_eq!(progress_item(&downloading("patchwork")), None);
    }

    #[test]
    fn a_rejection_and_a_fallback_convert_field_by_field() {
        let rejection = adoption::Rejection {
            check: "download_http_status",
            entry: Some("seed".into()),
            expected: Some("200".into()),
            found: Some("500".into()),
            http_status: Some(500),
        };
        assert_eq!(
            AdoptionRejection::from(&rejection),
            AdoptionRejection {
                check: "download_http_status".into(),
                entry: Some("seed".into()),
                expected: Some("200".into()),
                found: Some("500".into()),
                http_status: Some(500),
            }
        );
        let fallback = adoption::IndexFallback {
            reason: "index_not_published",
            check: Some("download_http_status"),
            http_status: Some(404),
            seed_bytes: 1_234,
        };
        assert_eq!(
            IndexFallbackView::from(&fallback),
            IndexFallbackView {
                reason: "index_not_published".into(),
                check: Some("download_http_status".into()),
                http_status: Some(404),
                seed_bytes: 1_234,
            }
        );
    }
}
