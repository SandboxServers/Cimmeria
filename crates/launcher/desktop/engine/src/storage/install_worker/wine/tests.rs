use super::*;
use crate::{AdmissionRequest, ExtractionBackend};
use sha2::{Digest, Sha256};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

fn setup(
    release: &VerifiedRelease,
    helper_hash: [u8; 32],
) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Uuid) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    state
        .save_preferences(
            Some(root.path().canonicalize().unwrap().join("install")),
            false,
            0,
        )
        .unwrap();
    let hash = crate::mac_runtime::ARCHIVE_SHA256
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let id = Uuid::new_v4();
    state
        .admit_install_backend(AdmissionRequest {
            id,
            operation_revision: 0,
            preferences_revision: 1,
            release,
            login_servers: crate::client_setup::login_servers::default_servers(),
            backend: ExtractionBackend::Wine {
                runtime_sha256: hash.try_into().unwrap(),
                helper_sha256: helper_hash,
            },
        })
        .unwrap();
    (root, Arc::new(Mutex::new(state)), id)
}
#[tokio::test]
async fn missing_or_changed_helper_cannot_transition_or_claim_destination() {
    let seed = super::super::tests::archive(true);
    let release = super::super::tests::verified(&seed);
    let (root, state, id) = setup(&release, [0; 32]);
    let before = state.lock().unwrap().operations().snapshot().clone();
    let helper = root.path().join("helper.exe");
    assert!(dispatch_wine(state.clone(), id, release, helper.clone()).is_err());
    std::fs::write(&helper, b"not the expected helper").unwrap();
    assert!(dispatch_wine(
        state.clone(),
        id,
        super::super::tests::verified(&seed),
        helper
    )
    .is_err());
    assert_eq!(&before, state.lock().unwrap().operations().snapshot());
    assert!(!root.path().join("install").exists());
    assert!(!root.path().join("state/runtimes").exists());
}
#[test]
fn preparation_errors_keep_cancellation_and_uncertainty_distinct() {
    assert_eq!(
        preparation_outcome(WineError::RosettaRequired),
        Outcome::RosettaRequired
    );
    assert_eq!(
        preparation_outcome(WineError::Runtime(RuntimeError::Cancelled)),
        Outcome::Cancelled
    );
    assert_eq!(
        preparation_outcome(WineError::Runtime(RuntimeError::Verification)),
        Outcome::RuntimeUnavailable
    );
    assert_eq!(
        preparation_outcome(WineError::Invalid),
        Outcome::ReconciliationRequired
    );
}

#[tokio::test]
#[ignore = "requires Windows-native helper; provisions pinned Wine and exercises retained install worker headlessly"]
async fn retained_wine_worker_prepares_content_after_observer_drop() {
    let helper =
        PathBuf::from(std::env::var_os("CIMMERIA_WINE_HELPER").expect("set native-built helper"));
    let seed = super::super::tests::archive(true);
    use ed25519_dalek::{Signer, SigningKey};
    let mut patch = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    patch
        .start_file("native-patch.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    patch
        .write_all(b"patched natively after Wine seed")
        .unwrap();
    let patch = patch.finish().unwrap().into_inner();
    let hash = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"seed":{"blob":"seed.zip","size":seed.len(),"sha256":hash(&seed)},
        "patches":[{"id":"native","blob":"patch.zip","size":patch.len(),"sha256":hash(&patch)}]})).unwrap();
    let signature = SigningKey::from_bytes(&[0x2a; 32])
        .sign(&body)
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let release = crate::catalog::verify_release(&body, signature.as_bytes()).unwrap();
    // Fixture admission binds this test artifact. Production admission instead
    // uses the packaged build's independently pinned helper identity.
    let (root, state, id) = setup(
        &release,
        Sha256::digest(std::fs::read(&helper).unwrap()).into(),
    );
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/patch.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(patch))
        .expect(1)
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        helper.clone(),
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    let original = state.lock().unwrap().cached_install_release().unwrap();
    assert!(
        dispatch_wine(state.clone(), id, original, helper).is_err(),
        "duplicate cannot dispatch a second worker"
    );
    let mut result = worker.result.clone();
    drop(worker); // UI/host observer loss must not abort native mutation.
    let outcome = tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            if let Some(value) = *result.borrow_and_update() {
                break value;
            }
            result.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(outcome, Outcome::ContentPrepared);
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
    assert!(root.path().join("install/content-ready.json").is_file());
    assert!(root
        .path()
        .join("install/game/Working/Binaries/SGW.exe")
        .is_file());
    assert!(!root
        .path()
        .join(format!("install/.cimmeria-stage-{id}"))
        .exists());
    assert_eq!(
        std::fs::read(root.path().join("install/game/native-patch.txt")).unwrap(),
        b"patched natively after Wine seed"
    );
    server.verify().await;
}

#[tokio::test]
#[ignore = "requires Windows-native helper; verifies cancellation before runtime download"]
async fn retained_wine_worker_cancels_before_runtime_provisioning() {
    let helper =
        PathBuf::from(std::env::var_os("CIMMERIA_WINE_HELPER").expect("set native-built helper"));
    let seed = super::super::tests::archive(true);
    let release = super::super::tests::verified(&seed);
    let (root, state, id) = setup(
        &release,
        Sha256::digest(std::fs::read(&helper).unwrap()).into(),
    );
    let server = MockServer::start().await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        helper,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    worker.request_cancel().unwrap();
    let result = super::super::tests::outcome(worker.result.clone()).await;
    assert_eq!(result, Outcome::Cancelled);
    assert!(state.lock().unwrap().helper_record(id).unwrap().is_none());
    assert!(!root.path().join("state/runtimes").exists());
    assert!(!root.path().join("state/wine-prefixes").exists());
    assert!(!root.path().join("install/content-ready.json").exists());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
#[ignore = "requires Windows-native helper; verifies explicit cleanup after real Wine extraction fails content validation"]
async fn failed_wine_content_can_be_cleaned_for_a_new_attempt() {
    let helper =
        PathBuf::from(std::env::var_os("CIMMERIA_WINE_HELPER").expect("set native-built helper"));
    let seed = super::super::tests::archive(false);
    let release = super::super::tests::verified(&seed);
    let (root, state, id) = setup(
        &release,
        Sha256::digest(std::fs::read(&helper).unwrap()).into(),
    );
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(seed))
        .expect(1)
        .mount(&server)
        .await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        helper,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    let mut result = worker.result.clone();
    let outcome = tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            if let Some(value) = *result.borrow_and_update() {
                break value;
            }
            result.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        outcome,
        Outcome::ContentInvalid | Outcome::InstallFailed
    ));
    drop(worker);
    let clone = state.clone();
    tokio::task::spawn_blocking(move || {
        let mut state = clone.lock().unwrap();
        assert_eq!(
            state.helper_record(id).unwrap().unwrap().phase,
            crate::HelperPhase::Finished {
                result: crate::HelperResult::Completed
            }
        );
        let revision = state.operations().snapshot().revision;
        assert!(!state.can_retry_install());
        state.clean_failed_install(id, revision).unwrap();
        assert!(state.can_retry_install());
        assert!(!state.preferences().launcher_summary_consent);
    })
    .await
    .unwrap();
    assert_eq!(
        std::fs::read_dir(root.path().join("install"))
            .unwrap()
            .count(),
        0
    );
    server.verify().await;
}

#[tokio::test]
#[ignore = "real signed release: requires original archive, release public key and Windows-native helper; downloads real patches"]
async fn original_signed_release_prepares_patched_content() {
    let source = PathBuf::from(std::env::var_os("SGW_CLIENT_RAR").expect("set original archive"));
    let manifest =
        PathBuf::from(std::env::var_os("CIMMERIA_SMOKE_MANIFEST").expect("set signed manifest"));
    let release = crate::catalog::verify_release(
        &std::fs::read(&manifest).unwrap(),
        &std::fs::read(manifest.with_extension("json.sig")).unwrap(),
    )
    .unwrap();
    let helper =
        PathBuf::from(std::env::var_os("CIMMERIA_WINE_HELPER").expect("set native helper"));
    let (root, state, id) = setup(
        &release,
        Sha256::digest(std::fs::read(&helper).unwrap()).into(),
    );
    let (intent, state_root) = {
        let mut owner = state.lock().unwrap();
        owner
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        (
            owner.install_intent().unwrap().unwrap(),
            owner.directory.root.clone(),
        )
    };
    // Use production ownership and preparation functions, preloading only the
    // already-authenticated seed cache to avoid a second multi-GB download.
    let _ownership = claim(&intent, &state_root).unwrap();
    let cache = intent.destination.join(format!(".cimmeria-cache-{id}"));
    std::fs::create_dir(&cache).unwrap();
    let archive = cache.join(format!(
        ".tmp-seed-{}.download",
        &release.manifest().seed.sha256[..12]
    ));
    std::fs::copy(source, &archive).unwrap();
    let http = download_client().unwrap();
    let outcome = install_claimed(
        &state,
        &intent,
        &release,
        crate::catalog::URL,
        &http,
        helper,
        CancellationToken::new(),
        ProgressSink::latest().0,
    )
    .await;
    assert_eq!(outcome, Outcome::ContentPrepared);
    assert_eq!(publish(&state, id, outcome), Outcome::ContentPrepared);
    assert!(root.path().join("install/content-ready.json").exists());
    assert!(content_valid(&root.path().join("install/game"), &release));
    // Independently measured from the authenticated original seed. This makes
    // an older helper that discards vendor installers fail the real smoke.
    let prerequisites = root.path().join("install/game/.cimmeria-prerequisites");
    for (path, expected) in [
        (
            "DOTNETC/NetFx20SP1_x86.exe",
            "c36c3a1d074de32d53f371c665243196a7608652a2fc6be9520312d5ce560871",
        ),
        (
            "DOTNETC/vcredist_x86.exe",
            "eb00f891919d4f894ab725b158459db8834470c382dc60cd3c3ee2c6de6da92c",
        ),
        (
            "DX9.0c/DXSETUP.exe",
            "ea13ab4b4f9ae747d7dc8c96e0be8d58568bed87c478bf3dffcaba9e95de1166",
        ),
        (
            "PhysX/PhysX_7.11.13_SystemSoftware.exe",
            "920d5e09e6ba0a92342271c18c67472461813424d70b5c0b981b6f13b129fbf6",
        ),
    ] {
        assert_eq!(
            Sha256::digest(std::fs::read(prerequisites.join(path)).unwrap())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            expected,
            "retained {path}"
        );
    }
    assert!(!state.lock().unwrap().preferences().launcher_summary_consent);
    let removal_state = state.clone();
    tokio::task::spawn_blocking(move || {
        let mut owner = removal_state.lock().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner.uninstall(Uuid::new_v4(), revision, id, true).unwrap();
        assert!(owner.installed_content().unwrap().is_none());
        assert!(!owner.preferences().launcher_summary_consent);
    })
    .await
    .unwrap();
    assert!(!root.path().join("install").exists());
    eprintln!(
        "Authenticated real seed and {} patches prepared then explicitly uninstalled; runtime/gameplay not tested",
        release.manifest().patches.len()
    );
}
