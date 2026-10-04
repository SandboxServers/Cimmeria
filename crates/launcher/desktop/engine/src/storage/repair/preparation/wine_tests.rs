use super::*;
use crate::mac_wine::HelperResource;
use sha2::{Digest, Sha256};
fn fixture(
    helper: &HelperResource,
    seed: &[u8],
) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Plan) {
    let ExtractionBackend::Wine {
        runtime_sha256,
        helper_sha256,
    } = helper.backend()
    else {
        unreachable!()
    };
    let hash = digest(seed);
    let (root, mut state, installed) = crate::runtime_setup::tests::fixture_with_backend(
        runtime_sha256,
        helper_sha256,
        &hash,
        seed.len(),
    );
    let revision = state.operations().snapshot().revision;
    let plan = state
        .admit_repair(Uuid::new_v4(), revision, installed.operation_id, true)
        .unwrap()
        .plan;
    (root, Arc::new(Mutex::new(state)), plan)
}
#[tokio::test]
async fn missing_resource_refuses_dispatch_and_precancel_never_downloads_or_claims() {
    let bundle = tempfile::tempdir().unwrap();
    let path = bundle.path().join("helper.exe");
    std::fs::write(&path, b"inert fixture").unwrap();
    let helper = HelperResource::open(path.clone(), &digest(b"inert fixture")).unwrap();
    let (root, state, plan) = fixture(&helper, b"seed");
    std::fs::remove_file(&path).unwrap();
    assert!(prepare_wine(state.clone(), plan.id, helper.clone()).is_err());
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
        OperationState::Starting
    );
    std::fs::write(&path, b"inert fixture").unwrap();
    let work = prepare_wine(state.clone(), plan.id, helper.clone()).unwrap();
    assert!(prepare_wine(state.clone(), plan.id, helper).is_err());
    work.request_cancel().unwrap();
    assert!(matches!(
        work.result.await.unwrap(),
        Err(Failure::Cancelled)
    ));
    assert!(!plan.work_directory().exists());
    assert!(!root.path().join("state/runtimes").exists());
    assert!(!root.path().join("state/wine-repair-prefixes").exists());
    assert_eq!(
        std::fs::read(
            plan.installation
                .destination
                .join("game/Working/Binaries/SGW.exe")
        )
        .unwrap(),
        b"inert fixture"
    );
}
#[tokio::test]
#[ignore = "downloads pinned Wine and reconstructs/commits a signed ZIP through the Windows-native helper headlessly"]
async fn retained_wine_repair_reconstructs_and_commits_under_all_ownership_locks() {
    use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
    let helper = HelperResource::open(
        PathBuf::from(
            std::env::var_os("CIMMERIA_WINE_HELPER").expect("Windows-native helper artifact"),
        ),
        "d0c89fad444cb4dc6478f1db8a5e62bc54d5696ee2a84a63d740bf3a5b92c6a3",
    )
    .unwrap();
    let seed = install_worker::tests::archive(true);
    let (root, state, plan) = fixture(&helper, &seed);
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .mount(&server)
        .await;
    let preparation = start_with_backend(
        state.clone(),
        plan.id,
        reqwest::Client::new(),
        format!("{}/manifest.json", server.uri()),
        Backend::Wine(helper),
    )
    .unwrap();
    let prepared = tokio::time::timeout(Duration::from_secs(180), preparation.result)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(prepared.wine.is_some());
    assert!(lock_owner(&plan.installation).is_err());
    let prefix_owner = root
        .path()
        .join("state/wine-repair-prefixes")
        .join(plan.id.to_string())
        .join("owner.json");
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&prefix_owner)
        .unwrap();
    assert!(marker.try_lock().is_err());
    drop(marker);
    assert_eq!(
        std::fs::read(plan.stage().join("later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(
        std::fs::read(
            plan.installation
                .destination
                .join("game/Working/Binaries/SGW.exe")
        )
        .unwrap(),
        b"inert fixture"
    );
    let result = commit::commit_wine(state.clone(), prepared)
        .unwrap()
        .await
        .unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(
        std::fs::read(plan.installation.destination.join("game/later.txt")).unwrap(),
        b"later entry"
    );
    assert_eq!(
        std::fs::read(plan.backup().join("Working/Binaries/SGW.exe")).unwrap(),
        b"inert fixture"
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
    assert_eq!(
        state
            .lock()
            .unwrap()
            .installed_content()
            .unwrap()
            .unwrap()
            .intent,
        plan.installation
    );
    assert!(lock_owner(&plan.installation).is_ok());
    assert!(OpenOptions::new()
        .read(true)
        .write(true)
        .open(prefix_owner)
        .unwrap()
        .try_lock()
        .is_ok());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
