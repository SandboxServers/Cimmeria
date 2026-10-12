//! In-place adoption against a real imported old-launcher folder (D-LX12).
use super::*;
use crate::install_worker::fixtures;
use serde_json::json;

const SCREENSHOT: &str = "Working/Binaries/screenshot.bmp";
const NOTES: &str = "notes.txt";

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
        let root = tempfile::tempdir().unwrap();
        let launcher = root.path().join("legacy");
        let game = root.path().join("old-game");
        std::fs::create_dir(&launcher).unwrap();
        std::fs::create_dir(&game).unwrap();
        let seed = root.path().join("seed.zip");
        let bytes = fixtures::archive(true);
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

#[test]
fn adopts_the_folder_in_place_by_rename_and_leaves_unlisted_files_where_they_were() {
    let f = Fixture::new(None);
    let exe = inventory::native(f.folder(), "Working/Binaries/SGW.exe");
    let exe_identity = inventory::identity(&exe).unwrap();
    let preview = f.preview().unwrap();
    assert!(preview.report.in_place());
    assert_eq!(preview.report.blocking().count(), 0);
    let expected = preview.reference.index.clone();
    commit(preview).unwrap();

    let game = f.folder().join("game");
    let moved = inventory::native(&game, "Working/Binaries/SGW.exe");
    // Renamed, not copied: the same file, now inside the owned tree.
    let identity = inventory::identity(&moved).unwrap();
    assert_eq!(
        (identity.device, identity.inode),
        (exe_identity.device, exe_identity.inode)
    );
    assert!(!exe.exists());
    assert_eq!(
        inventory::content_digest(&inventory::scan(&game, &CancellationToken::new()).unwrap())
            .unwrap(),
        inventory::content_digest(&expected).unwrap()
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
fn a_modified_release_file_blocks_adoption_names_it_and_changes_nothing() {
    let f = Fixture::new(None);
    std::fs::write(f.folder().join("later.txt"), b"edited by the player").unwrap();
    let before = f.scan();
    let preview = f.preview().unwrap();
    let blocking: Vec<_> = preview
        .report
        .blocking()
        .map(|d| (d.path.clone(), d.classification))
        .collect();
    assert_eq!(
        blocking,
        vec![("later.txt".to_string(), Classification::Modified)]
    );
    assert_eq!(preview.consent(&choices()), Err(Error::VerificationFailed));
    assert_eq!(commit(preview), Err(Error::VerificationFailed));
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
fn an_old_launcher_ledger_for_another_release_is_refused_before_any_download() {
    let f = Fixture::new(Some("another-seed"));
    assert_eq!(f.preview().err(), Some(Error::OutdatedInstall));
    let state = f.state.lock().unwrap();
    assert!(list_preparations(&state).unwrap().is_empty());
    assert!(state.operations().snapshot().operation.is_none());
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
    assert!(f.folder().join("launcher-installed.json").is_file());
}

#[test]
fn an_interrupted_move_is_recovered_after_restart() {
    let f = Fixture::new(None);
    let preview = f.preview().unwrap();
    let expected = inventory::content_digest(&preview.reference.index).unwrap();
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
    let game = f.folder().join("game");
    assert_eq!(
        inventory::content_digest(&inventory::scan(&game, &CancellationToken::new()).unwrap())
            .unwrap(),
        expected
    );
    assert!(!inventory::native(f.folder(), "Working/Binaries/SGW.exe").exists());
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
