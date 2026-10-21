use super::*;
use crate::install_worker::fixtures;
use serde_json::json;

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) state: Arc<Mutex<DesktopState>>,
    pub(super) source: migration::LegacySource,
    pub(super) seed: PathBuf,
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
            source: SourceRequest::Legacy {
                import_digest: state.legacy_import().unwrap().unwrap().confirmation,
            },
            mode: AdoptionMode::Copy,
            destination: self.destination(),
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
        replace_modified_files: false,
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

/// Lays `folder` out as a stock install: the seed unpacked and prepared.
fn stage_stock(folder: &Path, seed: &Path) {
    std::fs::create_dir(folder).unwrap();
    let (progress, _) = crate::install_progress::ProgressSink::latest();
    crate::unpack::unpack(
        seed,
        folder,
        &crate::unpack::UnpackSink {
            progress,
            label: "fixture".into(),
            cancel: CancellationToken::new(),
        },
    )
    .unwrap();
    crate::client_setup::prepare(
        folder,
        &[crate::client_setup::LoginServer {
            name: "First".into(),
            url: "https://example.invalid/first".into(),
        }],
    )
    .unwrap();
}
/// The request that adopts a picked folder by a verified copy.
fn folder_copy_request(f: &Fixture, picked: &Path) -> PreviewRequest {
    let mut request = f.request();
    request.source = SourceRequest::Folder {
        selected: picked.to_path_buf(),
    };
    request
}

#[test]
fn a_stock_folder_copies_into_a_new_folder_and_its_scan_is_unchanged() {
    let f = Fixture::new();
    let stock = f.root.path().join("stock");
    stage_stock(&stock, &f.seed);
    let before = inventory::scan(&stock, &CancellationToken::new()).unwrap();
    let preview = preview(
        f.state.clone(),
        folder_copy_request(&f, &stock),
        CancellationToken::new(),
        crate::install_progress::ProgressSink::latest().0,
    )
    .unwrap();
    let provenance = commit(preview).unwrap();
    assert_eq!(
        inventory::scan(&stock, &CancellationToken::new()).unwrap(),
        before
    );
    let copy = inventory::scan(&f.destination().join("game"), &CancellationToken::new()).unwrap();
    assert_eq!(
        inventory::content_digest(&copy).unwrap(),
        provenance.reference_digest
    );
}

/// With a published index there is no reference to supply a file; the login
/// file the folder lacks is written by setup instead of failing the copy.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_published_index_copy_lets_setup_write_a_missing_login_file() {
    use super::verification_tests::{published_index, reference, release};
    let mut f = Fixture::new();
    let seed = std::fs::read(&f.seed).unwrap();
    let index = published_index(&seed);
    f.release = release(&seed, Some(reference(&index, None)));
    std::fs::remove_file(crate::client_setup::login_servers::path(
        &f.source.game_directory,
    ))
    .unwrap();
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::path("/index/file-index.json"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_bytes(index))
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
    let provenance = tokio::task::block_in_place(|| commit(preview)).unwrap();
    let copy = f.destination().join("game");
    assert!(crate::client_setup::login_servers::path(&copy).is_file());
    assert_eq!(
        inventory::content_digest(&inventory::scan(&copy, &CancellationToken::new()).unwrap())
            .unwrap(),
        provenance.reference_digest
    );
}

#[test]
fn a_copy_destination_inside_the_source_folder_is_refused() {
    let f = Fixture::new();
    let stock = f.root.path().join("stock");
    stage_stock(&stock, &f.seed);
    let mut request = folder_copy_request(&f, &stock);
    request.destination = stock.canonicalize().unwrap().join("desktop-copy");
    assert!(matches!(
        preview(
            f.state.clone(),
            request,
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0,
        ),
        Err(Error::Storage(StorageError::InvalidDirectory))
    ));
    assert!(!stock.join("desktop-copy").exists());
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
    let expected = preview.reference.as_ref().unwrap().index.clone();
    let provenance = commit(preview).unwrap();
    assert_eq!(f.source_snapshot(), before);
    let mut state = f.state.lock().unwrap();
    let installed = state.installed_content().unwrap().unwrap();
    assert_ne!(
        Some(installed.intent.operation_id),
        provenance.legacy_install_id
    );
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
    assert_eq!(
        Some(imported.identity.install_id),
        provenance.legacy_install_id
    );
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
    assert!(source::lock_source(preview.legacy().unwrap()).is_err());
    drop(preview);
    assert!(
        source::lock_source(&f.state.lock().unwrap().legacy_import().unwrap().unwrap()).is_ok()
    );
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
    // Creating a symlink needs a privilege Windows does not grant by default.
    #[cfg(unix)]
    {
        let f = Fixture::new();
        std::os::unix::fs::symlink(&f.seed, f.source.game_directory.join("link")).unwrap();
        let Err(Error::Unsafe(report)) = f.preview() else {
            panic!("expected an unsafe report");
        };
        assert_eq!(
            report.entries,
            [UnsafeEntry {
                reason: UnsafeReason::Link,
                path: Some("link".into()),
            }]
        );
        assert_eq!(report.total, 1);
        assert!(!f.destination().exists());
    }
}
#[test]
fn hardlinks_and_ambiguous_layouts_fail_closed() {
    let f = Fixture::new();
    std::fs::hard_link(
        f.source.game_directory.join("later.txt"),
        f.source.game_directory.join("other.txt"),
    )
    .unwrap();
    let Err(Error::Unsafe(report)) = f.preview() else {
        panic!("expected an unsafe report");
    };
    assert_eq!(
        report.entries,
        [
            UnsafeEntry {
                reason: UnsafeReason::HardLink,
                path: Some("later.txt".into()),
            },
            UnsafeEntry {
                reason: UnsafeReason::HardLink,
                path: Some("other.txt".into()),
            },
        ]
    );
    assert_eq!(report.total, 2);
    let f = Fixture::new();
    std::fs::write(f.source.game_directory.join("SGW.exe"), b"ambiguous").unwrap();
    assert!(matches!(
        f.preview(),
        Err(Error::Storage(StorageError::InvalidDirectory))
    ));
}

#[test]
fn adopted_copy_exposes_installed_content_and_uninstall_preserves_original() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    commit(f.preview().unwrap()).unwrap();
    let mut state = f.state.lock().unwrap();
    assert!(state.effective_launch_binding().unwrap().is_some());
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
    // Windows locks are mandatory: the worker releases the legacy launcher's lock
    // a moment after the operation reports Succeeded, so retry the read.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let after = loop {
        let scanned = (
            inventory::scan(&f.source.game_directory, &CancellationToken::new()),
            inventory::scan(&f.source.launcher_directory, &CancellationToken::new()),
        );
        match scanned {
            (Ok(game), Ok(launcher)) => break (game, launcher),
            _ if tokio::time::Instant::now() >= deadline => panic!("source stayed locked for 5 s"),
            _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
        }
    };
    assert_eq!(after, before);
    let state = f.state.lock().unwrap();
    assert_eq!(inspect(&state, id).unwrap().phase, Phase::Published);
}

#[tokio::test]
async fn signed_update_preserves_adoption_provenance_and_exact_legacy_import() {
    use crate::storage::update;
    use std::io::{Cursor, Write};
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    let f = Fixture::new();
    let source_before = f.source_snapshot();
    commit(f.preview().unwrap()).unwrap();
    let index_path = f.root.path().join("state/installed-content.json");
    let before: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&index_path).unwrap()).unwrap();
    let imported = std::fs::read(f.root.path().join("state/legacy-import.json")).unwrap();
    let mut zip = zip::ZipWriter::new_append(Cursor::new(fixtures::archive(true))).unwrap();
    zip.start_file("next-release.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"updated authenticated seed").unwrap();
    let seed = zip.finish().unwrap().into_inner();
    let release = fixtures::verified(&seed);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let plan = {
        let mut state = f.state.lock().unwrap();
        let installed = state.installed_content().unwrap().unwrap();
        let revision = state.operations().snapshot().revision;
        state
            .admit_update(update::Request {
                id: Uuid::new_v4(),
                operation_revision: revision,
                installation_id: installed.intent.operation_id,
                expected_current: installed.current_release,
                target: &release,
                confirmed: true,
            })
            .unwrap()
            .plan
    };
    let prepared = update::preparation::start(
        f.state.clone(),
        plan.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
    )
    .unwrap()
    .result
    .await
    .unwrap()
    .unwrap();
    update::commit::start(f.state.clone(), prepared)
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(index_path).unwrap()).unwrap();
    assert_eq!(before["adoption"], after["adoption"]);
    assert_eq!(before["intent"], after["intent"]);
    assert_eq!(
        std::fs::read(f.root.path().join("state/legacy-import.json")).unwrap(),
        imported
    );
    assert_eq!(f.source_snapshot(), source_before);
    assert_eq!(
        f.state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .current_release,
        plan.target
    );
}
#[test]
#[rustfmt::skip]
fn a_cancelled_copy_preview_is_not_a_comparison_rejection() {
    let f = Fixture::new();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let progress = crate::install_progress::ProgressSink::latest().0;
    let (result, record) = rejection::scope(|| preview_using(f.state.clone(), f.request(), cancel, progress, Backend::Native, None));
    assert_eq!(result.err(), Some(Error::Cancelled));
    assert_eq!(record, None);
}
#[tokio::test]
#[rustfmt::skip]
async fn a_worker_returns_the_refusal_it_recorded() {
    let f = Fixture::new();
    std::fs::create_dir(f.destination()).unwrap();
    let mut worker = test_support::start_preview(f.state.clone(), f.request(), "http://127.0.0.1:9/manifest.json".into()).unwrap();
    assert_eq!(worker.wait().await.err(), Some(Error::Storage(StorageError::InvalidDirectory)));
    assert_eq!(worker.last_rejection().map(|r| r.check), Some("destination_exists"));
}
#[test]
fn idle_refusals_record_the_revisions_they_compared() {
    let f = Fixture::new();
    let state = f.state.lock().unwrap();
    let operation = state.operations().snapshot().revision;
    let preferences = state.preferences().revision;
    let (result, record) = rejection::scope(|| idle(&state, operation + 1, preferences));
    assert_eq!(result, Err(Error::Operation(ContractError::StaleRevision)));
    let record = record.unwrap();
    assert_eq!(record.check, "stale_revision");
    assert_eq!(record.expected, Some((operation + 1).to_string()));
    assert_eq!(record.found, Some(operation.to_string()));
    let (result, record) = rejection::scope(|| idle(&state, operation, preferences + 1));
    assert_eq!(result, Err(Error::Storage(StorageError::StaleRevision)));
    let record = record.unwrap();
    assert_eq!(record.check, "stale_revision");
    assert_eq!(record.expected, Some((preferences + 1).to_string()));
    assert_eq!(record.found, Some(preferences.to_string()));
}
