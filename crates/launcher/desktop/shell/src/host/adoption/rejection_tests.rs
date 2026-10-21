//! A refused step keeps the check that refused it beside the error, and the
//! status names the progress item and the rejection fields the webview reads.
use super::contract::{AdoptionError, Phase, Progress, ProgressItem};
use super::fixture::*;
use std::fs;
use std::time::Duration;
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_preview_shows_the_failed_check_beside_its_error() {
    let mut f = Fixture::new(serde_json::json!({}));
    let server = MockServer::start().await;
    Mock::given(path("/seed.zip"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    f.serve_from(format!("{}/manifest.json", server.uri()));
    f.begin().unwrap();
    let status = until(
        &f.host,
        "a refused preview",
        Duration::from_secs(30),
        |status| status.last_error.is_some(),
    )
    .await;
    assert_eq!(status.last_error, Some(AdoptionError::Network));
    let rejection = status.last_rejection.expect("the refused check");
    assert_eq!(rejection.check, "download_http_status");
    assert_eq!(rejection.entry.as_deref(), Some("seed"));
    assert_eq!(rejection.http_status, Some(500));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_confirmation_shows_the_failed_check() {
    let mut f = Fixture::new(serde_json::json!({}));
    let _server = f.serve().await;
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    // The player changes the old folder after the review, before the copy starts.
    fs::write(
        f.source.launcher_directory.join("launcher-config.json"),
        "{}",
    )
    .unwrap();
    f.host.adoption_command(confirmation(&review)).unwrap();
    let status = until(
        &f.host,
        "a refused confirmation",
        Duration::from_secs(30),
        |status| status.last_error.is_some(),
    )
    .await;
    assert_eq!(status.last_error, Some(AdoptionError::SourceChanged));
    let rejection = status.last_rejection.expect("the refused check");
    assert_eq!(rejection.check, "folder_changed_during_check");
    assert_eq!(rejection.entry, None);
}

#[test]
fn the_status_json_names_the_new_fields() {
    let f = Fixture::new(serde_json::json!({}));
    let status = serde_json::to_value(f.host.adoption_status().unwrap()).unwrap();
    assert!(status["last_rejection"].is_null());

    let with_item = Progress {
        phase: Phase::Download,
        current: 1,
        total: 2,
        item: Some(ProgressItem::Seed),
    };
    let value = serde_json::to_value(with_item).unwrap();
    assert_eq!(value["item"], "seed");

    let without_item = Progress {
        phase: Phase::Download,
        current: 1,
        total: 2,
        item: None,
    };
    let value = serde_json::to_value(without_item).unwrap();
    assert!(value.get("item").is_none());
}
