use super::*;
use crate::install_worker::fixtures;
use serde_json::json;

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) state: Arc<Mutex<DesktopState>>,
    source: migration::LegacySource,
    seed: PathBuf,
    release: VerifiedRelease,
}
impl Fixture {
    pub(super) fn new() -> Self {
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
        crate::client_setup::prepare(
            &game,
            &[
                crate::client_setup::LoginServer {
                    name: "Second".into(),
                    url: "https://example.invalid/second".into(),
                },
                crate::client_setup::LoginServer {
                    name: "First".into(),
                    url: "https://example.invalid/first".into(),
                },
            ],
        )
        .unwrap();
        std::fs::write(launcher.join("launcher-config.json"), serde_json::to_vec_pretty(&json!({"install_path":"C:\\Old Game", "manifest_url":crate::catalog::URL,"login_servers":[{"name":"Second","url":"https://example.invalid/second"},{"name":"First","url":"https://example.invalid/first"}],"client_patches":{"enabled":false},"telemetry":{"enabled":true,"opted_in":true,"prompt_answered":true,"auth_url":"https://example.invalid/auth"},"unknown_future":"preserved verbatim"})).unwrap()).unwrap();
        std::fs::write(launcher.join("install.json"), br#"{"schema_version":1,"install_id":"72a8a13b-2a4e-4ea0-b5ba-5ba3cf0a619d","machine_id":"fixture-machine","first_seen_ms":123456,"created_by_launcher_version":"0.8.4"}"#).unwrap();
        std::fs::write(game.join("launcher-installed.json"), br#"{"seed_sha256":"forged-current","seed_adopted":false,"applied_patches":["duplicate","duplicate"]}"#).unwrap();
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
    pub(super) fn destination(&self) -> PathBuf {
        self.root
            .path()
            .canonicalize()
            .unwrap()
            .join("desktop-copy")
    }
    pub(super) fn request(&self) -> PreviewRequest {
        let state = self.state.lock().unwrap();
        PreviewRequest {
            import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            destination: self.destination(),
            operation_revision: state.operations().snapshot().revision,
            preferences_revision: state.preferences().revision,
            release: crate::catalog::verify_release(
                self.release.evidence().0,
                self.release.evidence().1,
            )
            .unwrap(),
            artifacts: Artifacts {
                seed: self.seed.clone(),
                patches: vec![],
            },
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
    pub(super) fn source_snapshot(&self) -> (inventory::Index, inventory::Index) {
        (
            inventory::scan(&self.source.game_directory, &CancellationToken::new()).unwrap(),
            inventory::scan(&self.source.launcher_directory, &CancellationToken::new()).unwrap(),
        )
    }
}
pub(super) fn choices() -> Choices {
    Choices {
        normalize_managed_files: true,
        accept_unavailable_game_telemetry: true,
        old_game_closed: true,
    }
}
fn commit(preview: Preview) -> Result<Provenance, Error> {
    let handle = preview.report.preview_handle;
    confirm(
        preview,
        Uuid::new_v4(),
        handle,
        choices(),
        CancellationToken::new(),
    )
}

#[test]
fn signed_copy_ignores_forged_ledger_preserves_source_identity_config_and_excludes_unknown_code() {
    let f = Fixture::new();
    std::fs::write(f.source.game_directory.join("later.txt"), b"modified").unwrap();
    std::fs::write(
        f.source.game_directory.join("unknown.dll"),
        b"untrusted plugin",
    )
    .unwrap();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    assert!(preview
        .report
        .files
        .iter()
        .any(|d| d.path == "later.txt" && d.classification == Classification::Modified));
    assert!(preview
        .report
        .files
        .iter()
        .any(|d| d.path == "unknown.dll" && d.classification == Classification::Extra));
    let expected = preview.reference.index.clone();
    let provenance = commit(preview).unwrap();
    assert_eq!(f.source_snapshot(), before);
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_ne!(installed.intent.operation_id, provenance.legacy_install_id);
    assert_ne!(installed.intent.operation_id, provenance.work_id);
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .kind,
        OperationKind::Adopt
    );
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
    assert_eq!(
        inventory::content_digest(
            &inventory::scan(&f.destination().join("game"), &CancellationToken::new()).unwrap()
        )
        .unwrap(),
        inventory::content_digest(&expected).unwrap()
    );
    assert!(!f.destination().join("game/unknown.dll").exists());
    assert_eq!(
        std::fs::read(f.destination().join("game/later.txt")).unwrap(),
        b"later entry"
    );
    assert!(!state.preferences().launcher_summary_consent);
    let imported = state.legacy_import().unwrap().unwrap();
    assert!(!imported.config.client_patches.enabled);
    assert!(imported.config.telemetry.opted_in);
    assert_eq!(imported.identity.install_id, provenance.legacy_install_id);
    assert_eq!(installed.intent.login_servers[0].name, "Second");
    drop(state);
    std::fs::write(f.source.game_directory.join("later.txt"), b"after adoption").unwrap();
    assert_eq!(
        std::fs::read(f.destination().join("game/later.txt")).unwrap(),
        b"later entry"
    );
}
#[test]
fn source_edits_and_missing_consents_fail_before_destination_claim() {
    let f = Fixture::new();
    let preview = f.preview().unwrap();
    std::fs::write(
        f.source.game_directory.join("later.txt"),
        b"changed after review",
    )
    .unwrap();
    assert_eq!(commit(preview), Err(Error::SourceChanged));
    assert!(!f.destination().exists());
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    let mut consent = choices();
    consent.normalize_managed_files = false;
    assert_eq!(
        confirm(
            preview,
            Uuid::new_v4(),
            handle,
            consent,
            CancellationToken::new()
        ),
        Err(Error::ConsentRequired)
    );
    assert!(!f.destination().exists());
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    consent = choices();
    consent.accept_unavailable_game_telemetry = false;
    assert_eq!(
        confirm(
            preview,
            Uuid::new_v4(),
            handle,
            consent,
            CancellationToken::new()
        ),
        Err(Error::ConsentRequired)
    );
}
#[test]
fn actual_legacy_lock_is_held_for_the_entire_review_and_released_on_cancel() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    assert!(lock_source(&preview.imported).is_err());
    drop(preview);
    assert!(lock_source(&f.state.lock().unwrap().legacy_import().unwrap().unwrap()).is_ok());
    assert_eq!(f.source_snapshot(), before);
    assert!(!f.destination().exists());
}
#[test]
fn raw_aslr_is_compared_whole_file_then_normalized_only_in_copy() {
    let f = Fixture::new();
    let path = crate::install_layout::sgw_exe(&f.source.game_directory);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0x186] |= 0x40;
    std::fs::write(&path, &bytes).unwrap();
    let preview = f.preview().unwrap();
    assert!(
        preview
            .report
            .files
            .iter()
            .any(|d| d.path.ends_with("SGW.exe")
                && d.classification == Classification::KnownTransform)
    );
    commit(preview).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        std::fs::read(f.destination().join("game/Working/Binaries/SGW.exe")).unwrap()[0x186] & 0x40,
        0
    );
}
#[test]
fn arbitrary_executable_edit_is_modified_even_with_known_aslr_bit() {
    let f = Fixture::new();
    let path = crate::install_layout::sgw_exe(&f.source.game_directory);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0x186] |= 0x40;
    bytes[0x1ff] = 42;
    std::fs::write(path, bytes).unwrap();
    let preview = f.preview().unwrap();
    assert!(preview
        .report
        .files
        .iter()
        .any(|d| d.path.ends_with("SGW.exe") && d.classification == Classification::Modified));
}
#[test]
fn corrupt_signed_artifact_and_nested_links_are_refused_without_source_changes() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    std::fs::write(&f.seed, b"not the signed zip").unwrap();
    assert!(matches!(f.preview(), Err(Error::InvalidArtifact)));
    assert_eq!(f.source_snapshot(), before);
    let f = Fixture::new();
    std::os::unix::fs::symlink(&f.seed, f.source.game_directory.join("link")).unwrap();
    assert!(matches!(
        f.preview(),
        Err(Error::Storage(StorageError::UnsafeFile))
    ));
    assert!(!f.destination().exists());
}
#[test]
fn hardlinks_and_ambiguous_layouts_fail_closed() {
    let f = Fixture::new();
    std::fs::hard_link(
        f.source.game_directory.join("later.txt"),
        f.source.game_directory.join("other.txt"),
    )
    .unwrap();
    assert!(matches!(
        f.preview(),
        Err(Error::Storage(StorageError::UnsafeFile))
    ));
    let f = Fixture::new();
    std::fs::write(f.source.game_directory.join("SGW.exe"), b"ambiguous").unwrap();
    assert!(matches!(
        f.preview(),
        Err(Error::Storage(StorageError::InvalidDirectory))
    ));
}

#[test]
fn adopted_copy_is_not_play_ready_and_uninstall_preserves_original() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    commit(f.preview().unwrap()).unwrap();
    let mut state = f.state.lock().unwrap();
    assert!(matches!(
        state.installed_content_readonly(),
        Err(StorageError::Busy)
    ));
    let target = state.uninstall_target().unwrap().unwrap();
    assert_eq!(target.directory, f.destination());
    let revision = state.operations().snapshot().revision;
    state
        .uninstall(Uuid::new_v4(), revision, target.installation_id, true)
        .unwrap();
    assert!(!f.destination().exists());
    assert_eq!(f.source_snapshot(), before);
    assert!(state.installed_content().unwrap().is_none());
}

#[test]
fn overlap_stale_review_and_preconfirmation_cancel_never_claim_destination() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let mut request = f.request();
    request.destination = f.source.game_directory.join("nested-copy");
    assert!(matches!(
        preview(
            f.state.clone(),
            request,
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0
        ),
        Err(Error::Storage(StorageError::InvalidDirectory))
    ));
    let p = f.preview().unwrap();
    let handle = p.report.preview_handle;
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        confirm(p, Uuid::new_v4(), handle, choices(), cancel),
        Err(Error::Cancelled)
    );
    let p = f.preview().unwrap();
    {
        let mut state = f.state.lock().unwrap();
        let old = state.preferences().clone();
        state
            .save_preferences(old.install_directory, true, old.revision)
            .unwrap();
    }
    assert_eq!(commit(p), Err(Error::Storage(StorageError::StaleRevision)));
    assert_eq!(f.source_snapshot(), before);
    assert!(!f.destination().exists());
}
#[tokio::test]
async fn confirmation_observer_loss_does_not_abort_the_retained_worker() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    let id = Uuid::new_v4();
    let worker = start_confirmation(preview, id, handle, choices()).unwrap();
    drop(worker);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let done = f
                .state
                .lock()
                .unwrap()
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| op.id == id && op.state == OperationState::Succeeded);
            if done {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(f.source_snapshot(), before);
    let state = f.state.lock().unwrap();
    assert_eq!(inspect(&state, id).unwrap().phase, Phase::Published);
}
