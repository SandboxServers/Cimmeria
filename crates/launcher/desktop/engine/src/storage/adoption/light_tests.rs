//! Light adoption end to end (LA-11): a folder with no seed previews, publishes and
//! recovers, and the files it could not rebuild are recorded rather than refused.
use super::golden_tests::{hex, PLAN_BYTES, PLAN_DIGEST};
use super::in_place_tests::{commit, commit_with, Fixture};
use super::light_replay::tests::{
    big, patch_zip, release_with_index, release_with_roots, OpFixture, OVERLAY,
};
use super::*;
use crate::install_progress::ProgressSink;
use crate::storage::install_worker::content_valid;
use cimmeria_launcher_core::state::InstalledState;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

/// The PE stub `install_worker::fixtures::archive` ships as `SGW.exe`.
pub(super) fn exe_stub() -> Vec<u8> {
    let mut exe = vec![0u8; 0x200];
    exe[..2].copy_from_slice(b"MZ");
    exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
    exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
    exe
}

pub(super) fn write(root: &Path, rel: &str, bytes: &[u8]) {
    let path = inventory::native(root, rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// One patch that rebuilds `OVERLAY`.
fn overlay_patch() -> Vec<Vec<u8>> {
    vec![patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[],
    )]
}

/// A game folder, a state store and an origin that serves the patch archives.
pub(super) struct Light {
    pub(super) root: tempfile::TempDir,
    pub(super) state: Arc<Mutex<DesktopState>>,
    pub(super) folder: PathBuf,
    pub(super) release: VerifiedRelease,
    pub(super) server: MockServer,
}

impl Light {
    pub(super) async fn new(zips: Vec<Vec<u8>>, files: &[(&str, &[u8])]) -> Self {
        Self::with_release(release_with_roots(&zips, &[]), zips, files).await
    }
    pub(super) async fn with_release(
        release: VerifiedRelease,
        zips: Vec<Vec<u8>>,
        files: &[(&str, &[u8])],
    ) -> Self {
        let server = MockServer::start().await;
        for (i, zip) in zips.iter().enumerate() {
            Mock::given(matchers::path(format!("/p{i}.zip")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
                .mount(&server)
                .await;
        }
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("folder");
        write(&folder, "Working/Binaries/SGW.exe", &exe_stub());
        for (rel, bytes) in files {
            write(&folder, rel, bytes);
        }
        let state = DesktopState::open(&root.path().join("state")).unwrap();
        Self {
            folder: folder.canonicalize().unwrap(),
            state: Arc::new(Mutex::new(state)),
            root,
            release,
            server,
        }
    }
    pub(super) fn verified(&self) -> VerifiedRelease {
        crate::catalog::verify_release(self.release.evidence().0, self.release.evidence().1)
            .unwrap()
    }
    fn request(&self) -> PreviewRequest {
        let owner = self.state.lock().unwrap();
        PreviewRequest {
            source: SourceRequest::Folder {
                selected: self.folder.clone(),
            },
            mode: AdoptionMode::Light,
            destination: self.folder.clone(),
            operation_revision: owner.operations().snapshot().revision,
            preferences_revision: owner.preferences().revision,
            release: self.verified(),
            artifacts: None,
        }
    }
    fn transport(&self) -> artifacts::Transport {
        artifacts::Transport::loopback(format!("{}/manifest.json", self.server.uri())).unwrap()
    }
    pub(super) fn preview(&self) -> Result<Preview, Error> {
        self.preview_with(CancellationToken::new(), ProgressSink::latest().0)
    }
    fn preview_with(
        &self,
        cancel: CancellationToken,
        progress: ProgressSink,
    ) -> Result<Preview, Error> {
        let request = self.request();
        tokio::task::block_in_place(|| {
            preview_using(
                self.state.clone(),
                request,
                cancel,
                progress,
                Backend::Native,
                Some(self.transport()),
            )
        })
    }
    pub(super) fn game(&self) -> PathBuf {
        self.folder.join("game")
    }
    pub(super) fn ledger(&self) -> InstalledState {
        InstalledState::load(&self.game())
    }
    /// The overlay as the published tree holds it.
    fn overlay(&self) -> Vec<u8> {
        std::fs::read(inventory::native(&self.game(), OVERLAY)).unwrap()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stock_folder_publishes_as_a_light_adoption() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let preview = lt.preview().unwrap();
    assert!(preview.reference.is_none(), "no seed is read or kept");
    let summary = preview.light_summary().expect("a light preview has one");
    // The fixture release publishes no index, so there is nothing to compare.
    assert_eq!(
        summary.index_state,
        index_compare::IndexState::IndexNotPublished
    );
    assert_eq!(
        (
            summary.patches_total,
            summary.patches_current,
            summary.patches_pending,
            summary.skipped_total
        ),
        (1, 0, 1, 0)
    );
    let provenance = commit(preview).unwrap();
    assert_eq!(provenance.check, FileCheck::Light);
    assert!(content_valid(&lt.game(), &lt.verified()));
    assert_eq!(lt.overlay(), big("patched overlay"));
    let ledger = lt.ledger();
    assert_eq!(ledger.applied_patches.len(), 1);
    assert!(ledger.incomplete_patches.is_empty());
    // The stored provenance is read back and verified by the same code as before.
    assert!(lt
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_changed_source_publishes_with_an_incomplete_patch() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, b"modded by the player")]).await;
    let preview = lt.preview().expect("a changed input never blocks");
    assert_eq!(preview.light_summary().unwrap().skipped_total, 1);
    assert_eq!(
        preview.light_summary().unwrap().index_state,
        index_compare::IndexState::IndexNotPublished
    );
    let provenance = commit(preview).unwrap();
    assert_eq!(provenance.check, FileCheck::Light);
    assert!(content_valid(&lt.game(), &lt.verified()));
    assert_eq!(
        lt.overlay(),
        b"modded by the player",
        "the player's bytes stay"
    );
    let ledger = lt.ledger();
    let [incomplete] = ledger.incomplete_patches.as_slice() else {
        panic!("one incomplete patch: {:?}", ledger.incomplete_patches);
    };
    assert!(ledger.applied_patches.contains(&incomplete.state_key));
    assert_eq!(incomplete.skipped.len(), 1);
    assert_eq!(incomplete.skipped[0].path, OVERLAY);
    assert_eq!(incomplete.skipped[0].reason, "source_mismatch");
}

/// A crash mid-move (the in-place `Staged` phase) and one just before promotion both
/// recover to the same published light install, and the incomplete-patch list
/// written at admission survives.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_light_publication_recovers_after_staged() {
    for crash in [
        publication::Point::Moved,
        publication::Point::BeforePromotion,
    ] {
        let lt = Light::new(overlay_patch(), &[(OVERLAY, b"modded by the player")]).await;
        let failed = commit_with(lt.preview().unwrap(), |point| {
            if point == crash {
                Err(StorageError::Io.into())
            } else {
                Ok(())
            }
        });
        assert!(failed.is_err(), "{crash:?}");
        let state_dir = lt.root.path().join("state");
        let game = lt.game();
        let release = lt.verified();
        let id = lt
            .state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .clone()
            .unwrap()
            .id;
        drop(lt.state);
        let mut reopened = DesktopState::open(&state_dir).unwrap();
        let revision = reopened.operations().snapshot().revision;
        recover(&mut reopened, id, revision).unwrap();
        let operation = reopened.operations().snapshot().operation.clone().unwrap();
        assert_eq!(operation.state, OperationState::Succeeded, "{crash:?}");
        assert!(reopened.installed_content().unwrap().is_some());
        assert!(content_valid(&game, &release), "{crash:?}");
        let ledger = InstalledState::load(&game);
        assert_eq!(ledger.incomplete_patches.len(), 1, "{crash:?}");
    }
}

/// R-LA 8 and 10: a record written before the check existed reads as a full check,
/// writes back byte for byte, and hashes the digest it always did.
#[test]
fn a_pre_la_plan_reads_as_a_full_check_with_the_same_digest() {
    let plan: Plan = serde_json::from_slice(PLAN_BYTES).unwrap();
    assert!(!String::from_utf8_lossy(PLAN_BYTES).contains("\"check\""));
    assert_eq!(plan.provenance.check, FileCheck::Full);
    assert!(plan.incomplete_patches.is_empty());
    assert_eq!(serde_json::to_vec(&plan).unwrap(), PLAN_BYTES);
    assert_eq!(hex(&plan.digest().unwrap()), PLAN_DIGEST);
    let mut light = plan.clone();
    light.provenance.check = FileCheck::Light;
    assert_ne!(light.digest().unwrap(), plan.digest().unwrap());
    let written = serde_json::to_value(&light.provenance).unwrap();
    assert_eq!(written["check"], "light");
}

/// R-LA 10: the full-check in-place path publishes as before. Its provenance
/// carries no `check` key, its ledger no `incomplete_patches`, and it verifies.
#[test]
fn a_full_check_adoption_is_unchanged() {
    let f = Fixture::new(None);
    let provenance = commit(f.preview().unwrap()).unwrap();
    assert_eq!(provenance.check, FileCheck::Full);
    let written = serde_json::to_value(&provenance).unwrap();
    assert!(written.get("check").is_none());
    let ledger = serde_json::to_value(super::reference::ledger(&f.release, &[])).unwrap();
    assert!(ledger.get("incomplete_patches").is_none());
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_some());
}

/// With nothing in the folder to reuse the adoption is refused, with a closed code.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_scan_with_nothing_to_reuse_is_refused() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let request = lt.request();
    let ownership = preparation::Ownership::claim(
        lt.state.clone(),
        &request,
        crate::storage::install_intent::ExtractionBackend::Native,
        Vec::new(),
    )
    .unwrap();
    let (result, recorded) = rejection::scope(|| {
        light::light_comparison(
            &lt.state,
            &request,
            None,
            &ownership,
            &lt.folder,
            &inventory::Index::new(),
            &[],
            &CancellationToken::new(),
            ProgressSink::latest().0,
        )
    });
    assert!(matches!(result, Err(Error::NoReusableFiles)));
    let recorded = recorded.expect("the refusal is recorded");
    assert_eq!(recorded.check, "no_reusable_files");
    assert_eq!(recorded.found.as_deref(), Some("0 of 0"));
}

/// The review keeps `LIGHT_LIST_CAP` skipped targets and the true total beside it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_summary_caps_the_skipped_list_and_keeps_the_total() {
    let count = LIGHT_LIST_CAP + 3;
    let targets: Vec<String> = (0..count)
        .map(|i| format!("Working/Data/F{i}.dat"))
        .collect();
    let stock: Vec<Vec<u8>> = (0..count).map(|i| big(&format!("stock {i}"))).collect();
    let patched = big("patched");
    let sources: Vec<[(&str, &[u8]); 1]> = targets
        .iter()
        .zip(&stock)
        .map(|(t, s)| [(t.as_str(), s.as_slice())])
        .collect();
    let ops: Vec<OpFixture> = targets
        .iter()
        .zip(&sources)
        .map(|(target, sources)| OpFixture {
            target,
            sources,
            result: &patched,
        })
        .collect();
    let zips = vec![patch_zip("p0", &ops, &[])];
    let modded: Vec<(&str, &[u8])> = targets
        .iter()
        .map(|t| (t.as_str(), b"modded".as_slice()))
        .collect();
    let lt = Light::new(zips, &modded).await;
    let preview = lt.preview().unwrap();
    let summary = preview.light_summary().unwrap();
    assert_eq!(summary.skipped_total as usize, count);
    assert_eq!(summary.skipped.len(), LIGHT_LIST_CAP);
}

/// D-LA9: a published index the origin does not serve never blocks the preview. It
/// is reported as unreachable, compares nothing and is not recorded in the ledger.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreachable_index_never_blocks_and_is_not_recorded() {
    let zips = overlay_patch();
    // Nothing is mounted at the index path, so the origin answers 404.
    let index = serde_json::json!({
        "blob": "index/file-index.json",
        "size": 10,
        "sha256": "ab".repeat(32),
    });
    let release = release_with_index(&zips, &[], Some(index));
    let lt = Light::with_release(release, zips, &[(OVERLAY, &big("stock overlay"))]).await;
    let preview = lt.preview().expect("an unreachable index never blocks");
    let summary = preview.light_summary().unwrap();
    assert_eq!(
        summary.index_state,
        index_compare::IndexState::IndexUnreachable
    );
    assert!(summary.index_counts.is_empty());
    assert_eq!(summary.index_files_total, 0);
    assert_eq!(preview.file_index_sha256, None);
    let provenance = commit(preview).unwrap();
    assert_eq!(provenance.file_index_sha256, None);
}

/// A cooked file held under the stock spelling that a patch ships under the placed
/// spelling is stored as a move: the row names both paths and the copy is kept. A
/// folder with only the stock tier is placed by a directory rename instead (LA-11b),
/// so this folder also holds a placed file and keeps both tiers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stock_named_cooked_file_is_stored_as_a_move() {
    const STOCK: &str = "Working/SGWGame/Cache.en-US/Cooked.pak";
    const PLACED: &str = "Working/SGWGame/SourceCache.en-us/Cooked.pak";
    let zips = vec![patch_zip(
        "p0",
        &[OpFixture {
            target: OVERLAY,
            sources: &[(OVERLAY, &big("stock overlay"))],
            result: &big("patched overlay"),
        }],
        &[(PLACED, b"cooked")],
    )];
    let lt = Light::new(
        zips,
        &[
            (OVERLAY, &big("stock overlay")),
            (STOCK, b"cooked"),
            ("Working/SGWGame/SourceCache.en-us/Other.pak", b"other"),
        ],
    )
    .await;
    let preview = lt.preview().unwrap();
    let row = preview
        .report()
        .files
        .iter()
        .find(|d| d.path == PLACED)
        .expect("the placed path has a row");
    assert_eq!(row.classification, Classification::NeedsPatching);
    assert_eq!(row.source_path.as_deref(), Some(STOCK));
    let store = preview.preparation.record.directory.join("replacements");
    assert_eq!(
        std::fs::read(inventory::native(&store, PLACED)).unwrap(),
        b"cooked"
    );
    assert!(preview.replacements().iter().any(|r| r.path == PLACED));
    // Both tiers hold files, so the publish places nothing and stores the move.
    commit(preview).unwrap();
    let game = lt.game();
    assert_eq!(
        std::fs::read(inventory::native(&game, PLACED)).unwrap(),
        b"cooked"
    );
    assert_eq!(
        std::fs::read(inventory::native(
            &game,
            "Working/SGWGame/SourceCache.en-us/Other.pak"
        ))
        .unwrap(),
        b"other"
    );
}

/// A recorder for the progress a preview reports.
struct Seen(Mutex<Vec<crate::install::Progress>>);

impl crate::install_progress::ProgressReporter for Seen {
    fn report(&self, value: crate::install::Progress) {
        self.0.lock().unwrap().push(value);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_scan_reports_progress() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let seen = Arc::new(Seen(Mutex::new(Vec::new())));
    lt.preview_with(
        CancellationToken::new(),
        ProgressSink::Checkpoint(seen.clone()),
    )
    .unwrap();
    let seen = seen.0.lock().unwrap();
    let scanning: Vec<_> = seen
        .iter()
        .filter_map(|p| match p {
            crate::install::Progress::Extracting {
                label,
                current,
                total,
                ..
            } if label == "Scanning" => Some((*current, *total)),
            _ => None,
        })
        .collect();
    assert!(!scanning.is_empty(), "the scan reports");
    assert!(scanning.iter().all(|(current, total)| current <= total));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancelled_scan_is_cancelled_and_leaves_no_probe() {
    let lt = Light::new(overlay_patch(), &[(OVERLAY, &big("stock overlay"))]).await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let seen = Arc::new(Seen(Mutex::new(Vec::new())));
    let result = lt.preview_with(cancel, ProgressSink::Checkpoint(seen.clone()));
    assert!(matches!(result, Err(Error::Cancelled)));
    // The scan stops before it hashes anything, so it never reports.
    assert!(seen.0.lock().unwrap().is_empty());
    let mut stack = vec![lt.root.path().join("state")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            assert_ne!(entry.file_name(), "probe", "{:?}", entry.path());
            if entry.path().is_dir() {
                stack.push(entry.path());
            }
        }
    }
}
