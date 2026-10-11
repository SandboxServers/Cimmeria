//! The install-result event a finished install queues (LX-08c).
use super::tests::{archive, outcome, setup, verified};
use super::*;
use crate::game_telemetry::transport::TelemetryEvent;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

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
