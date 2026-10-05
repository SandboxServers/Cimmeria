use super::*;
use crate::client_setup::login_servers::default_servers;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

pub(crate) use super::fixtures::{archive, verified};
pub(super) fn setup(
    release: &VerifiedRelease,
) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Uuid) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    state
        .admit_install(id, 0, 1, release, default_servers())
        .unwrap();
    (root, Arc::new(Mutex::new(state)), id)
}
pub(super) async fn outcome(mut receiver: watch::Receiver<Option<Outcome>>) -> Outcome {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(result) = *receiver.borrow_and_update() {
                return result;
            }
            receiver.changed().await.unwrap();
        }
    })
    .await
    .expect("worker finishes within fixture deadline")
}
async fn requested(server: &MockServer) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !server.received_requests().await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn dispatch_installs_real_zip_and_persists_success_after_observer_drop() {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(seed)
                .set_delay(Duration::from_millis(50)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert!(dispatch_with(
        state.clone(),
        id,
        verified(&archive(true)),
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new()
    )
    .is_err());
    let results = worker.result.clone();
    drop(worker); // Dropping a view/observer is not cancellation.
    assert_eq!(outcome(results).await, Outcome::ContentPrepared);
    let content = root.path().join("install/game");
    assert!(content.join("Working/Binaries/SGW.exe").is_file());
    assert!(crate::client_setup::login_servers::path(&content).is_file());
    assert!(root.path().join("install/content-ready.json").is_file());
    assert_eq!(
        state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .intent
            .operation_id,
        id
    );
    assert!(!root
        .path()
        .join(format!("install/.cimmeria-stage-{id}"))
        .exists());
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
    server.verify().await;
}

#[tokio::test]
async fn legacy_success_without_executable_is_not_promoted() {
    let seed = archive(false);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentInvalid
    );
    assert!(!root.path().join("install/game").exists());
    assert!(root
        .path()
        .join(format!(
            "install/.cimmeria-stage-{id}/launcher-installed.json"
        ))
        .is_file());
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
        OperationState::Failed
    );
}

#[tokio::test]
async fn cancellation_is_committed_and_download_is_stopped_without_promotion() {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(seed)
                .set_delay(Duration::from_secs(10)),
        )
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    requested(&server).await;
    assert_eq!(
        worker.request_cancel().unwrap().operation.unwrap().state,
        OperationState::CancelRequested
    );
    assert_eq!(outcome(worker.result.clone()).await, Outcome::Cancelled);
    assert!(!root.path().join("install/game").exists());
    assert!(root.path().join("install/.cimmeria-install.json").is_file());
}

#[tokio::test]
async fn destination_changed_after_admission_is_preserved_without_requests() {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    std::fs::create_dir(root.path().join("install")).unwrap();
    std::fs::write(root.path().join("install/user.txt"), b"keep").unwrap();
    let server = MockServer::start().await;
    let worker = dispatch_with(
        state,
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::DestinationUnavailable
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(
        std::fs::read(root.path().join("install/user.txt")).unwrap(),
        b"keep"
    );
}

#[test]
fn receipt_failure_after_promotion_requires_reconciliation() {
    let release = verified(&archive(true));
    let (root, state, id) = setup(&release);
    let intent = {
        let mut state = state.lock().unwrap();
        state
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        state.install_intent().unwrap().unwrap()
    };
    let stage = root.path().join("install/stage");
    std::fs::create_dir_all(&stage).unwrap();
    std::fs::write(stage.join("fixture"), b"prepared").unwrap();
    std::fs::create_dir(root.path().join("install/content-ready.json")).unwrap();
    let result = publish(&state, id, promote(&intent, &stage));
    assert_eq!(result, Outcome::ReconciliationRequired);
    assert!(root.path().join("install/game/fixture").is_file());
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
}

#[tokio::test]
async fn extraction_checkpoint_cancellation_persists_cancelled_and_retains_partial_stage() {
    struct CancelAtExtraction {
        state: Arc<Mutex<DesktopState>>,
        id: Uuid,
        cancel: CancellationToken,
    }
    impl crate::install_progress::ProgressReporter for CancelAtExtraction {
        fn report(&self, progress: install::Progress) {
            if matches!(progress, install::Progress::Extracting { .. }) {
                self.state
                    .lock()
                    .unwrap()
                    .operations_mut()
                    .unwrap()
                    .request_cancel(self.id)
                    .unwrap();
                self.cancel.cancel();
            }
        }
    }
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let intent = {
        let mut state = state.lock().unwrap();
        state
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        state.install_intent().unwrap().unwrap()
    };
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let cancel = CancellationToken::new();
    let progress = ProgressSink::Checkpoint(Arc::new(CancelAtExtraction {
        state: state.clone(),
        id,
        cancel: cancel.clone(),
    }));
    let result = install_owned(
        &intent,
        &root.path().join("state"),
        &release,
        &format!("{}/manifest.json", server.uri()),
        &reqwest::Client::new(),
        cancel,
        progress,
    )
    .await;
    assert_eq!(publish(&state, id, result), Outcome::Cancelled);
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
        OperationState::Cancelled
    );
    let stage = root.path().join(format!("install/.cimmeria-stage-{id}"));
    assert!(stage.join("Working/Binaries/SGW.exe").is_file());
    assert!(!stage.join("later.txt").exists());
    assert!(!root.path().join("install/game").exists());
}

#[cfg(unix)]
#[test]
fn redirected_parent_after_admission_is_not_claimed() {
    let root = tempfile::tempdir().unwrap();
    for directory in ["parent", "other"] {
        std::fs::create_dir(root.path().join(directory)).unwrap();
    }
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(Some(root.path().join("parent/install")), false, 0)
        .unwrap();
    let release = verified(&archive(true));
    let intent = state
        .admit_install(Uuid::new_v4(), 0, 1, &release, default_servers())
        .unwrap()
        .intent;
    std::fs::rename(
        root.path().join("parent"),
        root.path().join("previous-parent"),
    )
    .unwrap();
    std::os::unix::fs::symlink(root.path().join("other"), root.path().join("parent")).unwrap();
    assert!(matches!(
        claim(&intent, &root.path().join("state")),
        Err(StorageError::InvalidDirectory)
    ));
    assert_eq!(
        std::fs::read_dir(root.path().join("other"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn result_write_failure_preserves_recovery_gate() {
    let release = verified(&archive(true));
    let (root, state, id) = setup(&release);
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    // A non-regular result destination deterministically prevents publication.
    std::fs::create_dir(root.path().join("state/install-result.json")).unwrap();
    assert_eq!(
        publish(&state, id, Outcome::InstallFailed),
        Outcome::ReconciliationRequired
    );
    let state = state.lock().unwrap();
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
    assert_eq!(state.install_outcome(), Ok(None));
}

#[test]
fn installation_reference_write_failure_cannot_publish_success() {
    let release = verified(&archive(true));
    let (root, state, id) = setup(&release);
    let intent = {
        let mut state = state.lock().unwrap();
        state
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        state.install_intent().unwrap().unwrap()
    };
    std::fs::create_dir_all(intent.destination.join("game")).unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    std::fs::create_dir(root.path().join("state/installed-content.json")).unwrap();
    assert_eq!(
        publish(&state, id, Outcome::ContentPrepared),
        Outcome::ReconciliationRequired
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
        OperationState::ReconciliationRequired
    );
    assert!(!root.path().join("state/install-result.json").exists());
}

/// Shared real ZIP installation fixture for repair reconstruction tests.
pub(crate) async fn prepared_fixture() -> (
    tempfile::TempDir,
    Arc<Mutex<DesktopState>>,
    Uuid,
    MockServer,
) {
    let seed = archive(true);
    let release = verified(&seed);
    let (root, state, id) = setup(&release);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentPrepared
    );
    (root, state, id, server)
}
