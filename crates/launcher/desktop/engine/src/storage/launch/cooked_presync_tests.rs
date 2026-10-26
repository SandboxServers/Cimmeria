//! Tests for the launch's cooked-data presync step (LP-22): the profile and
//! server early exits, the final update, the gate, the cancel and the events.
use super::cooked_presync::*;
use crate::phase_timing;
use crate::test_capture::capture;
use cimmeria_launcher_core::client_setup::login_servers::LoginServer;
use cimmeria_launcher_core::cooked_presync::fetch::{server_label, Timeouts};
use cimmeria_launcher_core::cooked_presync::report::{
    CategoryAction, CategoryReport, Outcome, Report, Step,
};
use std::net::TcpListener;
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn server_at(url: &str) -> Vec<LoginServer> {
    vec![LoginServer {
        name: "Test".into(),
        url: url.into(),
    }]
}

fn report(outcome: Outcome) -> Report {
    Report {
        outcome,
        reason: "test",
        failure: None,
        categories: Vec::new(),
        bytes_downloaded: 0,
        duration_ms: 0,
        attempts: 0,
    }
}

/// Runs the presync with the given profile and servers under the test limits,
/// and returns its report and the final update.
fn run_presync(
    profile: Profile,
    servers: &[LoginServer],
    deadline: Duration,
    cancel: &CancellationToken,
) -> (Report, ContentSync) {
    let rt = runtime();
    let (tx, rx) = watch::channel(None);
    let content = tempfile::tempdir().unwrap();
    let report = rt.block_on(presync_at(
        profile,
        content.path(),
        servers,
        Timeouts::DEFAULT,
        deadline,
        cancel,
        &tx,
    ));
    let last = rx.borrow().clone().expect("the final update is sent");
    (report, last)
}

#[test]
fn not_this_platform_is_skipped_and_sends_done() {
    let cancel = CancellationToken::new();
    let (report, last) = run_presync(
        Profile::NotThisPlatform,
        &[],
        Duration::from_secs(5),
        &cancel,
    );
    assert_eq!(report.outcome, Outcome::Skipped);
    assert_eq!(last.phase, ContentSyncPhase::Done);
    assert_eq!(last.outcome, Some(Outcome::Skipped));
    assert_eq!(last.reason, Some("platform_unsupported"));
    assert_eq!(last.failure, None);
    assert_eq!(last.server, None);
}

#[test]
fn an_unavailable_profile_fails_at_prepare() {
    let cancel = CancellationToken::new();
    let servers = server_at("http://127.0.0.1:9");
    let (_, last) = run_presync(
        Profile::Unavailable,
        &servers,
        Duration::from_secs(5),
        &cancel,
    );
    assert_eq!(last.outcome, Some(Outcome::Failed));
    let failure = last.failure.expect("a failed run names its failure");
    assert_eq!(failure.step, Step::Prepare);
    assert_eq!(failure.reason, "profile_unavailable");
    assert_eq!(last.server.as_deref(), Some("127.0.0.1:9"));
}

#[test]
fn no_login_server_fails_at_prepare() {
    let cancel = CancellationToken::new();
    let dir = tempfile::tempdir().unwrap();
    let profile = Profile::Firesky(dir.path().to_path_buf());
    let (_, last) = run_presync(profile, &[], Duration::from_secs(5), &cancel);
    let failure = last.failure.expect("a failed run names its failure");
    assert_eq!(failure.step, Step::Prepare);
    assert_eq!(failure.reason, "no_login_server");
    assert_eq!(last.server, None);
}

#[test]
fn a_500_index_fails_and_the_final_update_names_it() {
    let rt = runtime();
    let mock = rt.block_on(MockServer::start());
    rt.block_on(
        Mock::given(method("GET"))
            .and(path("/api/cooked/v1/index"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock),
    );
    let dir = tempfile::tempdir().unwrap();
    let profile = Profile::Firesky(dir.path().to_path_buf());
    let servers = server_at(&mock.uri());
    let cancel = CancellationToken::new();
    let (report, last) = run_presync(profile, &servers, Duration::from_secs(10), &cancel);
    let failure = last.failure.expect("a failed run names its failure");
    assert_eq!(failure.http_status, Some(500));
    assert_eq!(last.server, server_label(&mock.uri()));
    assert_eq!(gate(&report), Gate::Blocked);
}

#[test]
fn a_silent_server_hits_the_deadline() {
    // The listener accepts into its backlog and never answers.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let profile = Profile::Firesky(dir.path().to_path_buf());
    let servers = server_at(&url);
    let cancel = CancellationToken::new();
    let (_, last) = run_presync(profile, &servers, Duration::from_millis(300), &cancel);
    let failure = last.failure.expect("a failed run names its failure");
    assert_eq!(failure.reason, "deadline");
    assert_eq!(failure.step, Step::Index);
    drop(listener);
}

#[test]
fn cancel_gives_cancelled_and_the_cancelled_gate() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let profile = Profile::Firesky(dir.path().to_path_buf());
    let servers = server_at(&url);
    let rt = runtime();
    let (tx, rx) = watch::channel(None);
    let content = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let stopper = cancel.clone();
    let report = rt.block_on(async {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            stopper.cancel();
        });
        presync_at(
            profile,
            content.path(),
            &servers,
            Timeouts::DEFAULT,
            Duration::from_secs(30),
            &cancel,
            &tx,
        )
        .await
    });
    assert_eq!(report.outcome, Outcome::Cancelled);
    assert_eq!(gate(&report), Gate::Cancelled);
    let last = rx.borrow().clone().expect("the final update is sent");
    assert_eq!(last.outcome, Some(Outcome::Cancelled));
    assert_eq!(last.failure, None);
    drop(listener);
}

#[test]
fn finished_event_carries_the_failure_codes_and_no_host() {
    let rt = runtime();
    let mock = rt.block_on(MockServer::start());
    rt.block_on(
        Mock::given(method("GET"))
            .and(path("/api/cooked/v1/index"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock),
    );
    let dir = tempfile::tempdir().unwrap();
    let profile = Profile::Firesky(dir.path().to_path_buf());
    let servers = server_at(&mock.uri());
    let cancel = CancellationToken::new();
    let content = tempfile::tempdir().unwrap();
    let (tx, _rx) = watch::channel(None);
    let (_, captured) = capture(|| {
        rt.block_on(presync(
            Uuid::nil(),
            profile,
            content.path(),
            &servers,
            &cancel,
            &tx,
        ))
    });
    let finished = captured
        .iter()
        .find(|c| c.field("event") == Some("cooked_presync_finished"))
        .expect("the finished event is logged");
    assert_eq!(finished.level, tracing::Level::WARN);
    assert_eq!(finished.field("step"), Some("index"));
    assert_eq!(finished.field("reason"), Some("http_error"));
    assert_eq!(finished.field("http_status"), Some("500"));
    for event in captured.iter().filter(|c| c.target.starts_with("launcher")) {
        for value in event.fields.values() {
            assert!(!value.contains("127.0.0.1"), "a field names the host");
        }
    }
}

#[test]
fn category_events_carry_closed_codes_and_numbers() {
    let entry = CategoryReport {
        category_id: 4,
        pak: "CookedDataItems.pak".into(),
        action: CategoryAction::Written,
        cache_version: Some(1),
        source_version: None,
        served_version: 2,
        changed: 3,
        bytes: 512,
    };
    let mut done = report(Outcome::Synced);
    done.categories = vec![entry];
    let (_, captured) = capture(|| emit_events(Uuid::nil(), &done));
    let category = captured
        .iter()
        .find(|c| c.field("event") == Some("cooked_presync_category"))
        .expect("the category event is logged");
    assert_eq!(category.level, tracing::Level::INFO);
    assert_eq!(category.field("action"), Some("written"));
    assert_eq!(category.field("pak"), Some("CookedDataItems.pak"));
    assert_eq!(category.field("category_id"), Some("4"));
    assert_eq!(category.field("source_version"), None);
    let finished = captured
        .iter()
        .find(|c| c.field("event") == Some("cooked_presync_finished"))
        .expect("the finished event is logged");
    assert_eq!(finished.level, tracing::Level::INFO);
    assert_eq!(finished.field("categories_written"), Some("1"));
    assert_eq!(finished.field("step"), None);
}

#[test]
fn gate_continues_only_on_up_to_date_synced_and_skipped() {
    let table = [
        (Outcome::UpToDate, Gate::Continue),
        (Outcome::Synced, Gate::Continue),
        (Outcome::Skipped, Gate::Continue),
        (Outcome::Failed, Gate::Blocked),
        (Outcome::Cancelled, Gate::Cancelled),
    ];
    for (outcome, expected) in table {
        assert_eq!(gate(&report(outcome)), expected, "{outcome:?}");
    }
}

#[test]
fn timer_outcome_maps_each_report_outcome() {
    use phase_timing::Outcome as Timer;
    assert!(matches!(
        timer_outcome(&report(Outcome::UpToDate)),
        Timer::Succeeded
    ));
    assert!(matches!(
        timer_outcome(&report(Outcome::Synced)),
        Timer::Succeeded
    ));
    assert!(matches!(
        timer_outcome(&report(Outcome::Skipped)),
        Timer::Unavailable
    ));
    assert!(matches!(
        timer_outcome(&report(Outcome::Failed)),
        Timer::Failed
    ));
    assert!(matches!(
        timer_outcome(&report(Outcome::Cancelled)),
        Timer::Cancelled
    ));
}
