use super::*;
use crate::{
    catalog::verify_release,
    launcher_compatibility::{CompatibilityPolicy, Identity},
    OperationState,
};
use ed25519_dalek::{Signer, SigningKey};
use uuid::Uuid;
use wiremock::matchers::{header, method, path, path_regex, query_param};
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
        LogUploadStep::Collect(collection) => match collection.collect().unwrap() {
            CollectedLogs::Finished(status) => status.state,
            CollectedLogs::Send(pending) => panic!("expected a status, got {pending:?}"),
        },
    }
}

/// An admitted upload's collection step.
fn collect(state: &DesktopState, store: &LogStore) -> LogCollection {
    match state.begin_log_upload(upload(), Some(store)).unwrap() {
        LogUploadStep::Collect(collection) => collection,
        LogUploadStep::Finished(status) => panic!("expected a collection, got {status:?}"),
    }
}

/// The rest of an upload after the collection step, as the shell runs it.
async fn send(
    state: &DesktopState,
    store: &LogStore,
    collection: LogCollection,
) -> Result<LogUploadState, LogUploadError> {
    match collection.collect()? {
        CollectedLogs::Finished(status) => Ok(status.state),
        CollectedLogs::Send(pending) => {
            let stored = store.put(pending).await?;
            Ok(state.finish_log_upload(stored).state)
        }
    }
}

/// One full upload as the shell runs it.
async fn run(state: &DesktopState, store: &LogStore) -> Result<LogUploadState, LogUploadError> {
    match state.begin_log_upload(upload(), Some(store))? {
        LogUploadStep::Finished(status) => Ok(status.state),
        LogUploadStep::Collect(collection) => send(state, store, collection).await,
    }
}

/// A store that answers every PUT with 201 and expects `puts` of them.
async fn accepting_store(puts: u64) -> (MockServer, LogStore) {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(201))
        .expect(puts)
        .mount(&server)
        .await;
    let store = LogStore::insecure_for_tests(&format!("{}/c?{SECRET}", server.uri()));
    (server, store)
}

fn zip_names(body: &[u8]) -> Vec<String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(body.to_vec())).unwrap();
    let mut names: Vec<_> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    names.sort();
    names
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
    assert_eq!(
        zip_names(&requests[0].body),
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

/// A redirect is not followed: following it would send the signed request's
/// body (and a request) to wherever the answer points. Fails if
/// `.redirect(Policy::none())` is removed: the target then gets the request
/// and the upload reports success.
#[tokio::test]
async fn a_redirect_is_rejected_and_not_followed() {
    let server = MockServer::start().await;
    Mock::given(path("/elsewhere"))
        .respond_with(ResponseTemplate::new(201))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path_regex(r"^/c/logs/"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/elsewhere", server.uri()).as_str()),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = LogStore::insecure_for_tests(&format!("{}/c?{SECRET}", server.uri()));
    let (_root, state, _) = installed();
    assert_eq!(run(&state, &store).await, Err(LogUploadError::Rejected));
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests.iter().all(|r| r.url.path() != "/elsewhere"),
        "{requests:?}"
    );
}

/// reqwest's own error text names the request URL, signature included; the
/// mapping strips it first, so a later `{error}` anywhere cannot leak it.
#[tokio::test]
async fn a_transport_errors_url_is_stripped_before_it_is_mapped() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead_url = format!("http://{}/c?{SECRET}", closed.local_addr().unwrap());
    drop(closed);
    let error = reqwest::Client::new()
        .put(&dead_url)
        .send()
        .await
        .unwrap_err();
    // The hazard is real: the raw error carries the signature.
    assert!(format!("{error}").contains("sig="), "{error}");
    let stripped = error.without_url();
    for text in [format!("{stripped}"), format!("{stripped:?}")] {
        assert!(!text.contains("sig="), "{text}");
    }
    assert_eq!(store::network_error(&stripped), LogUploadError::Network);
}

/// The cap is applied from file metadata before anything is read: the oldest
/// logs are left out, the newest are sent, and the status says so. Fails if
/// the cap is not applied (all three logs would be zipped and the state would
/// be plain `uploaded`).
#[tokio::test]
async fn over_the_cap_the_oldest_logs_are_left_out() {
    let (server, store) = accepting_store(1).await;
    let (_root, state, binaries) = installed();
    // `installed` wrote two 5- and 7-byte logs just now; add an old 4000-byte one.
    let old = binaries.join("sessions/one/older.log");
    std::fs::write(&old, [b'o'; 4000]).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(86_400))
        .unwrap();
    let collection = collect(&state, &store).with_max_bytes(1000);
    assert_eq!(
        send(&state, &store, collection).await,
        Ok(LogUploadState::UploadedNewest)
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        zip_names(&requests[0].body),
        [
            "Binaries/SGWDebugLog.log",
            "Binaries/sessions/one/client.log"
        ]
    );
    // Every log alone over the cap: refused with its own code, nothing sent.
    let collection = collect(&state, &store).with_max_bytes(4);
    assert_eq!(
        collection.collect().unwrap_err(),
        LogUploadError::LogsTooLarge
    );
    assert_eq!(MAX_UPLOAD_BYTES, 64 * 1024 * 1024);
}

/// The PUT runs without the state lock, so an operation can start while it
/// is in flight. The ledger write waits behind the gate like any other disk
/// write: it is skipped, and the upload still reports success. Fails if
/// `finish_log_upload` drops its `ensure_no_active_operation` re-check.
#[tokio::test]
async fn an_operation_started_during_the_put_skips_the_ledger_write() {
    let (_server, store) = accepting_store(1).await;
    let (root, mut state, _) = installed();
    let collection = collect(&state, &store);
    let CollectedLogs::Send(pending) = collection.collect().unwrap() else {
        panic!("expected logs to send");
    };
    let stored = store.put(pending).await.unwrap();
    let digest = stored.digest.clone();
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
    assert_eq!(
        state.finish_log_upload(stored).state,
        LogUploadState::Uploaded
    );
    assert!(!state.uploaded_ledger().contains(&digest));
    assert!(!root.path().join("state").join(LEDGER).exists());
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
        (LogUploadError::LogsTooLarge, "logs_too_large"),
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
