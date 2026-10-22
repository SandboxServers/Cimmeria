//! Compares an adopted folder with the published file index (LA-12). Only a
//! cancel aborts; a missing, unreachable or refused index becomes an
//! [`IndexState`], so a preview never blocks on it.
use super::*;
use super::{index_source, inventory, rejection};
use crate::install_progress::ProgressSink;
use cimmeria_launcher_core::file_index::{self, FileClass, FileIndex, IndexComparison};
use cimmeria_launcher_core::install_layout::bundled_cooked_alias;
use std::collections::BTreeMap;

/// The most paths one `adoption_index_compared` sample names.
const SAMPLE_PATHS: usize = 10;
/// The most characters one `adoption_index_compared` sample holds.
const SAMPLE_CHARS: usize = 600;

/// How the published index fared. The serde names are the event's `state`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexState {
    /// The index verified and the folder was compared with it.
    Compared,
    /// The manifest names no index.
    IndexNotPublished,
    /// The index could not be downloaded, or the verified copy could not be cached.
    IndexUnreachable,
    /// The index was downloaded but refused by a check.
    IndexInvalid,
}

impl IndexState {
    /// The serde name of this state, as the event carries it.
    pub fn code(self) -> &'static str {
        match self {
            Self::Compared => "compared",
            Self::IndexNotPublished => "index_not_published",
            Self::IndexUnreachable => "index_unreachable",
            Self::IndexInvalid => "index_invalid",
        }
    }
}

/// The outcome of [`compare_to_published`]. `comparison` is set only when `state`
/// is [`IndexState::Compared`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IndexReport {
    pub state: IndexState,
    pub comparison: Option<IndexComparison>,
}

impl IndexReport {
    fn without_comparison(state: IndexState) -> Self {
        Self {
            state,
            comparison: None,
        }
    }
}

/// Compares `scan` with the published index for `release`. The index comes from
/// the cache when it verifies, else it is downloaded. The caller owns cancellation
/// and progress, as with `acquire_index`. Blocking: call it from a retained worker.
/// Err is only ever `Error::Cancelled`; it emits no event.
pub(super) fn compare_to_published(
    state_root: &Path,
    release: &VerifiedRelease,
    transport: &artifacts::Transport,
    scan: &inventory::Index,
    cancel: &CancellationToken,
    progress: &ProgressSink,
) -> Result<IndexReport, Error> {
    let report = match &release.manifest().file_index {
        None => IndexReport::without_comparison(IndexState::IndexNotPublished),
        Some(reference) => {
            // The scope keeps a rejection recorded by the fetch from becoming the cause.
            let (outcome, _recorded) = rejection::scope(|| {
                index_source::published(state_root, release, transport, reference, cancel, progress)
            });
            match outcome {
                Ok(Ok((index, _cached))) => IndexReport {
                    state: IndexState::Compared,
                    comparison: Some(compare(&index, scan)),
                },
                Ok(Err(fallback)) if fallback.reason == "index_download_failed" => {
                    IndexReport::without_comparison(IndexState::IndexUnreachable)
                }
                Ok(Err(_)) => IndexReport::without_comparison(IndexState::IndexInvalid),
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(_) => IndexReport::without_comparison(IndexState::IndexUnreachable),
            }
        }
    };
    compared_event(&report);
    Ok(report)
}

/// The folder as `file_index::compare` takes it: each scanned path, with cooked
/// data under its bundled alias, mapped to its lower-case SHA-256.
fn compare(index: &FileIndex, scan: &inventory::Index) -> IndexComparison {
    let folder: BTreeMap<String, String> = scan
        .iter()
        .map(|(name, entry)| {
            let key = bundled_cooked_alias(name).unwrap_or_else(|| name.clone());
            let digest: String = entry.sha256.iter().map(|b| format!("{b:02x}")).collect();
            (key, digest)
        })
        .collect();
    file_index::compare(index, &folder)
}

/// The signed index was compared (or refused). Info: one per preview. The samples
/// name only index paths (`Changed` and `Missing`), never a file the index lacks.
fn compared_event(report: &IndexReport) {
    let count = |class: FileClass| {
        report
            .comparison
            .as_ref()
            .and_then(|c| c.counts.get(&class))
            .copied()
            .unwrap_or(0)
    };
    let (changed_sample, missing_sample) = match &report.comparison {
        Some(c) => (
            sample(&c.files, FileClass::Changed),
            sample(&c.files, FileClass::Missing),
        ),
        None => (String::new(), String::new()),
    };
    tracing::info!(
        target: "adoption",
        event = "adoption_index_compared",
        state = report.state.code(),
        stock = count(FileClass::Stock),
        patched = count(FileClass::Patched),
        prepared = count(FileClass::Prepared),
        changed = count(FileClass::Changed),
        missing = count(FileClass::Missing),
        extra = count(FileClass::Extra),
        changed_sample = changed_sample.as_str(),
        missing_sample = missing_sample.as_str(),
        "adoption: compared the folder with the published file index ({})",
        report.state.code()
    );
}

/// Up to [`SAMPLE_PATHS`] paths of `class`, in order, comma-joined. It stops before
/// the string would pass [`SAMPLE_CHARS`] characters.
fn sample(files: &[(String, FileClass)], class: FileClass) -> String {
    let mut out = String::new();
    for (path, _) in files
        .iter()
        .filter(|(_, found)| *found == class)
        .take(SAMPLE_PATHS)
    {
        let next = if out.is_empty() {
            path.clone()
        } else {
            format!("{out},{path}")
        };
        if next.chars().count() > SAMPLE_CHARS {
            break;
        }
        out = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::verification_tests::{
        origin, published_index, reference, release, seed_bytes,
    };
    use super::*;
    use crate::test_capture::capture;
    use sha2::{Digest, Sha256};
    use std::cell::{Cell, RefCell};
    use std::sync::{Mutex, MutexGuard};
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

    static SERIAL: Mutex<()> = Mutex::new(());
    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Compares `folder` with the index of `release`, blocking as the worker does.
    fn run(
        state: &Path,
        release: &VerifiedRelease,
        server: &MockServer,
        folder: &Path,
    ) -> Result<IndexReport, Error> {
        let transport =
            artifacts::Transport::loopback(format!("{}/manifest.json", server.uri())).unwrap();
        let scan = inventory::scan(folder, &CancellationToken::new()).unwrap();
        let progress = ProgressSink::latest().0;
        tokio::task::block_in_place(|| {
            compare_to_published(
                state,
                release,
                &transport,
                &scan,
                &CancellationToken::new(),
                &progress,
            )
        })
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_manifest_without_an_index_is_index_not_published() {
        let seed = seed_bytes();
        let signed = release(&seed, None);
        let server = MockServer::start().await;
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let report = run(state.path(), &signed, &server, folder.path()).unwrap();
        assert_eq!(report.state, IndexState::IndexNotPublished);
        assert!(report.comparison.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_404_for_the_index_is_index_unreachable() {
        let seed = seed_bytes();
        let index = published_index(&seed);
        let signed = release(&seed, Some(reference(&index, None)));
        let server = MockServer::start().await;
        Mock::given(path("/seed.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(seed.clone()))
            .mount(&server)
            .await;
        Mock::given(path("/index/file-index.json"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let report = run(state.path(), &signed, &server, folder.path()).unwrap();
        assert_eq!(report.state, IndexState::IndexUnreachable);
        assert!(report.comparison.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_flipped_index_digest_is_index_invalid() {
        let seed = seed_bytes();
        let index = published_index(&seed);
        let wrong = hex(&Sha256::digest(b"another index"));
        let signed = release(&seed, Some(reference(&index, Some(wrong))));
        let server = origin(&seed, &index).await;
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let report = run(state.path(), &signed, &server, folder.path()).unwrap();
        assert_eq!(report.state, IndexState::IndexInvalid);
        assert!(report.comparison.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_folder_is_compared_with_the_published_index() {
        let seed = seed_bytes();
        let index = published_index(&seed);
        let signed = release(&seed, Some(reference(&index, None)));
        let server = origin(&seed, &index).await;
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("later.txt"), b"changed entry").unwrap();
        std::fs::write(folder.path().join("my_screenshot_Steve.png"), b"png").unwrap();
        let report = run(state.path(), &signed, &server, folder.path()).unwrap();
        assert_eq!(report.state, IndexState::Compared);
        let counts = &report.comparison.as_ref().unwrap().counts;
        assert_eq!(counts[&FileClass::Changed], 1);
        assert_eq!(counts[&FileClass::Extra], 1);
        assert!(counts[&FileClass::Missing] >= 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_compared_event_names_only_index_paths() {
        let seed = seed_bytes();
        let index = published_index(&seed);
        let signed = release(&seed, Some(reference(&index, None)));
        let server = origin(&seed, &index).await;
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("later.txt"), b"changed entry").unwrap();
        std::fs::write(folder.path().join("my_screenshot_Steve.png"), b"png").unwrap();
        let _serial = serial(); // first line: one capture test of this module at a time
                                // A second live Dispatch keeps callsite interest per dispatcher, so a callsite
                                // that another module's test hit first is not left disabled.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let (_, logs) = capture(|| {
            tracing::callsite::rebuild_interest_cache(); // first line in the closure
            run(state.path(), &signed, &server, folder.path())
        });
        let rows: Vec<_> = logs
            .iter()
            .filter(|row| row.field("event") == Some("adoption_index_compared"))
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].target, "adoption");
        assert_eq!(rows[0].field("state"), Some("compared"));
        assert_eq!(rows[0].field("changed"), Some("1"));
        assert_eq!(rows[0].field("extra"), Some("1"));
        assert!(rows[0]
            .field("changed_sample")
            .unwrap()
            .contains("later.txt"));
        assert!(!rows[0].field("missing_sample").unwrap().is_empty());
        assert!(logs.iter().all(|row| row
            .fields
            .values()
            .all(|v| !v.contains("my_screenshot_Steve"))));
    }

    #[test]
    fn a_sample_holds_at_most_ten_paths_and_600_characters_of_one_class() {
        let mut files: Vec<(String, FileClass)> = (0..12)
            .map(|i| (format!("p{i:02}"), FileClass::Changed))
            .collect();
        files.insert(3, ("x".to_string(), FileClass::Extra));
        files.insert(7, ("x".to_string(), FileClass::Extra));
        files.push(("x".to_string(), FileClass::Extra));
        assert_eq!(
            sample(&files, FileClass::Changed),
            "p00,p01,p02,p03,p04,p05,p06,p07,p08,p09"
        );
        let long: Vec<(String, FileClass)> = (0..10)
            .map(|_| ("a".repeat(100), FileClass::Missing))
            .collect();
        let missing = sample(&long, FileClass::Missing);
        assert!(missing.chars().count() <= 600);
        assert_eq!(missing.split(',').count(), 5);
        assert_eq!(sample(&[], FileClass::Changed), "");
    }

    #[test]
    fn scan_with_progress_reports_each_hashed_file_and_matches_scan() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("d/e")).unwrap();
        std::fs::write(dir.path().join("a.bin"), [1u8; 3]).unwrap();
        std::fs::write(dir.path().join("d/b.bin"), [2u8; 5]).unwrap();
        std::fs::write(dir.path().join("d/e/c.bin"), [3u8; 7]).unwrap();
        let calls = RefCell::new(Vec::new());
        let cancel = CancellationToken::new();
        let found = inventory::scan_with_progress(dir.path(), &cancel, &|hashed, total| {
            calls.borrow_mut().push((hashed, total));
        })
        .unwrap();
        let calls = calls.into_inner();
        assert_eq!(calls.first(), Some(&(0, 15)));
        assert_eq!(calls.last(), Some(&(15, 15)));
        assert!(calls.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert_eq!(found, inventory::scan(dir.path(), &cancel).unwrap());
    }

    #[test]
    fn a_cancelled_progress_scan_reports_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.bin"), [1u8; 3]).unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let calls = Cell::new(0);
        let result = inventory::scan_with_progress(dir.path(), &cancel, &|_, _| {
            calls.set(calls.get() + 1);
        });
        assert!(matches!(result, Err(Error::Cancelled)));
        assert_eq!(calls.get(), 0);
    }
}
