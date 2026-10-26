//! Tests for `run.rs`. Each one runs against a wiremock server on 127.0.0.1,
//! with made-up records in temp folders and short deadlines.

use std::fs;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cimmeria_cooked_pak::catalog::encode_body;
use cimmeria_cooked_pak::pak::{read_pak, read_version};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

#[path = "run_fixtures.rs"]
mod fixtures;
use fixtures::*;

#[tokio::test]
async fn a_stale_category_is_synced_and_written() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    serve_body(&server, 4, 6, body.1.clone()).await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Synced);
    assert_eq!(report.reason, "synced");
    assert_eq!(report.categories.len(), 1);
    assert_eq!(report.categories[0].action, CategoryAction::Written);
    assert_eq!(report.categories[0].bytes, body.1.len() as u64);
    assert_eq!(report.bytes_downloaded, body.1.len() as u64);
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(6));
    assert_eq!(read_pak(&cache.join(PAK_4)).unwrap().entries, composed());
    assert!(temps(&cache).is_empty());
}

#[tokio::test]
async fn matching_versions_are_up_to_date_and_write_nothing() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    Mock::given(method("GET"))
        .and(path("/api/cooked/v1/category/4"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 6, &base());
    let before = fs::read(cache.join(PAK_4)).unwrap();

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::UpToDate);
    assert_eq!(report.reason, "versions_match");
    assert_eq!(report.categories[0].action, CategoryAction::UpToDate);
    assert_eq!(report.bytes_downloaded, 0);
    assert_eq!(fs::read(cache.join(PAK_4)).unwrap(), before);
    server.verify().await;
}

#[tokio::test]
async fn source_newer_fails_at_plan_before_any_download() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    Mock::given(method("GET"))
        .and(path("/api/cooked/v1/category/4"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 7, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    assert_eq!(report.reason, "source_newer");
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Plan);
    assert_eq!(failure.category_id, Some(4));
    assert_eq!(failure.pak.as_deref(), Some(PAK_4));
    assert_eq!(failure.source_version, Some(7));
    assert_eq!(failure.served_version, Some(6));
    assert_eq!(report.categories.len(), 1);
    assert_eq!(report.categories[0].action, CategoryAction::Failed);
    assert_eq!(report.categories[0].category_id, 4);
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(4));
    server.verify().await;
}

#[tokio::test]
async fn a_500_on_a_body_fails_at_download_naming_the_category() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    Mock::given(method("GET"))
        .and(path("/api/cooked/v1/category/4"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());
    let before = fs::read(cache.join(PAK_4)).unwrap();

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Download);
    assert_eq!(failure.reason, "http_error");
    assert_eq!(failure.http_status, Some(500));
    assert_eq!(failure.category_id, Some(4));
    assert_eq!(failure.pak.as_deref(), Some(PAK_4));
    assert_eq!(report.categories.len(), 1);
    assert_eq!(report.categories[0].action, CategoryAction::Failed);
    assert_eq!(fs::read(cache.join(PAK_4)).unwrap(), before);
    assert!(temps(&cache).is_empty());
}

#[tokio::test]
async fn a_digest_mismatch_fails_at_stage_and_changes_nothing() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let good4 = served(4, 6);
    let good5 = served(5, 6);
    // Category 5's body carries a digest that the composed records do not match.
    let mut bad5 = served(5, 6);
    bad5.0.digest = [0xEE; 32];
    let bad5_bytes = encode_body(&bad5.0);
    serve_index(
        &server,
        index(vec![entry(4, PAK_4, &good4), entry(5, PAK_5, &good5)]),
    )
    .await;
    serve_body(&server, 4, 6, good4.1.clone()).await;
    serve_body(&server, 5, 6, bad5_bytes).await;
    for (dir, pak) in [(&source, PAK_4), (&source, PAK_5)] {
        write_pak_at(dir, pak, 5, &base());
    }
    write_pak_at(&cache, PAK_4, 4, &base());
    write_pak_at(&cache, PAK_5, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Stage);
    assert_eq!(failure.reason, "digest_mismatch");
    assert_eq!(failure.category_id, Some(5));
    assert_eq!(failure.pak.as_deref(), Some(PAK_5));
    assert_eq!(report.categories.len(), 1);
    assert_eq!(report.categories[0].category_id, 5);
    assert_eq!(report.categories[0].action, CategoryAction::Failed);
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(4));
    assert_eq!(read_pak(&cache.join(PAK_4)).unwrap().entries, base());
    assert_eq!(read_pak(&cache.join(PAK_5)).unwrap().entries, base());
    assert!(temps(&cache).is_empty());
}

#[tokio::test]
async fn catalog_unavailable_waits_retry_after_then_succeeds() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    // The first index answer is still building; the mocks answer in order.
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    serve_body(&server, 4, 6, body.1.clone()).await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Synced);
    assert_eq!(report.attempts, 2);
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(6));
}

#[tokio::test]
async fn a_retry_after_past_the_limit_fails_with_retry_after_s() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "60"))
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(120)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    assert_eq!(report.attempts, 1);
    assert!(report.duration_ms < 10_000, "the run must not wait 60 s");
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Index);
    assert_eq!(failure.reason, "catalog_unavailable");
    assert_eq!(failure.retry_after_s, Some(60));
}

#[tokio::test]
async fn catalog_failed_fails_at_once() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(
            ResponseTemplate::new(503)
                .set_body_json(serde_json::json!({ "error": "catalog_failed" })),
        )
        .expect(1)
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    assert_eq!(report.attempts, 1);
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Index);
    assert_eq!(failure.reason, "catalog_failed");
    assert_eq!(failure.http_status, Some(503));
    server.verify().await;
}

#[tokio::test]
async fn version_changed_restarts_once_then_fails() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    Mock::given(method("GET"))
        .and(path("/api/cooked/v1/category/4"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_secs(60)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    assert_eq!(report.reason, "version_changed");
    assert_eq!(
        report.attempts, 2,
        "one restart from the index, then the failure"
    );
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Download);
    assert_eq!(failure.category_id, Some(4));
    assert_eq!(failure.http_status, Some(409));
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(4));
}

#[tokio::test]
async fn disabled_and_404_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    write_pak_at(&source, PAK_4, 5, &base());

    let disabled = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(serde_json::json!({ "error": "disabled" })),
        )
        .mount(&disabled)
        .await;
    let report = run_quiet(&disabled, root.path(), &source, Duration::from_secs(60)).await;
    assert_eq!(report.outcome, Outcome::Skipped);
    assert_eq!(report.reason, "disabled");
    assert!(report.failure.is_none());

    let unsupported = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(ResponseTemplate::new(404))
        .mount(&unsupported)
        .await;
    let report = run_quiet(&unsupported, root.path(), &source, Duration::from_secs(60)).await;
    assert_eq!(report.outcome, Outcome::Skipped);
    assert_eq!(report.reason, "server_unsupported");
    assert!(report.failure.is_none());

    assert!(!cache.join(PAK_4).exists());
    assert!(temps(&cache).is_empty());
}

#[tokio::test]
async fn the_deadline_names_the_step_in_progress() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    Mock::given(method("GET"))
        .and(path("/api/cooked/v1/category/4"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body.1.clone(), "application/octet-stream")
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&server)
        .await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    let report = run_quiet(&server, root.path(), &source, Duration::from_millis(300)).await;

    assert_eq!(report.outcome, Outcome::Failed);
    assert_eq!(report.reason, "deadline");
    assert!(
        report.duration_ms < 5_000,
        "the run must stop at the deadline"
    );
    let failure = report.failure.expect("a failure");
    assert_eq!(failure.step, Step::Download);
    assert_eq!(failure.category_id, Some(4));
    assert_eq!(failure.pak.as_deref(), Some(PAK_4));
    assert_eq!(failure.deadline_s, Some(1));
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(4));
}

#[tokio::test]
async fn cancel_before_commit_leaves_no_temps_and_no_change() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let body = served(4, 6);
    serve_index(&server, index(vec![entry(4, PAK_4, &body)])).await;
    serve_body(&server, 4, 6, body.1.clone()).await;
    write_pak_at(&source, PAK_4, 5, &base());
    write_pak_at(&cache, PAK_4, 4, &base());

    // The cancel fires from the `Writing` callback, so the stage never starts.
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let progress = move |p: Progress| {
        if p.stage == Stage::Writing {
            trigger.cancel();
        }
    };
    let report = run(
        &fetcher_for(&server),
        &root.path().join("firesky"),
        &source,
        Instant::now() + Duration::from_secs(60),
        &cancel,
        &progress,
    )
    .await;

    assert_eq!(report.outcome, Outcome::Cancelled);
    assert_eq!(report.reason, "cancelled");
    let failure = report.failure.expect("the stop names its step");
    assert_eq!(failure.step, Step::Stage);
    assert_eq!(failure.category_id, Some(4));
    assert_eq!(read_pak(&cache.join(PAK_4)).unwrap().entries, base());
    assert_eq!(read_version(&cache.join(PAK_4)).unwrap(), Some(4));
    assert!(temps(&cache).is_empty());
}

#[tokio::test]
async fn progress_reports_totals_then_committing() {
    let server = MockServer::start().await;
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), cache_of(root.path()));
    let (good4, good5) = (served(4, 6), served(5, 6));
    serve_index(
        &server,
        index(vec![entry(4, PAK_4, &good4), entry(5, PAK_5, &good5)]),
    )
    .await;
    serve_body(&server, 4, 6, good4.1.clone()).await;
    serve_body(&server, 5, 6, good5.1.clone()).await;
    for pak in [PAK_4, PAK_5] {
        write_pak_at(&source, pak, 5, &base());
        write_pak_at(&cache, pak, 4, &base());
    }
    let total = (good4.1.len() + good5.1.len()) as u64;

    let seen = Mutex::new(Vec::new());
    let record = |p: Progress| seen.lock().unwrap().push(p);
    let report = run(
        &fetcher_for(&server),
        &root.path().join("firesky"),
        &source,
        Instant::now() + Duration::from_secs(60),
        &CancellationToken::new(),
        &record,
    )
    .await;

    assert_eq!(report.outcome, Outcome::Synced);
    assert_eq!(report.bytes_downloaded, total);
    let seen = seen.into_inner().unwrap();
    let stages: Vec<Stage> = seen.iter().map(|p| p.stage).collect();
    assert_eq!(
        stages,
        [
            Stage::Checking,
            Stage::Downloading,
            Stage::Writing,
            Stage::Downloading,
            Stage::Writing,
            Stage::Committing,
        ]
    );
    let last = seen.last().expect("progress was reported");
    assert_eq!(last.categories_done, 2);
    assert_eq!(last.categories_total, 2);
    assert_eq!(last.bytes_total, total);
    assert_eq!(last.bytes_done, total);
}
