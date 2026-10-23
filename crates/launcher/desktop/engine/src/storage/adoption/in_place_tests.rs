//! In-place adoption against a real imported old-launcher folder (D-LX12).
use super::*;
use crate::install_worker::fixtures;
use serde_json::json;

const SCREENSHOT: &str = "Working/Binaries/screenshot.bmp";
const NOTES: &str = "notes.txt";

/// The container format of the seed the fixture's release is built from.
#[derive(Clone, Copy)]
pub(super) enum SeedKind {
    Zip,
    Rar,
}

pub(super) struct Fixture {
    root: tempfile::TempDir,
    pub(super) state: Arc<Mutex<DesktopState>>,
    source: migration::LegacySource,
    seed: PathBuf,
    pub(super) release: VerifiedRelease,
}
impl Fixture {
    /// An old-launcher install of the signed release, with one file of the
    /// player's own beside the release and one inside a release folder.
    /// `ledger_seed` is what the old launcher's ledger claims was installed.
    pub(super) fn new(ledger_seed: Option<&str>) -> Self {
        Self::with_seed(ledger_seed, SeedKind::Zip)
    }
    /// As [`Self::new`], with the seed in the given container format. Both
    /// kinds carry the same entries.
    pub(super) fn with_seed(ledger_seed: Option<&str>, kind: SeedKind) -> Self {
        Self::with_archive(ledger_seed, kind, fixtures::archive(true))
    }
    /// As [`Self::with_seed`], with the release built from `archive`, a zip with
    /// the entries `fixtures::archive` writes.
    pub(super) fn with_archive(
        ledger_seed: Option<&str>,
        kind: SeedKind,
        archive: Vec<u8>,
    ) -> Self {
        let root = tempfile::tempdir().unwrap();
        let launcher = root.path().join("legacy");
        let game = root.path().join("old-game");
        std::fs::create_dir(&launcher).unwrap();
        std::fs::create_dir(&game).unwrap();
        let seed = match kind {
            SeedKind::Zip => root.path().join("seed.zip"),
            SeedKind::Rar => root.path().join("seed.rar"),
        };
        let bytes = match kind {
            SeedKind::Zip => archive,
            SeedKind::Rar => rar_seed(&archive, &seed),
        };
        std::fs::write(&seed, &bytes).unwrap();
        let release = fixtures::verified(&bytes);
        let (progress, _) = crate::install_progress::ProgressSink::latest();
        crate::unpack::unpack(
            &seed,
            &game,
            &crate::unpack::UnpackSink {
                progress,
                label: "fixture".into(),
                cancel: CancellationToken::new(),
            },
        )
        .unwrap();
        let servers = [("Second", "second"), ("First", "first")].map(|(name, host)| {
            crate::client_setup::LoginServer {
                name: name.into(),
                url: format!("https://example.invalid/{host}"),
            }
        });
        crate::client_setup::prepare(&game, &servers).unwrap();
        std::fs::write(game.join(NOTES), b"the player's notes").unwrap();
        std::fs::write(inventory::native(&game, SCREENSHOT), b"BM screenshot").unwrap();
        let seed_sha = release.manifest().seed.sha256.clone();
        std::fs::write(
            game.join("launcher-installed.json"),
            serde_json::to_vec(&json!({
                "seed_sha256": ledger_seed.unwrap_or(&seed_sha),
                "seed_adopted": true,
                "applied_patches": []
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            launcher.join("launcher-config.json"),
            serde_json::to_vec_pretty(&json!({
                "install_path": game,
                "manifest_url": crate::catalog::URL,
                "login_servers": [
                    {"name": "Second", "url": "https://example.invalid/second"},
                    {"name": "First", "url": "https://example.invalid/first"}
                ],
                "client_patches": {"enabled": true}
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(launcher.join("install.json"), br#"{"schema_version":1,"install_id":"72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d","machine_id":"fixture-machine","first_seen_ms":1,"created_by_launcher_version":"0.8.4"}"#).unwrap();
        let source = migration::LegacySource {
            launcher_directory: launcher.canonicalize().unwrap(),
            game_directory: game.canonicalize().unwrap(),
        };
        let mut state = DesktopState::open(&root.path().join("state")).unwrap();
        let import = state.preview_legacy_import(&source).unwrap();
        state
            .import_legacy(&source, &import.confirmation, 0)
            .unwrap();
        Self {
            root,
            state: Arc::new(Mutex::new(state)),
            source,
            seed,
            release,
        }
    }
    pub(super) fn folder(&self) -> &Path {
        &self.source.game_directory
    }
    pub(super) fn request(&self) -> PreviewRequest {
        let state = self.state.lock().unwrap();
        PreviewRequest {
            source: SourceRequest::Legacy {
                import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            },
            mode: AdoptionMode::InPlace,
            destination: self.folder().to_path_buf(),
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: state.preferences().revision,
            release: crate::catalog::verify_release(
                self.release.evidence().0,
                self.release.evidence().1,
            )
            .unwrap(),
            artifacts: Some(Artifacts {
                seed: self.seed.clone(),
                patches: vec![],
            }),
        }
    }
    pub(super) fn preview(&self) -> Result<Preview, Error> {
        preview(
            self.state.clone(),
            self.request(),
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0,
        )
    }
    pub(super) fn scan(&self) -> inventory::Index {
        inventory::scan(self.folder(), &CancellationToken::new()).unwrap()
    }
    /// A launcher restart: the same store, opened again.
    pub(super) fn reopen(self) -> Self {
        let Self {
            root,
            state,
            source,
            seed,
            release,
        } = self;
        drop(state);
        let state = DesktopState::open(&root.path().join("state")).unwrap();
        Self {
            root,
            state: Arc::new(Mutex::new(state)),
            source,
            seed,
            release,
        }
    }
    pub(super) fn operation(&self) -> (Uuid, OperationState) {
        let state = self.state.lock().unwrap();
        let op = state.operations().snapshot().operation.clone().unwrap();
        (op.id, op.state)
    }
}
/// A stock-shaped in-place install whose release publishes the file index: the
/// preview reads only the index, makes no seed request and keeps no reference.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_published_index_previews_in_place_without_the_seed() {
    use super::verification_tests::{published_index, reference, release};
    let mut f = Fixture::new(None);
    let seed = std::fs::read(&f.seed).unwrap();
    let index = published_index(&seed);
    f.release = release(&seed, Some(reference(&index, None)));
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::path("/index/file-index.json"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(index.clone()))
        .mount(&server)
        .await;
    let transport =
        artifacts::Transport::loopback(format!("{}/manifest.json", server.uri())).unwrap();
    let request = f.request();
    let preview = tokio::task::block_in_place(|| {
        preview_using(
            f.state.clone(),
            request,
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0,
            Backend::Native,
            Some(transport),
        )
    })
    .unwrap();
    assert!(preview.reference.is_none());
    commit(preview).unwrap();
    let seen: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.url.path().to_string())
        .collect();
    assert_eq!(seen, vec!["/index/file-index.json".to_string()]);
}

fn choices() -> Choices {
    Choices {
        normalize_managed_files: true,
        accept_unavailable_game_telemetry: true,
        old_game_closed: true,
        replace_modified_files: false,
    }
}
pub(super) fn commit(preview: Preview) -> Result<Provenance, Error> {
    commit_with(preview, |_| Ok(()))
}
pub(super) fn commit_with(
    preview: Preview,
    hook: impl FnMut(publication::Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    let handle = preview.report.preview_handle;
    publication::confirm(
        preview,
        Uuid::new_v4(),
        handle,
        choices(),
        CancellationToken::new(),
        &crate::install_progress::ProgressSink::latest().0,
        hook,
    )
}
/// Fails at the first moved file: the folder is left half in the owned tree.
pub(super) fn crash_after_first_move(point: publication::Point) -> Result<(), Error> {
    if point == publication::Point::Moved {
        Err(StorageError::Io.into())
    } else {
        Ok(())
    }
}
pub(super) fn assert_player_files_untouched(f: &Fixture) {
    assert_eq!(
        std::fs::read(f.folder().join(NOTES)).unwrap(),
        b"the player's notes"
    );
    assert_eq!(
        std::fs::read(inventory::native(f.folder(), SCREENSHOT)).unwrap(),
        b"BM screenshot"
    );
}

/// The zip seed's entries as a stored RAR4 with the same names and bytes,
/// written to `path`. Returns the archive's bytes.
fn rar_seed(zip_bytes: &[u8], path: &Path) -> Vec<u8> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        // RAR names use `\` on Windows, as the stock client's archive does.
        let name = entry.name().unwrap().replace('/', "\\");
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        entries.push((name, data));
    }
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice()))
        .collect();
    crate::unpack::test_fixtures::write_stored_rar4(path, &refs);
    std::fs::read(path).unwrap()
}

fn adopts_in_place_from(kind: SeedKind) {
    let f = Fixture::with_seed(None, kind);
    let originals = f.scan();
    let exe = inventory::native(f.folder(), "Working/Binaries/SGW.exe");
    let exe_identity = inventory::identity(&exe).unwrap();
    let preview = f.preview().unwrap();
    assert!(preview.report.in_place());
    assert_eq!(preview.report.blocking(preview.replacements()).count(), 0);
    let _expected = preview.reference.as_ref().unwrap().index.clone();
    let provenance = commit(preview).unwrap();

    assert!(!f.folder().join("game").exists());
    for (path, before) in &originals {
        if path == "launcher-installed.json" {
            continue;
        }
        let after = inventory::identity(&inventory::native(f.folder(), path)).unwrap();
        assert_eq!(
            (before.identity.device, before.identity.inode),
            (after.device, after.inode),
            "{path} was copied or relocated"
        );
    }

    let game = f.folder().to_path_buf();
    let moved = inventory::native(&game, "Working/Binaries/SGW.exe");
    // The same file at the same original path: neither renamed nor copied.
    let identity = inventory::identity(&moved).unwrap();
    assert_eq!(
        (identity.device, identity.inode),
        (exe_identity.device, exe_identity.inode)
    );
    assert!(exe.exists());
    assert_eq!(
        inventory::content_digest(
            &super::direct::scan(
                &game,
                &inspect(&f.state.lock().unwrap(), provenance.work_id)
                    .unwrap()
                    .plan
            )
            .unwrap()
        )
        .unwrap(),
        inventory::content_digest(&_expected).unwrap()
    );
    assert_player_files_untouched(&f);
    // The old launcher's ledger is not a release file and stays; the owned
    // tree carries a synthesized, verified one.
    assert!(f.folder().join("launcher-installed.json").is_file());
    let ledger: crate::state::InstalledState =
        serde_json::from_slice(&std::fs::read(game.join("launcher-installed.json")).unwrap())
            .unwrap();
    assert!(!ledger.seed_adopted);
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_eq!(installed.intent.destination, f.folder());
    assert!(state.adopted_in_place(&installed.intent).unwrap());
    assert_eq!(
        state.preferences().install_directory.as_deref(),
        Some(f.folder())
    );
    drop(state);
    assert_eq!(f.operation().1, OperationState::Succeeded);
}

#[test]
fn adoption_preserves_original_paths_file_ids_and_unlisted_files() {
    adopts_in_place_from(SeedKind::Zip);
}

// The production seed is a RAR. Native reference derivation used to demand a
// zip, so this refused the real seed on Windows with UnsupportedArchive.
#[test]
fn adopts_the_folder_in_place_from_a_rar_seed() {
    adopts_in_place_from(SeedKind::Rar);
}

#[test]
fn public_adoption_refuses_a_separate_copy_and_leaves_the_source_untouched() {
    let f = Fixture::new(None);
    let before = f.scan();
    let mut request = f.request();
    request.mode = AdoptionMode::Copy;
    request.destination = f.folder().parent().unwrap().join("separate-copy");
    assert_eq!(
        super::preview(
            f.state.clone(),
            request,
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0
        )
        .err(),
        Some(Error::UnsupportedMode)
    );
    assert_eq!(f.scan(), before);
    assert!(!f.folder().parent().unwrap().join("separate-copy").exists());
}

#[test]
fn a_modified_release_file_is_replaced_only_with_consent_and_changes_nothing() {
    let f = Fixture::new(None);
    std::fs::write(f.folder().join("later.txt"), b"edited by the player").unwrap();
    let before = f.scan();
    let preview = f.preview().unwrap();
    // The replay now produces the release copy, so the file no longer blocks; it
    // needs replace consent instead.
    assert!(preview
        .report
        .blocking(preview.replacements())
        .next()
        .is_none());
    assert_eq!(
        preview
            .replacements()
            .iter()
            .map(|r| (r.path.clone(), r.reason))
            .collect::<Vec<_>>(),
        vec![("later.txt".to_string(), Classification::Modified)]
    );
    assert_eq!(preview.consent(&choices()), Err(Error::ConsentRequired));
    assert_eq!(commit(preview), Err(Error::ConsentRequired));
    assert_eq!(f.scan(), before);
    for name in ["game", ".cimmeria-install.json", "content-ready.json"] {
        assert!(!f.folder().join(name).exists(), "{name} was created");
    }
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_none());
}

#[test]
fn a_modified_file_without_a_replacement_names_the_blocking_file() {
    let f = Fixture::new(None);
    std::fs::write(f.folder().join("later.txt"), b"edited by the player").unwrap();
    let mut preview = f.preview().unwrap();
    preview.replacements.clear();
    let (result, record) = rejection::scope(|| preview.consent(&choices()));
    assert_eq!(result, Err(Error::VerificationFailed));
    let record = record.unwrap();
    assert_eq!(record.check, "blocking_files");
    assert_eq!(record.entry.as_deref(), Some("later.txt"));
    assert_eq!(record.found.as_deref(), Some("1"));
}

#[test]
fn a_leftover_content_ready_file_is_named_in_the_refusal() {
    let f = Fixture::new(None);
    std::fs::write(f.folder().join("content-ready.json"), b"{}").unwrap();
    let (result, record) = rejection::scope(|| f.preview());
    assert_eq!(result.err(), Some(Error::LeftoverLauncherFiles));
    let record = record.unwrap();
    assert_eq!(record.check, "leftover_launcher_files");
    assert_eq!(record.entry.as_deref(), Some("content-ready.json"));
    assert_eq!(record.found.as_deref(), Some("content_ready"));
}

#[test]
fn an_old_launcher_ledger_for_another_release_is_classified_not_refused() {
    let f = Fixture::new(Some("another-seed"));
    let before = f.scan();
    let preview = f.preview().unwrap();
    // The ledger is not a gate: the files are classified against the release
    // itself. This folder holds the seed's own bytes, so nothing blocks.
    assert_eq!(preview.report.blocking(preview.replacements()).count(), 0);
    assert_eq!(f.scan(), before);
}

#[test]
fn uninstalling_an_adopted_folder_removes_only_what_the_launcher_manages() {
    let f = Fixture::new(None);
    commit(f.preview().unwrap()).unwrap();
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(
            Uuid::new_v4(),
            revision,
            installed.intent.operation_id,
            true,
        )
        .unwrap();
    assert!(state.installed_content().unwrap().is_none());
    drop(state);
    for name in ["game", ".cimmeria-install.json", "content-ready.json"] {
        assert!(!f.folder().join(name).exists(), "{name} was left");
    }
    assert_player_files_untouched(&f);
    assert!(!f.folder().join("launcher-installed.json").exists());
}

#[test]
fn an_interrupted_move_is_recovered_after_restart() {
    let f = Fixture::new(None);
    let preview = f.preview().unwrap();
    let expected = preview.expected_digest;
    assert!(commit_with(preview, crash_after_first_move).is_err());
    let f = f.reopen();
    let (id, phase) = f.operation();
    assert_eq!(phase, OperationState::ReconciliationRequired);
    let mut state = f.state.lock().unwrap();
    let interrupted = interrupted(&state, id).unwrap();
    assert!(interrupted.recoverable && interrupted.abandonable);
    let revision = state.operations().snapshot().revision;
    recover(&mut state, id, revision).unwrap();
    assert!(state.installed_content().unwrap().is_some());
    drop(state);
    let game = f.folder().to_path_buf();
    assert_eq!(
        inventory::content_digest(
            &super::direct::scan(&game, &inspect(&f.state.lock().unwrap(), id).unwrap().plan)
                .unwrap()
        )
        .unwrap(),
        expected
    );
    assert!(inventory::native(f.folder(), "Working/Binaries/SGW.exe").exists());
    assert_player_files_untouched(&f);
    assert_eq!(f.operation().1, OperationState::Succeeded);
}

#[test]
fn an_interrupted_move_can_be_abandoned_and_puts_every_file_back() {
    let f = Fixture::new(None);
    let before = f.scan();
    assert!(commit_with(f.preview().unwrap(), crash_after_first_move).is_err());
    let f = f.reopen();
    let (id, _) = f.operation();
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    abandon(&mut state, id, revision).unwrap();
    assert!(state.installed_content().unwrap().is_none());
    drop(state);
    assert_eq!(f.operation().1, OperationState::Cancelled);
    assert_eq!(f.scan(), before);
    for name in ["game", ".cimmeria-install.json"] {
        assert!(!f.folder().join(name).exists(), "{name} was left");
    }
}

/// Verification happens in the preview, before any marker exists. A launcher
/// that dies there leaves only its own reference work, which is removed
/// explicitly; the folder is untouched and adopting again succeeds.
#[test]
fn a_crash_between_verification_and_the_marker_leaves_the_folder_untouched() {
    let f = Fixture::new(None);
    let before = f.scan();
    let preparation = test_support::interrupted_preparation(f.state.clone(), &f.request()).unwrap();
    let f = f.reopen();
    assert_eq!(
        f.operation(),
        (preparation, OperationState::ReconciliationRequired)
    );
    assert_eq!(f.scan(), before);
    assert!(!f.folder().join(".cimmeria-install.json").exists());
    {
        let mut state = f.state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        abandon_preparation(&mut state, preparation, revision).unwrap();
    }
    commit(f.preview().unwrap()).unwrap();
    assert!(f
        .state
        .lock()
        .unwrap()
        .installed_content()
        .unwrap()
        .is_some());
}

/// Stores `bytes` as the replacement for `path` in the preparation store, as the
/// replay does, and returns the entry the review would hold for it.
pub(super) fn stored_replacement(preview: &Preview, path: &str, bytes: &[u8]) -> Replacement {
    let dest = inventory::native(
        &preview.preparation.record.directory.join("replacements"),
        path,
    );
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, bytes).unwrap();
    let entry = inventory::hash(&dest, &CancellationToken::new()).unwrap();
    Replacement {
        path: path.to_string(),
        size: entry.size,
        sha256: entry.sha256,
        reason: Classification::Modified,
    }
}

/// A folder whose `later.txt` the player edited, with the release bytes stored as
/// its replacement. Returns the release bytes too.
fn modified_with_replacement() -> (Fixture, Preview, Vec<u8>) {
    let f = Fixture::new(None);
    let release = std::fs::read(f.folder().join("later.txt")).unwrap();
    std::fs::write(f.folder().join("later.txt"), b"edited by the player").unwrap();
    let mut preview = f.preview().unwrap();
    preview.replacements = vec![stored_replacement(&preview, "later.txt", &release)];
    (f, preview, release)
}

/// Confirms with `replace_modified_files` set; the other choices are as `choices()`.
pub(super) fn confirm_replacing(
    preview: Preview,
    hook: impl FnMut(publication::Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    let handle = preview.report.preview_handle;
    let choices = Choices {
        replace_modified_files: true,
        ..choices()
    };
    publication::confirm(
        preview,
        Uuid::new_v4(),
        handle,
        choices,
        CancellationToken::new(),
        &crate::install_progress::ProgressSink::latest().0,
        hook,
    )
}

#[test]
fn a_replaced_file_needs_replace_modified_files_consent_and_moves_nothing() {
    let (f, preview, _) = modified_with_replacement();
    let before = f.scan();
    assert_eq!(preview.consent(&choices()), Err(Error::ConsentRequired));
    assert_eq!(commit(preview), Err(Error::ConsentRequired));
    assert_eq!(f.scan(), before);
    assert!(!f.folder().join("game").exists());
}

#[test]
fn admission_refuses_a_backup_folder_that_already_exists() {
    let (f, preview, _) = modified_with_replacement();
    let backup = preview.report.backup.clone().unwrap();
    assert!(backup.ends_with(format!(
        "cimmeria-adoption-backup-{}",
        preview.report.preview_handle
    )));
    std::fs::create_dir(&backup).unwrap();
    let before = f.scan();
    assert_eq!(
        confirm_replacing(preview, |_| Ok(())),
        Err(Error::BackupConflict)
    );
    assert_eq!(f.scan(), before);
    assert!(!f.folder().join("game").exists());
}

#[test]
fn admission_stages_each_replacement_under_the_work_folder() {
    let (f, preview, release) = modified_with_replacement();
    // The hook stops the moves at once; admission has already staged the replacements.
    let _ = confirm_replacing(preview, |_| Err(Error::Cancelled));
    let stage = std::fs::read_dir(f.folder())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".cimmeria-adopt-"))
        })
        .expect("admission made the work folder");
    let staged = std::fs::read(stage.join("replacements").join("later.txt")).unwrap();
    assert_eq!(staged, release);
}
