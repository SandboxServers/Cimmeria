//! Launcher summaries through the real install worker: a mock download host,
//! a tempdir state root, and nothing sent anywhere.
use super::tests::outcome;
use super::*;
use crate::{
    client_setup::login_servers::default_servers,
    launcher_summary::{
        MintRequest, Summary, SummaryErrorCode, SummaryOperation, SummaryOutcome, SummaryPhase,
    },
    storage::launcher_summary::tests::{
        config, open_phase, queue_bytes, queued, upload_body, Clock, ENDPOINT,
    },
};
use fixtures::{archive, verified};
use std::io::Read;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

/// A configured, opted-in state with one admitted install. The directory names
/// are the caller's so that a test can seed them with markers.
fn admitted(
    release: &VerifiedRelease,
    state_name: &str,
    install_name: &str,
) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Clock, Uuid) {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join(state_name)).unwrap();
    let clock = Clock::install(&mut state);
    state.configure_summaries(config(Some(ENDPOINT)));
    state
        .save_preferences(Some(root.path().join(install_name)), true, 0)
        .unwrap();
    let id = Uuid::new_v4();
    state
        .admit_install(id, 0, 1, release, default_servers())
        .unwrap();
    (root, Arc::new(Mutex::new(state)), clock, id)
}

fn rows(state: &Mutex<DesktopState>) -> Vec<Summary> {
    let mut owner = state.lock().unwrap();
    owner.finalize_summaries();
    queued(&owner)
}

fn tracking(state: &Mutex<DesktopState>) -> serde_json::Value {
    let bytes = queue_bytes(&state.lock().unwrap()).expect("queue file");
    serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["tracking"].clone()
}

fn timed(summary: &Summary) -> Vec<(TimedPhase, u32)> {
    summary
        .phases
        .iter()
        .flatten()
        .map(|entry| (entry.phase, entry.duration_ms.get()))
        .collect()
}

async fn eventually(what: &str, mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !ready() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {what}"));
}

#[tokio::test]
async fn a_failing_install_reaches_the_queue_as_exactly_one_failed_row() {
    let release = verified(&archive(true));
    let (_root, state, _clock, id) = admitted(&release, "state", "install");
    // The download host has no such blob: the install fails before any byte.
    let server = MockServer::start().await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::InstallFailed);
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].operation, SummaryOperation::Install);
    assert_eq!(rows[0].outcome, SummaryOutcome::Failed);
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::InstallFailed));
    assert_eq!(rows[0].phase, SummaryPhase::Running);
    assert_eq!(
        timed(&rows[0])
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        [TimedPhase::Starting, TimedPhase::Running]
    );
    assert!(rows[0].duration_ms.is_some());
    assert_ne!(rows[0].attempt_id, id);
    // Finalizing again adds nothing.
    assert_eq!(self::rows(&state), rows);
}

/// A download host that sends `body` in `parts` pieces, `gap` apart, to the one
/// request it serves. Returns its base URL and the thread to join.
fn slow_host(body: Vec<u8>, parts: usize, gap: Duration) -> (String, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let served = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        for (index, part) in body.chunks(body.len().div_ceil(parts)).enumerate() {
            if index != 0 {
                std::thread::sleep(gap);
            }
            stream.write_all(part).unwrap();
            stream.flush().unwrap();
        }
    });
    (base, served)
}

#[tokio::test]
async fn a_successful_install_is_one_succeeded_row_with_its_download_and_extraction() {
    let seed = archive(true);
    let release = verified(&seed);
    let (_root, state, _clock, id) = admitted(&release, "state", "install");
    // The seed arrives in pieces a tenth of a second apart. The installer
    // reports download progress at most every 33 ms and then waits for the next
    // piece, so the watcher is certain to see a download report while nothing
    // else can have been reported. (With the whole body at once the only report
    // comes just before unpacking starts on another thread, and the watcher
    // may see that thread's first extraction report instead.)
    let (base, served) = slow_host(seed, 5, Duration::from_millis(100));
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{base}/manifest.json"),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(
        outcome(worker.result.clone()).await,
        Outcome::ContentPrepared
    );
    served.join().unwrap();
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].operation, rows[0].outcome, rows[0].error_code),
        (SummaryOperation::Install, SummaryOutcome::Succeeded, None)
    );
    // The journal phases, then the two the watcher marked from real progress.
    assert_eq!(
        timed(&rows[0])
            .iter()
            .map(|(phase, _)| *phase)
            .collect::<Vec<_>>(),
        [
            TimedPhase::Starting,
            TimedPhase::Running,
            TimedPhase::Download,
            TimedPhase::Extraction,
        ]
    );
    assert_eq!(rows[0].phase, SummaryPhase::Extraction);
    assert!(rows[0].duration_ms.is_some());
}

// A retry is admitted without a finalize of its own. The failed attempt's row
// must exist by then, built while the install result was still its own.
#[tokio::test]
async fn a_failed_install_is_finalized_before_a_retry_can_replace_its_result() {
    let release = verified(&archive(true));
    let (_root, state, _clock, id) = admitted(&release, "state", "install");
    // The download host has no such blob: the install fails before any byte.
    let server = MockServer::start().await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::InstallFailed);

    // What the queue holds before anything but the worker has run.
    let at_once = queued(&state.lock().unwrap());
    // The shell's retry: remove the failed attempt's files, then admit again.
    // Neither call finalizes summaries.
    let retry = Uuid::new_v4();
    {
        let mut owner = state.lock().unwrap();
        let revision = owner.operations().snapshot().revision;
        owner.clean_failed_install(id, revision).unwrap();
        let (operation, preferences) = (
            owner.operations().snapshot().revision,
            owner.preferences().revision,
        );
        let release = verified(&archive(true));
        let admission =
            owner.admit_install(retry, operation, preferences, &release, default_servers());
        assert!(admission.unwrap().dispatch);
    }
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].outcome, SummaryOutcome::Failed);
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::InstallFailed));
    assert_eq!(at_once, rows, "the row was there before the retry");
    // The first attempt's tracking went with its row; the retry's is its own.
    assert_eq!(
        tracking(&state)["local_operation_id"],
        serde_json::json!(retry)
    );
}

#[tokio::test]
async fn a_cancelled_install_is_one_cancelled_row() {
    let seed = archive(true);
    let release = verified(&seed);
    let (_root, state, _clock, id) = admitted(&release, "state", "install");
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
    tokio::time::timeout(Duration::from_secs(5), async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the seed request leaves");
    worker.request_cancel().unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::Cancelled);
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].operation, rows[0].outcome, rows[0].error_code),
        (SummaryOperation::Install, SummaryOutcome::Cancelled, None)
    );
}

#[test]
fn a_result_that_cannot_be_recorded_is_one_unknown_row() {
    let release = verified(&archive(true));
    let (root, state, _clock, id) = admitted(&release, "state", "install");
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
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].operation, rows[0].outcome, rows[0].error_code),
        (SummaryOperation::Install, SummaryOutcome::Unknown, None)
    );
}

#[tokio::test]
async fn the_watcher_enters_a_phase_at_every_change_of_progress_kind_and_ends_with_the_worker() {
    let release = verified(&archive(true));
    let (_root, state, clock, id) = admitted(&release, "state", "install");
    state
        .lock()
        .unwrap()
        .operations_mut()
        .unwrap()
        .observe(id, OperationState::Running)
        .unwrap();
    let (sink, observed) = ProgressSink::latest();
    let watcher = tokio::spawn(watch_phases(Arc::downgrade(&state), id, observed));
    let phase = || open_phase(&state.lock().unwrap());
    let downloading = |label: &str, downloaded| install::Progress::Downloading {
        label: label.into(),
        downloaded,
        total: 10,
    };
    let extracting = |label: &str, current| install::Progress::Extracting {
        label: label.into(),
        current,
        total: 2,
        filename: "SGW.exe".into(),
    };
    use crate::install_progress::ProgressReporter;

    // Progress keeps only the newest value, so the watcher is given time to
    // take each report before the clock moves or the next one is sent.
    clock.advance_ms(5);
    sink.report(downloading("seed", 1));
    eventually("download", || phase() == Some(TimedPhase::Download)).await;
    clock.advance_ms(100);
    // More of the same kind is not a phase change.
    sink.report(downloading("seed", 10));
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(phase(), Some(TimedPhase::Download));
    sink.report(extracting("seed", 1));
    eventually("extraction", || phase() == Some(TimedPhase::Extraction)).await;
    clock.advance_ms(30);
    // A patch is downloaded and unpacked after the seed: both phases reopen
    // and add to their totals.
    sink.report(downloading("patch", 3));
    eventually("the patch download", || {
        phase() == Some(TimedPhase::Download)
    })
    .await;
    clock.advance_ms(40);
    sink.report(extracting("patch", 2));
    eventually("the patch extraction", || {
        phase() == Some(TimedPhase::Extraction)
    })
    .await;
    clock.advance_ms(7);
    assert_eq!(
        publish(&state, id, Outcome::ContentInvalid),
        Outcome::ContentInvalid
    );
    // The worker dropping its sink ends the watcher, which kept no state handle.
    drop(sink);
    tokio::time::timeout(Duration::from_secs(5), watcher)
        .await
        .expect("watcher ends with the worker")
        .unwrap();
    assert_eq!(Arc::strong_count(&state), 1);
    let rows = rows(&state);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        timed(&rows[0]),
        [
            (TimedPhase::Starting, 0),
            (TimedPhase::Running, 5),
            (TimedPhase::Download, 140),
            (TimedPhase::Extraction, 37),
        ]
    );
    assert_eq!(rows[0].phase, SummaryPhase::Extraction);
    assert_eq!(rows[0].error_code, Some(SummaryErrorCode::ContentInvalid));
    assert_eq!(rows[0].duration_ms.map(|value| value.get()), Some(182));
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[tokio::test]
async fn no_path_url_or_operation_id_reaches_the_queue_or_a_request_body() {
    const STATE: &str = "MARKER-STATE-ROOT";
    const INSTALL: &str = "MARKER-INSTALL-DIR";
    const URL: &str = "MARKER-MANIFEST-URL";
    let release = verified(&archive(true));
    let (root, state, _clock, id) = admitted(
        &release,
        &format!("state-{STATE}"),
        &format!("install-{INSTALL}"),
    );
    let state_root = root.path().join(format!("state-{STATE}"));
    let server = MockServer::start().await;
    let worker = dispatch_with(
        state.clone(),
        id,
        release,
        format!("{}/{URL}/manifest.json", server.uri()),
        reqwest::Client::new(),
    )
    .unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::InstallFailed);
    // The failing request really carried the marker.
    let requests = server.received_requests().await.unwrap();
    assert!(requests
        .iter()
        .any(|request| request.url.path().contains(URL)));

    let mut owner = state.lock().unwrap();
    owner.finalize_summaries();
    let rows = queued(&owner);
    assert_eq!(rows.len(), 1);
    let queue = queue_bytes(&owner).expect("queue file");
    let body = upload_body(&mut owner).expect("one row to upload");
    let mint = serde_json::to_vec(&MintRequest::new(
        Uuid::new_v4(),
        crate::launcher_summary::LauncherVersion::new((0, 1, 0)),
    ))
    .unwrap();
    let row = serde_json::to_vec(&rows[0]).unwrap();

    let id_forms = [
        id.hyphenated().to_string().into_bytes(),
        id.simple().to_string().into_bytes(),
        id.hyphenated().to_string().to_uppercase().into_bytes(),
        id.as_bytes().to_vec(),
    ];
    let markers = [
        STATE,
        INSTALL,
        URL,
        "127.0.0.1",
        "manifest.json",
        "seed.zip",
    ];
    for (name, bytes) in [
        ("queue file", &queue),
        ("ingest body", &body),
        ("mint body", &mint),
        ("summary", &row),
    ] {
        for marker in markers {
            // The loopback test endpoint is configuration, not content: it is
            // in no file and no body either.
            assert!(!contains(bytes, marker.as_bytes()), "{marker} in {name}");
        }
        for form in &id_forms {
            assert!(!contains(bytes, form), "operation id in {name}");
        }
    }
    // Positive controls: the same scan finds each value where it does live.
    let intent = std::fs::read(state_root.join(format!("install-intent-{id}.json"))).unwrap();
    assert!(contains(&intent, INSTALL.as_bytes()));
    let journal = std::fs::read(state_root.join("operation.json")).unwrap();
    assert!(contains(&journal, &id_forms[0]));
    assert!(contains(&body, rows[0].event_id.to_string().as_bytes()));
}
