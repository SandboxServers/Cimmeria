use super::*;
use crate::{
    catalog::verify_release,
    launcher_compatibility::{CompatibilityPolicy, Identity},
    OperationState,
};
use ed25519_dalek::{Signer, SigningKey};
use uuid::Uuid;
use wiremock::matchers::{header, method, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SECRET: &str = "sv=2026&sig=not-a-real-signature";

fn release() -> crate::catalog::VerifiedRelease {
    let body = serde_json::to_vec(&serde_json::json!({"schema":1,"min_launcher":"launcher-20261002-aaaaaaa","seed":{"blob":"seed.zip","size":1,"sha256":"a".repeat(64)},"patches":[]})).unwrap();
    let signature = SigningKey::from_bytes(&crate::manifest::DEV_MANIFEST_PRIVKEY).sign(&body);
    let hex: String = signature
        .to_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    verify_release(&body, hex.as_bytes()).unwrap()
}

/// A succeeded install (as `launcher_minimum_tests`) whose game directory holds
/// a debug log and one session log. Returns the game's binaries directory.
fn installed() -> (tempfile::TempDir, DesktopState, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open_with_compatibility(
        &root.path().join("state"),
        CompatibilityPolicy::new(Identity::from_parts("0.1.0", None, None, None), vec![]),
    )
    .unwrap();
    state
        .save_preferences(Some(root.path().join("install")), false, 0)
        .unwrap();
    let id = Uuid::new_v4();
    let intent = state
        .admit_install(id, 0, 1, &release(), vec![])
        .unwrap()
        .intent;
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    std::fs::create_dir(&intent.destination).unwrap();
    for name in [".cimmeria-install.json", "content-ready.json"] {
        atomic::write(&intent.destination, name, &intent).unwrap();
    }
    state.remember_prepared_content().unwrap();
    state
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Succeeded)
        .unwrap();
    let binaries = intent.destination.join("game/Working/Binaries");
    std::fs::create_dir_all(binaries.join("sessions/one")).unwrap();
    std::fs::write(binaries.join("SGWDebugLog.log"), b"debug").unwrap();
    std::fs::write(binaries.join("sessions/one/client.log"), b"session").unwrap();
    (root, state, binaries)
}

fn upload() -> LogUploadCommand {
    LogUploadCommand::Upload { schema_version: 1 }
}

fn status(step: LogUploadStep) -> LogUploadState {
    match step {
        LogUploadStep::Finished(status) => status.state,
        LogUploadStep::Send(pending) => panic!("expected a status, got {pending:?}"),
    }
}

/// One full upload as the shell runs it.
async fn run(state: &DesktopState, store: &LogStore) -> Result<LogUploadState, LogUploadError> {
    match state.begin_log_upload(upload(), Some(store))? {
        LogUploadStep::Finished(status) => Ok(status.state),
        LogUploadStep::Send(pending) => {
            store.put(&pending).await?;
            Ok(state.finish_log_upload(pending).state)
        }
    }
}

#[tokio::test]
async fn one_put_per_new_content_and_none_for_a_repeat() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path_regex(
            r"^/logs-container/logs/[A-Za-z0-9-]+-\d{8}T\d{6}Z-[0-9a-f]{12}\.zip$",
        ))
        .and(query_param("sig", "not-a-real-signature"))
        .and(header("x-ms-blob-type", "BlockBlob"))
        .respond_with(ResponseTemplate::new(201))
        .expect(2)
        .mount(&server)
        .await;
    let store = LogStore::insecure_for_tests(&format!("{}/logs-container?{SECRET}", server.uri()));
    let (root, state, binaries) = installed();

    assert_eq!(run(&state, &store).await, Ok(LogUploadState::Uploaded));
    // Same files: the ledger answers and no request is made. Dropping the
    // ledger makes this a second PUT, and `expect(2)` then sees three.
    assert_eq!(run(&state, &store).await, Ok(LogUploadState::NothingNew));
    // The ledger is on disk, not in memory: a reopened state still skips.
    drop(state);
    let state = DesktopState::open_with_compatibility(
        &root.path().join("state"),
        CompatibilityPolicy::new(Identity::from_parts("0.1.0", None, None, None), vec![]),
    )
    .unwrap();
    assert_eq!(run(&state, &store).await, Ok(LogUploadState::NothingNew));
    // New log content is a new upload.
    std::fs::write(binaries.join("SGWDebugLog.log"), b"debug, more").unwrap();
    assert_eq!(run(&state, &store).await, Ok(LogUploadState::Uploaded));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(requests[0].body.clone())).unwrap();
    let mut names: Vec<_> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "Binaries/SGWDebugLog.log",
            "Binaries/sessions/one/client.log"
        ]
    );
}

#[tokio::test]
async fn a_failed_put_names_no_part_of_the_signed_url_and_records_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(403).set_body_string(SECRET))
        .expect(2)
        .mount(&server)
        .await;
    let store = LogStore::insecure_for_tests(&format!("{}/c?{SECRET}", server.uri()));
    let (_root, state, _) = installed();
    let refused = run(&state, &store).await.unwrap_err();
    assert_eq!(refused, LogUploadError::Rejected);
    // Not recorded, so a retry sends again.
    assert_eq!(run(&state, &store).await, Err(LogUploadError::Rejected));

    // A transport failure: reqwest's own error text would carry the URL.
    // A port nothing listens on (wiremock pools its servers, so a dropped
    // `MockServer` keeps answering).
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead_url = format!("http://{}/c?{SECRET}", closed.local_addr().unwrap());
    drop(closed);
    let unreachable = run(&state, &LogStore::insecure_for_tests(&dead_url))
        .await
        .unwrap_err();
    assert_eq!(unreachable, LogUploadError::Network);

    for error in [refused, unreachable] {
        for text in [serde_json::to_string(&error).unwrap(), format!("{error:?}")] {
            assert!(!text.contains("sig="), "{text}");
            assert!(!text.contains("not-a-real-signature"), "{text}");
        }
    }
    assert!(!format!("{store:?}").contains("sig"));
}

#[test]
fn only_https_stores_are_accepted() {
    assert!(LogStore::new("http://example.invalid/c?sig=x").is_none());
    assert!(LogStore::new("").is_none());
    assert!(LogStore::new("https://example.invalid/c?sig=x").is_some());
}

#[test]
fn no_store_reports_disabled_and_no_install_reports_not_installed() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    let inspect = LogUploadCommand::Inspect { schema_version: 1 };
    for request in [inspect, upload()] {
        assert_eq!(
            status(state.begin_log_upload(request, None).unwrap()),
            LogUploadState::Disabled
        );
    }
    let store = LogStore::new("https://example.invalid/c?sig=x").unwrap();
    assert_eq!(
        status(state.begin_log_upload(inspect, Some(&store)).unwrap()),
        LogUploadState::NotInstalled
    );
    let (_root, state, binaries) = installed();
    assert_eq!(
        status(state.begin_log_upload(inspect, Some(&store)).unwrap()),
        LogUploadState::Ready
    );
    std::fs::remove_dir_all(binaries).unwrap();
    assert_eq!(
        status(state.begin_log_upload(upload(), Some(&store)).unwrap()),
        LogUploadState::NoLogs
    );
}

#[test]
fn upload_is_refused_while_an_operation_is_nonterminal() {
    let (_root, mut state, _) = installed();
    let revision = state.operations().snapshot().revision;
    state
        .operations_mut()
        .unwrap()
        .begin(
            Uuid::new_v4(),
            crate::OperationKind::Launch,
            [0; 32],
            revision,
        )
        .unwrap();
    let store = LogStore::new("https://example.invalid/c?sig=x").unwrap();
    assert_eq!(
        state.begin_log_upload(upload(), Some(&store)).unwrap_err(),
        LogUploadError::Storage(StorageError::Busy)
    );
    // The gate comes first: even a build with no store answers busy.
    assert_eq!(
        state.begin_log_upload(upload(), None).unwrap_err(),
        LogUploadError::Storage(StorageError::Busy)
    );
}

#[test]
fn rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state
            .begin_log_upload(LogUploadCommand::Inspect { schema_version: 2 }, None)
            .unwrap_err(),
        LogUploadError::Storage(StorageError::UnsupportedSchema)
    );
    assert!(serde_json::from_str::<LogUploadCommand>(
        r#"{"command":"upload","schema_version":1,"path":"/other"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<LogUploadCommand>(
        r#"{"command":"upload","schema_version":1,"url":"https://elsewhere"}"#
    )
    .is_err());
}

#[test]
fn errors_and_states_serialize_as_flat_codes() {
    for (error, code) in [
        (LogUploadError::CollectFailed, "collect_failed"),
        (LogUploadError::Network, "network"),
        (LogUploadError::Rejected, "rejected"),
        (LogUploadError::Storage(StorageError::Busy), "busy"),
        (
            LogUploadError::Storage(StorageError::UnsupportedSchema),
            "unsupported_schema",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!(code)
        );
    }
    assert_eq!(
        serde_json::to_value(LogUploadStatus::new(LogUploadState::NothingNew)).unwrap(),
        serde_json::json!({"schema_version": 1, "state": "nothing_new"})
    );
}
