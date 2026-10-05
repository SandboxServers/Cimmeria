use super::super::tests::{archive, outcome, setup, verified};
use super::*;
use wiremock::{
    matchers::{header, path},
    Mock, MockServer, ResponseTemplate,
};

fn interrupted(seed: &[u8]) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Uuid, PathBuf) {
    let release = verified(seed);
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
    let guard = claim(&intent, &root.path().join("state")).unwrap();
    let stage = intent.destination.join(format!(".cimmeria-stage-{id}"));
    std::fs::create_dir(&stage).unwrap();
    let partial = stage.join(format!(
        ".tmp-seed-{}.download",
        &release.manifest().seed.sha256[..12]
    ));
    std::fs::write(partial, &seed[..8]).unwrap();
    drop(guard);
    drop(state);
    let state = Arc::new(Mutex::new(
        DesktopState::open(&root.path().join("state")).unwrap(),
    ));
    (root, state, id, stage)
}
fn revision(state: &Mutex<DesktopState>) -> u64 {
    state.lock().unwrap().operations().snapshot().revision
}

#[tokio::test]
async fn explicit_resume_uses_saved_signed_release_and_http_range_then_promotes() {
    let seed = archive(true);
    let (root, state, id, stage) = interrupted(&seed);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .and(header("range", "bytes=8-"))
        .respond_with(ResponseTemplate::new(206).set_body_bytes(seed[8..].to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let previous = revision(&state);
    let worker = resume_with(
        state.clone(),
        id,
        previous,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert!(matches!(
        resume_with(
            state.clone(),
            id,
            previous,
            format!("{}/manifest.json", server.uri()),
            reqwest::Client::new()
        ),
        Err(ResumeError::Intent(IntentError::Operation(
            ContractError::StaleRevision
        )))
    ));
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentPrepared
    );
    assert!(!stage.exists());
    assert!(root
        .path()
        .join("install/game/Working/Binaries/SGW.exe")
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
        OperationState::Succeeded
    );
    server.verify().await;
}

#[tokio::test]
async fn corrupt_ledger_is_preserved_and_does_not_dispatch() {
    let seed = archive(true);
    let (_root, state, id, stage) = interrupted(&seed);
    std::fs::write(stage.join("launcher-installed.json"), b"not json").unwrap();
    let previous = revision(&state);
    let error = resume_with(
        state.clone(),
        id,
        previous,
        "http://127.0.0.1:1/manifest.json".into(),
        reqwest::Client::new(),
    );
    assert!(matches!(
        error,
        Err(ResumeError::Intent(IntentError::Storage(
            StorageError::Corrupt
        )))
    ));
    assert_eq!(revision(&state), previous);
    assert_eq!(
        std::fs::read(stage.join("launcher-installed.json")).unwrap(),
        b"not json"
    );
}

#[tokio::test]
async fn live_owner_or_promoted_content_prevents_resume() {
    let seed = archive(true);
    let (root, state, id, _stage) = interrupted(&seed);
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("install/.cimmeria-install.json"))
        .unwrap();
    marker.lock().unwrap();
    let previous = revision(&state);
    assert!(matches!(
        resume_with(
            state.clone(),
            id,
            previous,
            "http://127.0.0.1:1/m".into(),
            reqwest::Client::new()
        ),
        Err(ResumeError::Intent(IntentError::Storage(
            StorageError::InUse
        )))
    ));
    marker.unlock().unwrap();
    std::fs::create_dir(root.path().join("install/game")).unwrap();
    assert!(matches!(
        resume_with(
            state.clone(),
            id,
            previous,
            "http://127.0.0.1:1/m".into(),
            reqwest::Client::new()
        ),
        Err(ResumeError::Intent(IntentError::Storage(
            StorageError::Busy
        )))
    ));
    assert_eq!(revision(&state), previous);
}

#[cfg(unix)]
#[tokio::test]
async fn resume_refuses_symlinks_inside_partial_output() {
    let seed = archive(true);
    let (root, state, id, stage) = interrupted(&seed);
    let outside = root.path().join("outside");
    std::fs::write(&outside, b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, stage.join("link")).unwrap();
    let previous = revision(&state);
    assert!(matches!(
        resume_with(
            state.clone(),
            id,
            previous,
            "http://127.0.0.1:1/m".into(),
            reqwest::Client::new()
        ),
        Err(ResumeError::Intent(IntentError::Storage(
            StorageError::UnsafeFile
        )))
    ));
    assert_eq!(revision(&state), previous);
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep");
}

#[tokio::test]
async fn interruption_before_stage_creation_resumes_with_full_download() {
    let seed = archive(true);
    let (_root, state, id, stage) = interrupted(&seed);
    std::fs::remove_dir_all(stage).unwrap();
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .expect(1)
        .mount(&server)
        .await;
    let previous = revision(&state);
    let worker = resume_with(
        state,
        id,
        previous,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentPrepared
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("range"));
}

#[tokio::test]
async fn failed_running_commit_does_not_dispatch_or_retain_owner_lock() {
    let seed = archive(true);
    let (root, state, id, stage) = interrupted(&seed);
    let journal = root.path().join("state/operation.json");
    let original = std::fs::read(&journal).unwrap();
    std::fs::remove_file(&journal).unwrap();
    std::fs::create_dir(&journal).unwrap(); // Force atomic replacement validation to fail.
    let previous = revision(&state);
    let server = MockServer::start().await;
    assert!(matches!(
        resume_with(
            state.clone(),
            id,
            previous,
            format!("{}/manifest.json", server.uri()),
            reqwest::Client::new()
        ),
        Err(ResumeError::Intent(IntentError::Operation(
            ContractError::PersistenceFailed
        )))
    ));
    assert_eq!(revision(&state), previous);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(std::fs::read_dir(stage).unwrap().count(), 1);
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.path().join("install/.cimmeria-install.json"))
        .unwrap();
    marker.try_lock().unwrap();
    marker.unlock().unwrap();
    // Restore the injected obstruction and prove the original recovery state survived.
    std::fs::remove_dir(&journal).unwrap();
    std::fs::write(&journal, original).unwrap();
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
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
