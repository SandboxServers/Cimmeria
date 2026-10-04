use super::*;
use cimmeria_runtime_probe::prerequisite::package::physx_msi;
#[tokio::test]
async fn refused_helper_never_advances_admission_or_claims_prefix() {
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_runtime([7; 32]);
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_runtime_setup(id, revision, installed.operation_id, [7; 32], [9; 32])
        .unwrap()
        .plan;
    let state = Arc::new(Mutex::new(state));
    assert!(dispatch(state.clone(), id, root.path().join("missing.exe")).is_err());
    let state = state.lock().unwrap();
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::Starting
    );
    assert!(state.runtime_record().unwrap().is_none());
    assert!(!plan.prefix_directory(state.state_root()).exists());
}
#[test]
fn prefix_identity_locks_and_canonical_parents_refuse_aliases() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    let marker = canonical.join("owner.json");
    std::fs::write(&marker, b"{}").unwrap();
    let guard = prefix::lock_owner(&marker).unwrap();
    assert!(prefix::lock_owner(&marker).is_err());
    let alias = canonical.join("alias");
    std::os::unix::fs::symlink(&canonical, &alias).unwrap();
    assert!(prefix::directory(&alias).is_err());
    let file_alias = canonical.join("file-alias");
    std::os::unix::fs::symlink(&marker, &file_alias).unwrap();
    assert!(prefix::lock_owner(&file_alias).is_err());
    drop(guard);
    assert!(prefix::lock_owner(&marker).is_ok());
}
#[tokio::test]
#[ignore = "requires original SGW files and native Windows prerequisite worker; provisions owned headless Wine"]
async fn retained_coordinator_installs_checks_and_persists_owned_runtime() {
    let helper = PathBuf::from(std::env::var_os("SGW_PREREQUISITE_WORKER").expect("worker"));
    let digest = std::env::var("SGW_PREREQUISITE_WORKER_SHA256").expect("worker digest");
    let resource = HelperResource::open(helper.clone(), &digest).unwrap();
    let ExtractionBackend::Wine {
        runtime_sha256,
        helper_sha256,
    } = resource.backend()
    else {
        unreachable!()
    };
    let (root, mut state, installed) =
        crate::runtime_setup::tests::fixture_with_runtime(runtime_sha256);
    let original =
        PathBuf::from(std::env::var_os("SGW_PROBE_BINARIES").expect("original SGW binaries"));
    let game = installed.destination.join("game");
    for (name, digest) in [
        (
            "SGW.exe",
            "b25adf3880256c6a6bab31594c0879c411ec0005ea4b7260aef991eaf8947e31",
        ),
        (
            "PhysXLoader.dll",
            "863e3ec87198bf1a5d5638a20695529dacc9460b0939f2579fe7a7faad2af924",
        ),
    ] {
        let bytes = std::fs::read(original.join(name)).unwrap();
        assert_eq!(hex(&Sha256::digest(&bytes)), digest);
        std::fs::write(game.join("Working/Binaries").join(name), bytes).unwrap();
    }
    let package = PathBuf::from(std::env::var_os("SGW_PHYSX_INSTALLER").expect("vendor package"));
    let bytes = std::fs::read(package).unwrap();
    physx_msi(&bytes).unwrap();
    let packages = game.join(".cimmeria-prerequisites/PhysX");
    std::fs::create_dir_all(&packages).unwrap();
    std::fs::write(packages.join("PhysX_7.11.13_SystemSoftware.exe"), bytes).unwrap();
    mac_runtime::prepare(
        state.state_root().join("runtimes"),
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .await
    .unwrap();
    let id = Uuid::new_v4();
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_runtime_setup(
            id,
            revision,
            installed.operation_id,
            runtime_sha256,
            helper_sha256,
        )
        .unwrap()
        .plan;
    let state_root = state.state_root().to_path_buf();
    let state = Arc::new(Mutex::new(state));
    let worker = dispatch(state.clone(), id, helper.clone()).unwrap();
    assert!(dispatch(state.clone(), id, helper).is_err());
    let interrupted_snapshot = state.lock().unwrap().operations().snapshot().clone();
    let mut result = worker.result.clone();
    drop(worker); // The task survives UI/observer ownership loss.
    tokio::time::timeout(std::time::Duration::from_secs(240), async {
        while result.borrow().is_none() {
            result.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(*result.borrow(), Some(Outcome::PrerequisitesVerified));
    let prefix_root = plan.prefix_directory(&state_root);
    let marker = prefix::lock_owner(&prefix_root.join("owner.json")).unwrap();
    assert_eq!(prefix::read_owner::<Plan>(&marker).unwrap(), plan);
    assert!(prefix_root.join("bottle/system.reg").exists());
    assert!(!state_root.join("wine-prefixes").exists());
    drop(state);
    let mut state = DesktopState::open(&state_root).unwrap();
    assert_eq!(
        state.runtime_record().unwrap().unwrap().phase,
        crate::runtime_setup::Phase::Quiescent
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
        state.installed_content().unwrap().unwrap().intent,
        installed
    );
    assert!(!state.preferences().launcher_summary_consent);
    drop(marker);
    drop(state);
    // Preserve actual worker evidence but restore the preterminal operation
    // snapshot to reproduce the quiescent-record/terminal-commit crash boundary.
    std::fs::write(
        state_root.join("operation.json"),
        serde_json::to_vec(&interrupted_snapshot).unwrap(),
    )
    .unwrap();
    let state = Arc::new(Mutex::new(DesktopState::open(&state_root).unwrap()));
    let revision = state.lock().unwrap().operations().snapshot().revision;
    assert_eq!(
        state
            .lock()
            .unwrap()
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        OperationState::ReconciliationRequired
    );
    assert!(reconcile(state.clone(), id, revision - 1).await.is_err());
    assert_eq!(
        reconcile(state.clone(), id, revision).await.unwrap(),
        Outcome::PrerequisitesVerified
    );
    assert_eq!(
        state
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
    assert!(!state.lock().unwrap().preferences().launcher_summary_consent);
    drop(state);
    drop(root);
}
