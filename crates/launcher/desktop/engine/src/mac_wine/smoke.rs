use super::*;
use crate::{catalog::verify_release, AdmissionRequest, OperationState};
use ed25519_dalek::{Signer, SigningKey};
use std::io::Write;
#[tokio::test]
#[ignore = "downloads pinned Wine and runs the Windows CI helper headlessly in a private temporary prefix"]
async fn native_windows_helper_extracts_zip_under_managed_wine() {
    let helper = PathBuf::from(
        std::env::var_os("CIMMERIA_WINE_HELPER").expect("set Windows-native CI helper path"),
    );
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let destination = root.path().canonicalize().unwrap().join("game");
    state
        .save_preferences(Some(destination.clone()), false, 0)
        .unwrap();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(
        "Unicode 星門/hello.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"headless fixture").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let seed_hash = hex(&Sha256::digest(&bytes));
    let body=serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"fixture.zip","size":bytes.len(),"sha256":seed_hash},"patches":[]})).unwrap();
    let sig = SigningKey::from_bytes(&[0x2a; 32]).sign(&body);
    let signature = hex(&sig.to_bytes());
    let release = verify_release(&body, signature.as_bytes()).unwrap();
    let id = Uuid::new_v4();
    let runtime_hash: Vec<u8> = mac_runtime::ARCHIVE_SHA256
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Wine {
                runtime_sha256: runtime_hash.try_into().unwrap(),
                helper_sha256: file_digest(&helper),
            },
        })
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    let state = Arc::new(Mutex::new(state));
    let cancel = CancellationToken::new();
    let mut adapter = WineSeedExtractor::prepare(
        state.clone(),
        id,
        helper,
        cancel.clone(),
        ProgressSink::latest().0,
    )
    .await
    .unwrap();
    adapter.limits.operation = std::time::Duration::from_secs(120);
    let cache = destination.join(format!(".cimmeria-cache-{id}"));
    std::fs::create_dir_all(&cache).unwrap();
    let archive = cache.join("seed.zip");
    std::fs::write(&archive, bytes).unwrap();
    let stage = destination.join(format!(".cimmeria-stage-{id}"));
    let result = adapter
        .extract(SeedExtraction {
            archive: &archive,
            destination: &stage,
            sha256: &seed_hash,
            cancel,
            progress: ProgressSink::latest().0,
        })
        .await;
    if result.is_err() {
        eprintln!(
            "helper record: {:?}",
            state.lock().unwrap().helper_record(id)
        );
    }
    result.unwrap();
    assert_eq!(
        std::fs::read(stage.join("Unicode 星門/hello.txt")).unwrap(),
        b"headless fixture"
    );
    let record = state.lock().unwrap().helper_record(id).unwrap().unwrap();
    assert_eq!(
        record.phase,
        crate::HelperPhase::Finished {
            result: crate::HelperResult::Completed
        }
    );
    // Simulate reopening after extraction but before a content terminal commit.
    drop(adapter);
    let state_root = state.lock().unwrap().state_root().to_path_buf();
    drop(state);
    tokio::task::spawn_blocking(move || {
        let reopened = DesktopState::open(&state_root).unwrap();
        assert_eq!(
            reopened
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
        let _stopped = recovery::stop_for_recovery(&reopened).unwrap();
        assert_eq!(
            std::fs::read(stage.join("Unicode 星門/hello.txt")).unwrap(),
            b"headless fixture"
        );
        // Quiescence neither deletes content nor marks the operation complete.
        assert_eq!(
            reopened
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .state,
            OperationState::ReconciliationRequired
        );
    })
    .await
    .unwrap();
}

fn file_digest(path: &Path) -> [u8; 32] {
    let mut file = File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    hash.finalize().into()
}

#[tokio::test]
#[ignore = "requires the original client RAR, signed manifest, and Windows-native CI helper; headless extraction only"]
async fn original_client_rar_under_managed_wine() {
    let source = PathBuf::from(std::env::var_os("SGW_CLIENT_RAR").expect("set SGW_CLIENT_RAR"));
    let manifest = PathBuf::from(
        std::env::var_os("CIMMERIA_SMOKE_MANIFEST").expect("set signed manifest path"),
    );
    let body = std::fs::read(&manifest).unwrap();
    let signature = std::fs::read(manifest.with_extension("json.sig")).unwrap();
    let release =
        verify_release(&body, &signature).expect("manifest must authenticate before extraction");
    assert_eq!(
        std::fs::metadata(&source).unwrap().len(),
        release.manifest().seed.size
    );
    assert_eq!(hex(&file_digest(&source)), release.manifest().seed.sha256);
    let helper = PathBuf::from(
        std::env::var_os("CIMMERIA_WINE_HELPER").expect("set Windows-native helper path"),
    );
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let destination = root.path().canonicalize().unwrap().join("game");
    state
        .save_preferences(Some(destination.clone()), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let runtime_hash: Vec<u8> = mac_runtime::ARCHIVE_SHA256
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release: &release,
            login_servers: vec![],
            backend: ExtractionBackend::Wine {
                runtime_sha256: runtime_hash.try_into().unwrap(),
                helper_sha256: file_digest(&helper),
            },
        })
        .unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    let state = Arc::new(Mutex::new(state));
    let adapter = WineSeedExtractor::prepare(
        state.clone(),
        id,
        helper,
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .await
    .unwrap();
    let cache = destination.join(format!(".cimmeria-cache-{id}"));
    std::fs::create_dir_all(&cache).unwrap();
    let archive = cache.join("client.rar");
    // Read-only input copy; never let test cleanup remove the caller's archive.
    std::fs::copy(source, &archive).unwrap();
    let stage = destination.join(format!(".cimmeria-stage-{id}"));
    let (progress, mut observed) = ProgressSink::latest();
    let running = adapter.extract(SeedExtraction {
        archive: &archive,
        destination: &stage,
        sha256: &release.manifest().seed.sha256,
        cancel: CancellationToken::new(),
        progress,
    });
    tokio::pin!(running);
    let result = loop {
        tokio::select! {
            result = &mut running => break result,
            update = observed.changed() => {
                if update.is_err() { break running.await; }
                if let Some(crate::install::Progress::Extracting { current, total, .. }) = observed.borrow_and_update().as_ref() {
                    eprintln!("extraction {current}/{total}");
                }
            }
        }
    };
    eprintln!(
        "helper result: {:?}",
        state.lock().unwrap().helper_record(id)
    );
    result.unwrap();
    let exe = crate::install_layout::sgw_exe(&stage);
    assert!(std::fs::metadata(&exe).unwrap().len() > 0);
    let mut magic = [0u8; 2];
    File::open(exe).unwrap().read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"MZ");
    assert!(crate::install_layout::sgwgame_dir(&stage).is_dir());
    assert!(!stage.join(".tmp-unpack").exists());
    assert_eq!(
        state
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
    // This check proves original archive extraction, not patching, prerequisites,
    // executable launch, login, or gameplay. TempDir removes only this owned tree.
}
