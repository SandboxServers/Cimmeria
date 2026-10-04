use super::*;
use crate::{install::SeedExtraction, OperationState};
fn repair() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    ExtractionWork,
    PathBuf,
) {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    let work = state.extraction_work(plan.id).unwrap();
    let state_root = state.state_root().to_path_buf();
    (root, Arc::new(Mutex::new(state)), work, state_root)
}
#[test]
fn repair_prefix_is_distinct_from_install_prefix_and_cannot_be_readopted() {
    let (_root, _state, work, state_root) = repair();
    let (original, original_owner) =
        claim_prefix(&state_root.join("wine-prefixes"), &work.installation).unwrap();
    let (repair, owner) = claim_work_prefix(&state_root, &work).unwrap();
    assert_ne!(repair, original);
    assert_eq!(
        repair,
        state_root
            .join("wine-repair-prefixes")
            .join(work.operation_id.to_string())
            .join("bottle")
    );
    let saved: RepairOwner = serde_json::from_slice(
        &std::fs::read(repair.parent().unwrap().join("owner.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved.schema_version, 1);
    assert_eq!(saved.work, work);
    assert_eq!(
        std::fs::read_link(repair.join("dosdevices/c:")).unwrap(),
        Path::new("../drive_c")
    );
    assert!(claim_work_prefix(&state_root, &work).is_err());
    drop(owner);
    assert!(claim_work_prefix(&state_root, &work).is_err());
    drop(original_owner);
    let saved: InstallIntent = serde_json::from_slice(
        &std::fs::read(original.parent().unwrap().join("owner.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, work.installation);
}
#[tokio::test]
async fn repair_helper_uses_work_id_and_rejects_original_install_paths() {
    use std::os::unix::fs::PermissionsExt;
    let (root, state, work, state_root) = repair();
    let (prefix, owner) = claim_work_prefix(&state_root, &work).unwrap();
    let runtime = root.path().join("fake-runtime");
    std::fs::create_dir_all(runtime.join("bin")).unwrap();
    let event = serde_json::to_string(&crate::archive_worker::WorkerEvent {
        schema_version: 1,
        operation_id: Some(work.operation_id),
        event: crate::archive_worker::EventKind::Finished { error: None },
    })
    .unwrap();
    let script = format!("#!/bin/sh\nread -r request\nprintf '%s' \"$request\" > \"$WINEPREFIX/request.json\"\nprintf '%s\\n' '{event}'\n");
    for (name, script) in [
        ("wine", script.as_str()),
        ("wineserver", "#!/bin/sh\nexit 0\n"),
    ] {
        let path = runtime.join("bin").join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::create_dir_all(&work.cache).unwrap();
    let archive = work.cache.join("seed.zip");
    let adapter = WineSeedExtractor {
        state: state.clone(),
        work: work.clone(),
        runtime,
        helper: PathBuf::from("/fixture/helper.exe"),
        prefix: prefix.clone(),
        _owner: owner,
        used: std::sync::atomic::AtomicBool::new(false),
        limits: Deadlines::default(),
    };
    let hash = "a".repeat(64);
    let original_stage = work.installation.destination.join(format!(
        ".cimmeria-stage-{}",
        work.installation.operation_id
    ));
    let request = |destination| SeedExtraction {
        archive: &archive,
        destination,
        sha256: &hash,
        cancel: CancellationToken::new(),
        progress: ProgressSink::latest().0,
    };
    assert!(adapter.extract(request(&original_stage)).await.is_err());
    assert!(state
        .lock()
        .unwrap()
        .helper_record(work.operation_id)
        .unwrap()
        .is_none());
    let (progress, _updates) = tokio::sync::watch::channel(None);
    assert!(helper_supervisor::run_owned(
        state.clone(),
        adapter.command().unwrap(),
        ExtractRequest {
            schema_version: 1,
            operation_id: work.operation_id,
            archive: paths::guest(&archive).unwrap().into(),
            destination: paths::guest(&work.stage).unwrap().into(),
            sha256: "0".repeat(64)
        },
        CancellationToken::new(),
        progress,
        Deadlines::default()
    )
    .await
    .is_err());
    assert!(state
        .lock()
        .unwrap()
        .helper_record(work.operation_id)
        .unwrap()
        .is_none());
    assert!(!prefix.join("request.json").exists());
    adapter.extract(request(&work.stage)).await.unwrap();
    let sent: ExtractRequest =
        serde_json::from_slice(&std::fs::read(prefix.join("request.json")).unwrap()).unwrap();
    assert_eq!(sent.operation_id, work.operation_id);
    assert_ne!(sent.operation_id, work.installation.operation_id);
    assert_eq!(
        sent.destination,
        PathBuf::from(paths::guest(&work.stage).unwrap())
    );
    assert_eq!(
        state
            .lock()
            .unwrap()
            .helper_record(work.operation_id)
            .unwrap()
            .unwrap()
            .intent_digest,
        work.intent_digest
    );
    assert!(adapter.extract(request(&work.stage)).await.is_err());
    assert!(work.installation.destination.join("game").is_dir());
}

#[tokio::test]
#[ignore = "downloads pinned Wine and runs Windows-native helper in an isolated headless repair prefix"]
async fn native_helper_extracts_zip_with_repair_work_identity() {
    let helper = PathBuf::from(
        std::env::var_os("CIMMERIA_WINE_HELPER").expect("Windows-native helper path"),
    );
    let runtime_hash: Vec<u8> = mac_runtime::ARCHIVE_SHA256
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(
        "Unicode 星門/repaired.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"repair adapter fixture").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let digest = hex(&Sha256::digest(&bytes));
    let helper_hash = Sha256::digest(std::fs::read(&helper).unwrap()).into();
    let (_root, mut state, installed) = crate::runtime_setup::tests::fixture_with_backend(
        runtime_hash.try_into().unwrap(),
        helper_hash,
        &digest,
        bytes.len(),
    );
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    state
        .operations_mut()
        .unwrap()
        .observe(plan.id, OperationState::Running)
        .unwrap();
    let work = state.extraction_work(plan.id).unwrap();
    let state = Arc::new(Mutex::new(state));
    let mut adapter = WineSeedExtractor::prepare(
        state.clone(),
        plan.id,
        helper,
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .await
    .unwrap();
    adapter.limits.operation = std::time::Duration::from_secs(120);
    std::fs::create_dir_all(&work.cache).unwrap();
    let archive = work.cache.join("seed.zip");
    std::fs::write(&archive, bytes).unwrap();
    adapter
        .extract(SeedExtraction {
            archive: &archive,
            destination: &work.stage,
            sha256: &digest,
            cancel: CancellationToken::new(),
            progress: ProgressSink::latest().0,
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(work.stage.join("Unicode 星門/repaired.txt")).unwrap(),
        b"repair adapter fixture"
    );
    assert_eq!(
        std::fs::read(installed.destination.join("game/Working/Binaries/SGW.exe")).unwrap(),
        b"inert fixture"
    );
    let record = state
        .lock()
        .unwrap()
        .helper_record(plan.id)
        .unwrap()
        .unwrap();
    assert_eq!(record.operation_id, plan.id);
    assert_eq!(record.intent_digest, work.intent_digest);
    assert_eq!(
        record.phase,
        crate::HelperPhase::Finished {
            result: crate::HelperResult::Completed
        }
    );
    assert!(adapter
        .prefix
        .to_string_lossy()
        .contains("wine-repair-prefixes"));
    assert_eq!(
        state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .intent,
        installed
    );
}
