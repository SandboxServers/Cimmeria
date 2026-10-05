use super::tests::Fixture;
use super::*;

#[test]
fn retained_preview_owns_the_operation_and_releases_only_its_reference() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let preview = f.preview().unwrap();
    let id = preview.report.preview_handle;
    let directory = preview.preparation.record.directory.clone();
    {
        let mut state = f.state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        assert_eq!(
            state.operations_mut().unwrap().begin(
                Uuid::new_v4(),
                OperationKind::Install,
                [0; 32],
                revision
            ),
            Err(ContractError::Busy)
        );
        assert_eq!(
            abandon_preparation(&mut state, id, revision),
            Err(Error::Storage(StorageError::InUse))
        );
        use base64::Engine;
        let keys = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let config = crate::updater::Config::new(
            "https://updates.example.test/feed",
            &base64::engine::general_purpose::STANDARD
                .encode(keys.pk.to_box().unwrap().to_string()),
            "1.0.0",
            "darwin-aarch64",
            vec!["updates.example.test".into()],
        )
        .unwrap();
        let update = state.launcher_update_snapshot(Some(&config)).unwrap();
        assert!(matches!(
            state.begin_launcher_update_check(Some(&config), update.revision, revision),
            Err(crate::updater::Error::Busy)
        ));
        assert_eq!(
            list_preparations(&state).unwrap(),
            vec![preview.preparation.record.clone()]
        );
        let work = state.extraction_work(id).unwrap();
        assert_eq!(work.stage, directory.join("raw"));
        assert_eq!(work.cache, directory.join("cache"));
        assert_ne!(work.installation.operation_id, id);
        assert!(!state
            .state_root()
            .join(format!(
                "install-intent-{}.json",
                work.installation.operation_id
            ))
            .exists());
        assert!(state.cached_extraction_release(id).is_ok());
    }
    assert!(matches!(
        f.preview(),
        Err(Error::Operation(ContractError::Busy))
    ));
    drop(preview);
    assert!(!directory.exists());
    assert!(list_preparations(&f.state.lock().unwrap())
        .unwrap()
        .is_empty());
    assert!(!f.destination().exists());
    assert_eq!(f.source_snapshot(), before);
    assert_eq!(
        f.state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Cancelled
    );
}

#[test]
fn uncertain_reference_survives_drop_and_reopen_requires_explicit_cleanup() {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let mut preview = f.preview().unwrap();
    let record = preview.preparation.record.clone();
    // Model the adapter's uncertain completion: no failure path may replay or
    // remove a possibly live extraction directory.
    preview.preparation.uncertain = true;
    drop(preview);
    assert!(record.stage().exists());
    assert!(matches!(
        f.preview(),
        Err(Error::Operation(ContractError::Busy))
    ));
    let root = f.state.lock().unwrap().state_root().to_path_buf();
    let state = f.state.clone();
    drop(f.state);
    drop(state);
    let mut state = DesktopState::open(&root).unwrap();
    let revision = state.operations().snapshot().revision;
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert_eq!(
        abandon_preparation(&mut state, record.id, revision - 1),
        Err(Error::Operation(ContractError::StaleRevision))
    );
    abandon_preparation(&mut state, record.id, revision).unwrap();
    assert!(!record.directory.exists());
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Cancelled
    );
    let imported = state.legacy_import().unwrap().unwrap();
    assert_eq!(
        inventory::scan(&imported.source.game_directory, &CancellationToken::new()).unwrap(),
        before.0
    );
    assert_eq!(
        inventory::scan(
            &imported.source.launcher_directory,
            &CancellationToken::new()
        )
        .unwrap(),
        before.1
    );
}

#[test]
fn reference_cleanup_refuses_replaced_directories_and_nested_links() {
    for linked in [false, true] {
        let f = Fixture::new();
        let mut preview = f.preview().unwrap();
        let record = preview.preparation.record.clone();
        preview.preparation.uncertain = true;
        drop(preview);
        let foreign = f.root.path().join("foreign");
        std::fs::create_dir(&foreign).unwrap();
        std::fs::write(foreign.join("sentinel"), b"keep").unwrap();
        if linked {
            std::os::unix::fs::symlink(&foreign, record.directory.join("foreign")).unwrap();
        } else {
            std::fs::rename(&record.directory, f.root.path().join("saved-reference")).unwrap();
            std::fs::rename(&foreign, &record.directory).unwrap();
        }
        let mut state = f.state.lock().unwrap();
        let revision = state.operations().snapshot().revision;
        assert!(abandon_preparation(&mut state, record.id, revision).is_err());
        let sentinel = if linked {
            foreign.join("sentinel")
        } else {
            record.directory.join("sentinel")
        };
        assert_eq!(std::fs::read(sentinel).unwrap(), b"keep");
        assert_eq!(
            state
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
    }
}

#[test]
fn orphan_reference_cleanup_uses_saved_identity_and_never_replays_work() {
    let f = Fixture::new();
    let preview = f.preview().unwrap();
    let record = preview.preparation.record.clone();
    let mut state = f.state.lock().unwrap();
    // A consumed handoff can leave its reference behind if the caller already
    // holds the mutex when dropping it. Cleanup uses its durable identity.
    preview.preparation.handoff(&mut state).unwrap();
    drop(preview);
    let revision = state.operations().snapshot().revision;
    let next = Uuid::new_v4();
    state
        .operations_mut()
        .unwrap()
        .begin(next, OperationKind::Launch, [9; 32], revision)
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(next, OperationState::Failed)
        .unwrap();
    let revision = state.operations().snapshot().revision;
    abandon_preparation(&mut state, record.id, revision).unwrap();
    assert!(!record.directory.exists());
    assert_eq!(
        state.operations().snapshot().operation.as_ref().unwrap().id,
        next
    );
    assert!(!f.destination().exists());
}

#[tokio::test]
#[ignore = "runs the pinned Windows-native helper headlessly with isolated fixture state and Wine prefix"]
async fn retained_wine_reference_uses_real_rar_adapter_then_publishes_verified_copy() {
    wine_case(false).await;
}
#[tokio::test]
#[ignore = "runs the pinned Windows helper cabinet.dll extraction in an isolated Wine prefix"]
async fn retained_wine_reference_expands_installer_cabinet_then_publishes_verified_copy() {
    wine_case(true).await;
}
async fn wine_case(cabinet: bool) {
    let f = Fixture::new();
    let before = f.source_snapshot();
    let mut request = f.request();
    let bytes = std::fs::read(&request.artifacts.as_ref().unwrap().seed).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let entries: Vec<(String, Vec<u8>)> = (0..zip.len())
        .map(|i| {
            let mut file = zip.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            (name, bytes)
        })
        .collect();
    let entries = if cabinet {
        let cab = super::cab_fixture::cabinet(&entries);
        let mut inf =
            "[cabinet list]\r\nDisk 1, Cabinet 1, DATA1.CAB\r\n[file list]\r\n".to_string();
        for (i, (name, bytes)) in entries.iter().enumerate() {
            inf.push_str(&format!(
                "{}: Cabinet 1, {}, {}\r\n",
                i + 1,
                name.replace('/', "\\"),
                bytes.len()
            ));
        }
        vec![
            ("Data/DATA.INF".to_string(), inf.into_bytes()),
            ("Data/DATA1.CAB".to_string(), cab),
        ]
    } else {
        entries
    };
    let archive = f.root.path().join("seed.rar");
    crate::unpack::test_fixtures::write_stored_rar4(
        &archive,
        &entries
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_slice()))
            .collect::<Vec<_>>(),
    );
    request.release = crate::install_worker::fixtures::verified(&std::fs::read(&archive).unwrap());
    request.artifacts.as_mut().unwrap().seed = archive;
    let helper = crate::mac_wine::HelperResource::open(
        PathBuf::from(
            std::env::var_os("CIMMERIA_WINE_HELPER").expect("native-built pinned helper"),
        ),
        &std::env::var("CIMMERIA_WINE_HELPER_SHA256").expect("independent build identity"),
    )
    .unwrap();
    if let Some(cached) = std::env::var_os("CIMMERIA_WINE_RUNTIME_TREE") {
        let runtimes = f.root.path().join("state/runtimes");
        std::fs::create_dir_all(&runtimes).unwrap();
        let destination = runtimes.join("wine-r17-dc67cf0c2dd1e4c1");
        assert!(std::process::Command::new("/bin/cp")
            .arg("-R")
            .arg(cached)
            .arg(destination)
            .status()
            .unwrap()
            .success());
    }
    let mut worker = start_preview_wine(f.state.clone(), request, helper).unwrap();
    let preview = worker.wait().await.unwrap();
    let id = preview.report.preview_handle;
    assert!(matches!(
        preview.preparation.record.descriptor.backend,
        ExtractionBackend::Wine { .. }
    ));
    assert_eq!(
        f.state
            .lock()
            .unwrap()
            .helper_record(id)
            .unwrap()
            .unwrap()
            .phase,
        crate::HelperPhase::Finished {
            result: crate::HelperResult::Completed
        }
    );
    let work = Uuid::new_v4();
    let worker = start_confirmation(preview, work, id, super::tests::choices()).unwrap();
    let mut result = worker.result;
    result
        .wait_for(|value| value.is_some())
        .await
        .unwrap()
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap();
    assert_eq!(f.source_snapshot(), before);
    assert!(f
        .destination()
        .join("game/Working/Binaries/SGW.exe")
        .is_file());
    assert_eq!(
        f.state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
}

#[test]
fn live_helper_evidence_blocks_reference_cleanup_without_replay() {
    let f = Fixture::new();
    let request = f.request();
    let mut owner = preparation::Ownership::claim(
        f.state.clone(),
        &request,
        ExtractionBackend::Wine {
            runtime_sha256: {
                let bytes = crate::mac_runtime::ARCHIVE_SHA256.as_bytes();
                std::array::from_fn(|i| {
                    u8::from_str_radix(std::str::from_utf8(&bytes[i * 2..i * 2 + 2]).unwrap(), 16)
                        .unwrap()
                })
            },
            helper_sha256: [3; 32],
        },
        vec![],
    )
    .unwrap();
    let record = owner.record.clone();
    {
        let mut state = f.state.lock().unwrap();
        let helper = state.begin_helper(record.id).unwrap();
        state
            .record_helper_host(record.id, helper.attempt_id, std::process::id())
            .unwrap();
        assert!(state.begin_helper(record.id).is_err());
    }
    owner.uncertain = true;
    drop(owner);
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    assert!(abandon_preparation(&mut state, record.id, revision).is_err());
    assert!(record.directory.exists());
    assert_eq!(
        state.helper_record(record.id).unwrap().unwrap().phase,
        crate::HelperPhase::HostStarted
    );
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
}

#[test]
fn missing_reference_needs_cleanup_checkpoint_and_completed_cleanup_is_idempotent() {
    let f = Fixture::new();
    let preview = f.preview().unwrap();
    let record = preview.preparation.record.clone();
    drop(preview);
    let mut state = f.state.lock().unwrap();
    let revision = state.operations().snapshot().revision;
    abandon_preparation(&mut state, record.id, revision).unwrap();
    std::fs::remove_file(
        state
            .state_root()
            .join(format!("adoption-reference-cleanup-{}.json", record.id)),
    )
    .unwrap();
    assert!(abandon_preparation(&mut state, record.id, revision).is_err());
}
