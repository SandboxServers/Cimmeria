//! The install-result event a finished install queues (LX-08c).
use super::tests::{archive, outcome, requested, setup, verified};
use super::*;
use crate::game_telemetry::transport::TelemetryEvent;
use crate::install_report::PatchOutcomeKind;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use serde_json::{Map, Value};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

/// The one install-result event the queue holds, as its fields.
fn the_result(state: &Mutex<DesktopState>) -> Map<String, Value> {
    let root = state.lock().unwrap().state_root().to_path_buf();
    let queued: Vec<TelemetryEvent> = DiskQueue::new(&root).drain().unwrap();
    assert_eq!(queued.len(), 1, "{queued:?}");
    let TelemetryEvent::ClientNative(event) = &queued[0] else {
        panic!("expected ClientNative, got {queued:?}");
    };
    assert_eq!(event.target, "client.launcher.install_result");
    assert!(event.ts_ms > 0, "stamped with the install time");
    event.fields.clone()
}

fn queue_is_empty(state: &Mutex<DesktopState>) -> bool {
    let root = state.lock().unwrap().state_root().to_path_buf();
    DiskQueue::new(&root)
        .drain::<TelemetryEvent>()
        .unwrap()
        .is_empty()
}

/// Install the fixture seed and return what the telemetry queue holds.
async fn install(opted_in: bool) -> Vec<TelemetryEvent> {
    let seed = archive(true);
    let release = verified(&seed);
    let (_root, state, id) = setup(&release);
    state.lock().unwrap().set_game_telemetry(opted_in).unwrap();
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
    let root = state.lock().unwrap().state_root().to_path_buf();
    DiskQueue::new(&root).drain().unwrap()
}

#[tokio::test]
async fn an_opted_in_install_queues_exactly_one_result_from_its_report() {
    let queued = install(true).await;
    assert_eq!(queued.len(), 1, "{queued:?}");
    let TelemetryEvent::ClientNative(event) = &queued[0] else {
        panic!("expected ClientNative, got {queued:?}");
    };
    assert_eq!(event.target, "client.launcher.install_result");
    assert_eq!(event.level, "info");
    assert!(event.ts_ms > 0, "stamped with the install time");
    assert_eq!(event.fields["result"], "ok");
    // From install_all's report, not the desktop outcome alone.
    assert_eq!(event.fields["seed_applied"], true);
    assert_eq!(event.fields["patch_count"], 0);
}

#[tokio::test]
async fn an_opted_out_install_queues_nothing() {
    assert!(install(false).await.is_empty());
}

#[tokio::test]
async fn a_failed_download_queues_an_error_result() {
    let release = verified(&archive(true));
    let (_root, state, id) = setup(&release);
    state.lock().unwrap().set_game_telemetry(true).unwrap();
    // No seed is served: the download fails.
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
    let fields = the_result(&state);
    assert_eq!(fields["result"], "error");
    assert_eq!(fields["seed_applied"], false);
    assert!(fields["error"].as_str().is_some_and(|e| !e.is_empty()));
}

#[tokio::test]
async fn a_cancelled_install_queues_a_cancelled_result() {
    let seed = archive(true);
    let release = verified(&seed);
    let (_root, state, id) = setup(&release);
    state.lock().unwrap().set_game_telemetry(true).unwrap();
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
    worker.request_cancel().unwrap();
    assert_eq!(outcome(worker.result.clone()).await, Outcome::Cancelled);
    assert_eq!(the_result(&state)["result"], "cancelled");
}

/// A Running install whose report has one applied patch, owned by `owner`.
fn running(owner_is_this_run: bool) -> (tempfile::TempDir, Arc<Mutex<DesktopState>>, Uuid) {
    let release = verified(&archive(true));
    let (root, state, id) = setup(&release);
    {
        let mut owner = state.lock().unwrap();
        owner.set_game_telemetry(true).unwrap();
        owner
            .operations_mut()
            .unwrap()
            .observe(id, OperationState::Running)
            .unwrap();
        let mut report = InstallReport::default();
        report.push("001", PatchOutcomeKind::Applied, None);
        let stashed_for = if owner_is_this_run {
            id
        } else {
            Uuid::new_v4()
        };
        owner.stash_install_report(stashed_for, report);
    }
    (root, state, id)
}

#[test]
fn a_failed_stage_carries_the_desktop_code_and_its_own_report() {
    let (_root, state, id) = running(true);
    assert_eq!(
        publish(&state, id, Outcome::ContentInvalid),
        Outcome::ContentInvalid
    );
    let fields = the_result(&state);
    assert_eq!(fields["result"], "error");
    assert_eq!(fields["error"], "content_invalid");
    assert_eq!(fields["patch_count"], 1, "the run's own report");
}

#[test]
fn a_failed_download_logs_the_install_lifecycle() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (worker_outcome, events) = crate::test_capture::capture(|| {
        rt.block_on(async {
            let release = verified(&archive(true));
            let (_root, state, id) = setup(&release);
            state.lock().unwrap().set_game_telemetry(true).unwrap();
            // No seed is served: the download fails.
            let server = MockServer::start().await;
            let worker = dispatch_with(
                state.clone(),
                id,
                release,
                format!("{}/manifest.json", server.uri()),
                reqwest::Client::new(),
            )
            .unwrap();
            outcome(worker.result.clone()).await
        })
    });
    assert_eq!(worker_outcome, Outcome::InstallFailed);
    let install: Vec<_> = events
        .iter()
        .filter(|e| e.target == "launcher.install")
        .collect();
    let started = install
        .iter()
        .find(|e| e.field("event") == Some("install_started"))
        .expect("install_started");
    assert_eq!(started.field("action"), Some("install"));
    let stage = install
        .iter()
        .find(|e| e.field("event") == Some("install_stage_failed"))
        .expect("install_stage_failed");
    assert_eq!(stage.field("http_status"), Some("404"));
    let failed = install
        .iter()
        .find(|e| e.field("event") == Some("install_failed"))
        .expect("install_failed");
    assert_eq!(failed.level, tracing::Level::WARN);
    assert_eq!(failed.field("reason"), Some("install_failed"));
    let operation = started.field("operation");
    assert!(operation.is_some());
    assert_eq!(stage.field("operation"), operation);
    assert_eq!(failed.field("operation"), operation);
}

#[test]
fn a_report_stashed_by_another_operation_is_not_attached() {
    let (_root, state, id) = running(false);
    publish(&state, id, Outcome::ContentInvalid);
    let fields = the_result(&state);
    assert_eq!(fields["error"], "content_invalid");
    assert_eq!(fields["patch_count"], 0, "another run's report");
}

#[test]
fn a_lost_completion_queues_reconciliation_required() {
    let (_root, state, id) = running(true);
    assert_eq!(
        publish(&state, id, Outcome::ReconciliationRequired),
        Outcome::ReconciliationRequired
    );
    let fields = the_result(&state);
    assert_eq!(fields["result"], "error");
    assert_eq!(fields["error"], "reconciliation_required");
}

#[test]
fn opting_out_before_the_run_ends_queues_nothing() {
    let (_root, state, id) = running(true);
    state.lock().unwrap().set_game_telemetry(false).unwrap();
    publish(&state, id, Outcome::InstallFailed);
    assert!(queue_is_empty(&state));
}

#[test]
fn the_phase_watcher_logs_the_open_phase_when_the_sink_closes() {
    // The state is gone (`Weak::new`), so the watcher takes its `break` path;
    // the open `copy` phase must still be logged once with its bytes.
    let ((), events) = crate::test_capture::capture(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (tx, rx) = watch::channel(None);
            tx.send_replace(Some(install::Progress::Copying {
                copied: 5,
                total: 5,
            }));
            let task = tokio::spawn(watch_phases(std::sync::Weak::new(), Uuid::new_v4(), rx));
            drop(tx);
            task.await.unwrap();
        });
    });
    let finished: Vec<_> = events
        .iter()
        .filter(|e| e.field("event") == Some("install_phase_finished"))
        .collect();
    assert_eq!(finished.len(), 1, "{events:?}");
    assert_eq!(finished[0].field("phase"), Some("copy"));
    assert_eq!(finished[0].field("bytes"), Some("5"));
}
