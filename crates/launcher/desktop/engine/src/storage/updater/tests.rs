use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign::KeyPair;
use reqwest::Url;
use std::io::Cursor;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
const PAYLOAD: &[u8] = b"inert signed launcher fixture, never executable";
fn fixture(version: &str) -> (Config, Offer) {
    let keys = KeyPair::generate_unencrypted_keypair().unwrap();
    let signature = minisign::sign(
        Some(&keys.pk),
        &keys.sk,
        Cursor::new(PAYLOAD),
        Some(&format!("timestamp:1\tfile:fixture\tversion:{version}")),
        None,
    )
    .unwrap();
    let config = Config::new(
        "https://updates.example.test/feed",
        &STANDARD.encode(keys.pk.to_box().unwrap().to_string()),
        "1.0.0",
        "darwin-aarch64",
        vec!["updates.example.test".into()],
    )
    .unwrap();
    let offer = Offer {
        id: Uuid::new_v4(),
        version: version.into(),
        notes: "Fixture release".into(),
        url: "https://updates.example.test/fixture".into(),
        signature: STANDARD.encode(signature.to_string()),
    };
    (config, offer)
}
fn available(state: &mut DesktopState, config: &Config, offer: Offer) -> Snapshot {
    let before = state.launcher_update_snapshot(Some(config)).unwrap();
    let ticket = state
        .begin_launcher_update_check(Some(config), before.revision, before.operation_revision)
        .unwrap();
    state
        .finish_launcher_update_check(ticket, Ok(Some(offer)))
        .unwrap();
    state.launcher_update_snapshot(Some(config)).unwrap()
}
fn prepare(
    state: &mut DesktopState,
    config: &Config,
    snapshot: Snapshot,
    bytes: &[u8],
) -> Snapshot {
    let mut ticket = state
        .begin_launcher_update_prepare(
            Some(config),
            snapshot.offer.unwrap().id,
            snapshot.revision,
            snapshot.operation_revision,
        )
        .unwrap();
    state.mark_launcher_update_verifying(&mut ticket).unwrap();
    state
        .finish_launcher_update_prepare(config, ticket, Ok(bytes.to_vec()))
        .unwrap();
    state.launcher_update_snapshot(Some(config)).unwrap()
}
#[test]
fn signed_artifact_ready_reopens_and_tampered_stage_is_never_ready() {
    let (config, offer) = fixture("1.1.0");
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    let available = available(&mut state, &config, offer);
    let ready = prepare(&mut state, &config, available, PAYLOAD);
    assert_eq!(ready.phase, Phase::Ready);
    assert_eq!(state.staged_update_bytes().unwrap(), PAYLOAD);
    drop(state);
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state.launcher_update_snapshot(Some(&config)).unwrap().phase,
        Phase::Ready
    );
    std::fs::write(root.path().join(ARTIFACT), b"tampered").unwrap();
    let snapshot = state.launcher_update_snapshot(Some(&config)).unwrap();
    assert_eq!(snapshot.phase, Phase::Failed);
    assert_eq!(snapshot.failure, Some(Error::Signature));
}
#[test]
fn signed_old_artifact_cannot_be_relabelled_as_new_and_tampered_payload_fails() {
    let (config, mut offer) = fixture("0.9.0");
    offer.version = "9.0.0".into();
    assert_eq!(config.verify(&offer, PAYLOAD), Err(Error::SignedVersion));
    assert_eq!(config.verify(&offer, b"tampered"), Err(Error::Signature));
    offer.version = "0.9.0".into();
    assert_eq!(config.verify(&offer, PAYLOAD), Err(Error::NotNewer));
    // Altering the trusted comment without re-signing fails the global signature.
    let sig = String::from_utf8(STANDARD.decode(&offer.signature).unwrap()).unwrap();
    offer.signature = STANDARD.encode(sig.replace("version:0.9.0", "version:9.0.0"));
    offer.version = "9.0.0".into();
    assert_eq!(config.verify(&offer, PAYLOAD), Err(Error::Signature));
}
#[test]
fn unsigned_version_and_duplicate_fields_fail_closed() {
    let keys = KeyPair::generate_unencrypted_keypair().unwrap();
    let (mut config, mut offer) = fixture("1.1.0");
    config.public_key = STANDARD.encode(keys.pk.to_box().unwrap().to_string());
    for comment in ["timestamp:1\tfile:fixture", "version:1.1.0\tversion:1.1.0"] {
        offer.signature = STANDARD.encode(
            minisign::sign(
                Some(&keys.pk),
                &keys.sk,
                Cursor::new(PAYLOAD),
                Some(comment),
                None,
            )
            .unwrap()
            .to_string(),
        );
        assert_eq!(config.verify(&offer, PAYLOAD), Err(Error::SignedVersion));
    }
}
#[test]
fn updater_and_game_mutations_are_exclusive_and_reopen_marks_interruption() {
    let (config, _) = fixture("1.1.0");
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    let ticket = state
        .begin_launcher_update_check(Some(&config), 0, 0)
        .unwrap();
    assert!(matches!(state.operations_mut(), Err(StorageError::Busy)));
    assert!(matches!(
        state.begin_launcher_update_check(Some(&config), 1, 0),
        Err(Error::Busy)
    ));
    drop(ticket);
    drop(state);
    let mut state = DesktopState::open(root.path()).unwrap();
    let snapshot = state.launcher_update_snapshot(Some(&config)).unwrap();
    assert_eq!(snapshot.phase, Phase::Failed);
    assert_eq!(snapshot.failure, Some(Error::Interrupted));
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), crate::OperationKind::Launch, [1; 32], 0)
        .unwrap();
    assert!(matches!(
        state.begin_launcher_update_check(Some(&config), snapshot.revision, 1),
        Err(Error::Busy)
    ));
}
#[test]
fn disabled_stale_revision_and_stale_offer_never_begin_work() {
    let (config, offer) = fixture("1.1.0");
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state.launcher_update_snapshot(None).unwrap().phase,
        Phase::Disabled
    );
    assert!(matches!(
        state.begin_launcher_update_check(None, 0, 0),
        Err(Error::Disabled)
    ));
    assert!(!root.path().join(RECORD).exists());
    assert!(matches!(
        state.begin_launcher_update_check(Some(&config), 1, 0),
        Err(Error::StaleRevision)
    ));
    let snapshot = available(&mut state, &config, offer);
    assert!(matches!(
        state.begin_launcher_update_prepare(Some(&config), Uuid::new_v4(), snapshot.revision, 0),
        Err(Error::StaleOffer)
    ));
    assert_eq!(
        state
            .launcher_update_snapshot(Some(&config))
            .unwrap()
            .revision,
        snapshot.revision
    );
}
#[test]
fn url_policy_rejects_host_suffix_credentials_ports_http_and_fragment() {
    let (config, _) = fixture("1.1.0");
    for url in [
        "http://updates.example.test/file",
        "https://updates.example.test.evil.test/file",
        "https://evil.test/file",
        "https://user@updates.example.test/file",
        "https://updates.example.test:444/file",
        "https://updates.example.test/file#fragment",
    ] {
        assert_eq!(
            config.allow(&Url::parse(url).unwrap()),
            Err(Error::Policy),
            "{url}"
        );
    }
}
async fn server_fixture(version: &str, payload: &[u8]) -> (MockServer, Config) {
    let server = MockServer::start().await;
    let (mut config, mut offer) = fixture(version);
    config.loopback = true;
    config.endpoint = Url::parse(&format!("{}/feed", server.uri())).unwrap();
    offer.url = format!("{}/artifact", server.uri());
    let feed = serde_json::json!({"version":version,"notes":"Fixture <script>not markup</script>","platforms":{"darwin-aarch64":{"url":offer.url,"signature":offer.signature}}});
    Mock::given(method("GET"))
        .and(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(feed))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/artifact"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(payload))
        .mount(&server)
        .await;
    (server, config)
}
#[tokio::test]
async fn actual_signed_local_feed_download_and_tampered_artifact() {
    for (payload, phase) in [(PAYLOAD, Phase::Ready), (&b"tampered"[..], Phase::Failed)] {
        let (_server, config) = server_fixture("1.1.0", payload).await;
        let root = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(root.path()).unwrap();
        let ticket = state
            .begin_launcher_update_check(Some(&config), 0, 0)
            .unwrap();
        state
            .finish_launcher_update_check(ticket, check(&config).await)
            .unwrap();
        let snapshot = state.launcher_update_snapshot(Some(&config)).unwrap();
        let ticket = state
            .begin_launcher_update_prepare(
                Some(&config),
                snapshot.offer.unwrap().id,
                snapshot.revision,
                0,
            )
            .unwrap();
        let bytes = download(&config, ticket.offer().unwrap()).await;
        state
            .finish_launcher_update_prepare(&config, ticket, bytes)
            .unwrap();
        assert_eq!(
            state.launcher_update_snapshot(Some(&config)).unwrap().phase,
            phase
        );
        assert_eq!(root.path().join(ARTIFACT).exists(), phase == Phase::Ready);
    }
}
#[tokio::test]
async fn feed_size_old_versions_and_hostile_redirect_fail_before_offering() {
    let (server, config) = server_fixture("0.9.0", PAYLOAD).await;
    assert!(check(&config).await.unwrap().is_none());
    server.reset().await;
    Mock::given(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; policy::MAX_FEED + 1]))
        .mount(&server)
        .await;
    assert!(matches!(check(&config).await, Err(Error::Size)));
    server.reset().await;
    Mock::given(path("/feed"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("Location", "http://localhost:1/private"),
        )
        .mount(&server)
        .await;
    assert!(matches!(check(&config).await, Err(Error::Transport)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "Driven by updater-native-uat.mjs over stdin/stdout"]
async fn updater_native_uat_bridge() {
    use std::io::{BufRead, Write};
    let (_server, config) = server_fixture("1.1.0", PAYLOAD).await;
    let root = tempfile::tempdir().unwrap();
    let mut state = Some(DesktopState::open(root.path()).unwrap());
    for line in std::io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result: Result<Snapshot, Error> = async {
            if value["command"] == "reopen" {
                drop(state.take());
                state = Some(DesktopState::open(root.path()).unwrap());
            }
            let state = state.as_mut().unwrap();
            match value["command"].as_str().unwrap() {
                "check" => {
                    let ticket = state.begin_launcher_update_check(
                        Some(&config),
                        value["revision"].as_u64().unwrap(),
                        value["operation_revision"].as_u64().unwrap(),
                    )?;
                    state.finish_launcher_update_check(ticket, check(&config).await)?;
                }
                "prepare" => {
                    let mut ticket = state.begin_launcher_update_prepare(
                        Some(&config),
                        serde_json::from_value(value["offer_id"].clone()).unwrap(),
                        value["revision"].as_u64().unwrap(),
                        value["operation_revision"].as_u64().unwrap(),
                    )?;
                    let bytes = download(&config, ticket.offer()?).await;
                    state.mark_launcher_update_verifying(&mut ticket)?;
                    state.finish_launcher_update_prepare(&config, ticket, bytes)?;
                }
                "advance_operation" => {
                    let id = Uuid::new_v4();
                    let revision = state.operations().snapshot().revision;
                    let operations = state.operations_mut().unwrap();
                    operations
                        .begin(id, crate::OperationKind::Launch, [1; 32], revision)
                        .unwrap();
                    operations
                        .observe(id, crate::OperationState::Running)
                        .unwrap();
                    operations
                        .observe(id, crate::OperationState::Succeeded)
                        .unwrap();
                }
                "tamper" => {
                    std::fs::write(root.path().join(ARTIFACT), b"tampered").unwrap();
                }
                _ => {}
            }
            state.launcher_update_snapshot(Some(&config))
        }
        .await;
        let result = match result {
            Ok(snapshot) => serde_json::json!({"ok":snapshot}),
            Err(error) => serde_json::json!({"error":error}),
        };
        println!("UPDATER_NATIVE_UAT {result}");
        std::io::stdout().flush().unwrap();
    }
}

#[tokio::test]
async fn transport_deadline_and_stream_cap_reject_responses() {
    use std::time::Duration;
    let (server, config) = server_fixture("1.1.0", PAYLOAD).await;
    server.reset().await;
    Mock::given(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0; 32]))
        .mount(&server)
        .await;
    assert_eq!(
        transport::get(
            &config,
            config.endpoint.clone(),
            16,
            Duration::from_secs(1),
            false
        )
        .await
        .unwrap_err(),
        Error::Size
    );
    server.reset().await;
    Mock::given(path("/feed"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(100)))
        .mount(&server)
        .await;
    assert_eq!(
        transport::get(
            &config,
            config.endpoint.clone(),
            16,
            Duration::from_millis(20),
            false
        )
        .await
        .unwrap_err(),
        Error::Timeout
    );
}

#[test]
fn active_updater_refuses_early_mutation_but_preserves_consent_control() {
    use crate::{
        launch,
        migration::{LegacySource, MigrationError},
        IntentError,
    };
    fn files(path: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            out.insert(
                entry.file_name().to_str().unwrap().to_owned(),
                if entry.file_name() == "launcher.lock" {
                    // The state owns the Windows byte-range lock. Its empty
                    // payload is checked without a forbidden second read.
                    assert_eq!(entry.metadata().unwrap().len(), 0);
                    vec![]
                } else {
                    std::fs::read(entry.path()).unwrap()
                },
            );
        }
        out
    }
    let root = tempfile::tempdir().unwrap();
    let state_path = root.path().join("state");
    let destination = root.path().join("game");
    let mut state = DesktopState::open(&state_path).unwrap();
    state
        .save_preferences(Some(destination.clone()), true, 0)
        .unwrap();
    let (config, _) = fixture("1.1.0");
    let _ticket = state
        .begin_launcher_update_check(Some(&config), 0, 0)
        .unwrap();
    let before = files(&state_path);
    let seed = crate::install_worker::fixtures::archive(true);
    let release = crate::install_worker::fixtures::verified(&seed);
    let id = Uuid::new_v4();
    let busy = IntentError::Storage(StorageError::Busy);
    assert_eq!(
        state.admit_install(id, 0, 1, &release, vec![]).unwrap_err(),
        busy
    );
    assert!(matches!(
        state.admit_repair(id, 0, id, true),
        Err(IntentError::Storage(StorageError::Busy))
    ));
    assert_eq!(state.uninstall(id, 0, id, true), Err(busy));
    assert!(matches!(
        state.admit_runtime_setup(id, 0, id, [0; 32], [0; 32]),
        Err(IntentError::Storage(StorageError::Busy))
    ));
    assert_eq!(state.clean_failed_install(id, 0), Err(busy));
    let inert = root.path().join("inert");
    std::fs::write(&inert, b"").unwrap();
    let artifact = launch::Artifact::open(
        inert.canonicalize().unwrap(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    )
    .unwrap();
    assert_eq!(
        state
            .admit_launch(
                id,
                0,
                id,
                launch::Resources {
                    helper: artifact,
                    client_patches: None,
                    graphics: None
                }
            )
            .unwrap_err(),
        busy
    );
    let source = LegacySource {
        launcher_directory: root.path().join("legacy"),
        game_directory: root.path().join("old-game"),
    };
    assert!(matches!(
        state.import_legacy(&source, "unused", 1),
        Err(MigrationError::Storage(StorageError::Busy))
    ));
    assert_eq!(
        state.save_preferences(Some(root.path().join("other")), true, 1),
        Err(StorageError::Busy)
    );
    assert!(matches!(
        crate::adoption::recover(&mut state, id, 0),
        Err(crate::adoption::Error::Storage(StorageError::Busy))
    ));
    assert!(matches!(
        crate::adoption::abandon(&mut state, id, 0),
        Err(crate::adoption::Error::Storage(StorageError::Busy))
    ));
    assert_eq!(files(&state_path), before);
    assert!(!destination.exists());
    let saved = state.save_preferences(Some(destination), false, 1).unwrap();
    assert!(!saved.launcher_summary_consent);
    assert_eq!(saved.revision, 2);
    assert_eq!(state.operations().snapshot().revision, 0);
    #[cfg(target_os = "macos")]
    {
        let before = files(&state_path);
        let (progress, _) = crate::install_progress::ProgressSink::latest();
        let result = crate::adoption::preview(
            std::sync::Arc::new(std::sync::Mutex::new(state)),
            crate::adoption::PreviewRequest {
                import_digest: "unused".into(),
                destination: root.path().join("adopted"),
                operation_revision: 0,
                preferences_revision: 2,
                release,
                artifacts: crate::adoption::Artifacts {
                    seed: root.path().join("unused.zip"),
                    patches: vec![],
                },
            },
            tokio_util::sync::CancellationToken::new(),
            progress,
        );
        assert!(matches!(
            result,
            Err(crate::adoption::Error::Storage(StorageError::Busy))
        ));
        assert_eq!(files(&state_path), before);
        assert!(!root.path().join("adopted").exists());
    }
}
