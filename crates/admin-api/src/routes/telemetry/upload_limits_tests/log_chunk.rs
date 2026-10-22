//! `POST /api/telemetry/upload-chunk` under a launcher log session (C-LF4..6):
//! the log limits, the target lists, the whole-chunk budgets and the gate's
//! 20 chunks a minute. An event session's limits are checked beside them.

use std::time::Instant;

use axum::http::header::RETRY_AFTER;
use axum::response::IntoResponse;

use crate::routes::dev_session::AuthError;
use crate::routes::telemetry::chunk::chunk_inner_with;
use crate::routes::telemetry::dto::{ChunkResponse, IngestError};
use crate::routes::telemetry::launcher_budget::LauncherBudget;
use crate::routes::telemetry::launcher_log_budget::LogBudget;
use crate::routes::telemetry::replay_tests::capture_async;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{gzip, launcher_chunk_request, log_chunk_request, run, Env, PEER};

const INSTALL: &str = "install-log-test";

/// One valid launcher log row.
fn log_row(seq: usize, target: &str, message: &str) -> String {
    format!(
        r#"{{"type":"client_native","ts_ms":1,"seq":{seq},"target":"{target}","level":"info","fields":{{"message":"{message}","upload_id":"0123456789abcdef","log_level":"INFO"}}}}"#
    )
}

fn event_row(seq: usize) -> String {
    format!(
        r#"{{"type":"client_native","ts_ms":1,"seq":{seq},"target":"client.launcher.adoption","level":"info","fields":{{"step":"start"}}}}"#
    )
}

/// `rows` gzipped as NDJSON.
fn body(rows: &[String]) -> Vec<u8> {
    gzip(rows.join("\n").as_bytes())
}

fn log_rows(n: usize) -> Vec<String> {
    (0..n)
        .map(|seq| log_row(seq, "client.launcher.log", "a launcher log line"))
        .collect()
}

/// A policy whose install and address windows do not get in the way of the
/// session window under test.
fn wide_policy() -> UploadPolicy {
    UploadPolicy {
        launcher_chunk_per_install: 1_000,
        launcher_chunk_per_ip: 1_000,
        ..UploadPolicy::defaults()
    }
}

fn post(
    state: &UploadState,
    budget: &LauncherBudget,
    sid: &str,
    rows: &[String],
) -> Result<ChunkResponse, IngestError> {
    run(chunk_inner_with(
        state,
        &wide_policy(),
        PEER,
        log_chunk_request(sid, INSTALL, body(rows)),
        Instant::now(),
        budget,
    ))
}

fn state() -> UploadState {
    UploadState::new(UploadLimits::default())
}

/// **A 1,000-row log chunk is admitted.** On the event path's 200-row limit it
/// would be cut at 200, so this fails if the log session takes the event limits.
#[test]
fn a_1000_row_log_chunk_is_admitted() {
    let _env = Env::install();
    let resp = post(
        &state(),
        &LauncherBudget::new(300, 3_000),
        "sess-log-1000",
        &log_rows(1_000),
    )
    .expect("1,000 log rows fit one chunk");
    assert_eq!((resp.accepted, resp.truncated), (1_000, false));
}

/// **A 1,001-row log chunk is refused with 413**, not cut.
#[test]
fn a_1001_row_log_chunk_is_refused_with_413() {
    let _env = Env::install();
    let err = post(
        &state(),
        &LauncherBudget::new(300, 3_000),
        "sess-log-1001",
        &log_rows(1_001),
    )
    .expect_err("1,001 rows are over the chunk cap");
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "rows",
                limit: 1_000
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.into_response().status(), 413);
}

/// **A log chunk body over 64 KiB compressed is refused with 413.** The cap is
/// the C-LF6 limit, not the event path's.
#[test]
fn a_log_chunk_body_over_64_kib_is_refused_with_413() {
    let _env = Env::install();
    let err = run(chunk_inner_with(
        &state(),
        &wide_policy(),
        PEER,
        log_chunk_request("sess-log-body", INSTALL, vec![0u8; 64 * 1024 + 1]),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect_err("over the body cap");
    assert!(matches!(err, IngestError::TooLarge(..)), "{err:?}");
    assert_eq!(err.into_response().status(), 413);
}

/// **A log chunk over 512 KiB expanded is refused with 413.** The rows compress
/// far below 64 KiB, so only the decompressed cap binds.
#[test]
fn a_log_chunk_over_512_kib_expanded_is_refused_with_413() {
    let _env = Env::install();
    let message = "x".repeat(3_900);
    let rows: Vec<String> = (0..150)
        .map(|seq| log_row(seq, "client.launcher.log", &message))
        .collect();
    let err = post(
        &state(),
        &LauncherBudget::new(300, 3_000),
        "sess-log-exp",
        &rows,
    )
    .expect_err("over the decompressed cap");
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "decompressed bytes",
                limit: 524_288
            }
        ),
        "{err:?}"
    );
    assert_eq!(err.into_response().status(), 413);
}
/// **A log row over 4 KiB is dropped and counted**; the rows beside it stay.
#[test]
fn a_log_row_over_4_kib_is_dropped() {
    let _env = Env::install();
    let rows = vec![
        log_row(1, "client.launcher.log", &"x".repeat(5_000)),
        log_row(2, "client.launcher.log", "short"),
    ];
    let resp = post(
        &state(),
        &LauncherBudget::new(300, 3_000),
        "sess-log-row",
        &rows,
    )
    .expect("the chunk is accepted");
    assert_eq!((resp.accepted, resp.bad_rows), (1, 1));
}

/// **The 21st chunk of 1,000 rows in a minute gets 429, and none of it is
/// written.** The first twenty are admitted; a log session's gate window is
/// 20, where an event session's is 10.
#[test]
fn the_21st_log_chunk_in_a_minute_is_refused_and_writes_nothing() {
    let _env = Env::install();
    let state = state();
    let budget = LauncherBudget::new(300, 3_000);
    let rows = log_rows(1_000);
    for n in 0..20 {
        post(&state, &budget, "sess-log-21", &rows)
            .unwrap_or_else(|e| panic!("chunk {n} is within the window: {e:?}"));
    }
    let (result, replayed) = run(capture_async(chunk_inner_with(
        &state,
        &wide_policy(),
        PEER,
        log_chunk_request("sess-log-21", INSTALL, body(&rows)),
        Instant::now(),
        &budget,
    )));
    let err = result.expect_err("the 21st chunk is over the window");
    match &err {
        IngestError::Auth(AuthError::QuotaExceeded(q)) => {
            assert_eq!(q.scope, "upload/launcher_session");
            assert!(q.retry_after_secs >= 1);
        }
        other => panic!("expected a session quota refusal, got {other:?}"),
    }
    assert_eq!(err.into_response().status(), 429);
    let written = replayed
        .iter()
        .filter(|r| r.target == "client.native")
        .count();
    assert_eq!(written, 0, "no row of the refused chunk is written");
}

/// **The session budget refuses a whole chunk with 429 and no `Retry-After`.**
#[test]
fn the_session_row_budget_refuses_a_whole_chunk() {
    let _env = Env::install();
    let state = state();
    let budget = LauncherBudget::new(300, 3_000).with_logs(LogBudget::new(1_500, 80_000, 40_000));
    let rows = log_rows(1_000);
    post(&state, &budget, "sess-log-budget", &rows).expect("the first chunk fits");
    let err = post(&state, &budget, "sess-log-budget", &rows).expect_err("1,000 more do not");
    assert_eq!(err.reason(), "log_session_budget");
    let resp = err.into_response();
    assert_eq!(resp.status(), 429);
    assert!(resp.headers().get(RETRY_AFTER).is_none());
}

/// **The install-day budget answers 429 with a `Retry-After` up to midnight.**
#[test]
fn the_install_day_budget_carries_a_retry_after() {
    let _env = Env::install();
    let state = state();
    let budget = LauncherBudget::new(300, 3_000).with_logs(LogBudget::new(40_000, 1_500, 40_000));
    let rows = log_rows(1_000);
    post(&state, &budget, "sess-day-a", &rows).expect("the first chunk fits");
    let err = post(&state, &budget, "sess-day-b", &rows).expect_err("the install is spent");
    assert_eq!(err.reason(), "log_install_budget");
    let resp = err.into_response();
    assert_eq!(resp.status(), 429);
    let secs: u64 = resp.headers()[RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=86_400).contains(&secs), "{secs}");
}

/// **The global ceiling answers 503 with a `Retry-After`.**
#[test]
fn the_global_log_ceiling_is_a_503() {
    let _env = Env::install();
    let state = state();
    let budget = LauncherBudget::new(300, 3_000).with_logs(LogBudget::new(40_000, 80_000, 1_500));
    let rows = log_rows(1_000);
    post(&state, &budget, "sess-glob-a", &rows).expect("the first chunk fits");
    let err = post(&state, &budget, "sess-glob-b", &rows).expect_err("the minute is spent");
    assert_eq!(err.reason(), "log_global_budget");
    let resp = err.into_response();
    assert_eq!(resp.status(), 503);
    assert!(resp.headers().get(RETRY_AFTER).is_some());
}

/// **The env override changes the session budget.** With 500 rows per
/// session, a 600-row chunk is refused and a 500-row one is not.
#[test]
fn the_env_override_changes_the_session_budget() {
    let _env = Env::install();
    const VAR: &str = "CIMMERIA_TELEMETRY_LOG_ROWS_PER_SESSION";
    struct Reset(Option<String>);
    impl Drop for Reset {
        fn drop(&mut self) {
            match self.0.take() {
                Some(v) => std::env::set_var(VAR, v),
                None => std::env::remove_var(VAR),
            }
        }
    }
    let _reset = Reset(std::env::var(VAR).ok());
    std::env::set_var(VAR, "500");
    let state = state();
    let budget = LauncherBudget::from_env();
    let err = post(&state, &budget, "sess-env-a", &log_rows(600)).expect_err("600 > 500");
    assert_eq!(err.reason(), "log_session_budget");
    post(&state, &budget, "sess-env-b", &log_rows(500)).expect("500 fit");
}

/// **An event token posting a log target gets 403.**
#[test]
fn an_event_token_posting_a_log_target_gets_403() {
    let _env = Env::install();
    let err = run(chunk_inner_with(
        &state(),
        &UploadPolicy::defaults(),
        PEER,
        launcher_chunk_request(
            "sess-event-logs",
            body(&[log_row(1, "client.launcher.log", "hi")]),
        ),
        Instant::now(),
        &LauncherBudget::new(300, 3_000),
    ))
    .expect_err("an event session may not post log rows");
    assert_eq!(err.reason(), "purpose_target");
    assert_eq!(err.into_response().status(), 403);
}

/// **A log token posting an event target gets 403**, even beside good rows.
#[test]
fn a_log_token_posting_an_event_target_gets_403() {
    let _env = Env::install();
    let rows = vec![log_row(1, "client.launcher.log", "ok"), event_row(2)];
    let err = post(
        &state(),
        &LauncherBudget::new(300, 3_000),
        "sess-log-events",
        &rows,
    )
    .expect_err("a log session may not post event rows");
    assert_eq!(err.reason(), "purpose_target");
    assert_eq!(err.into_response().status(), 403);
}

/// **An event session's budget and row limit are unchanged.** Two chunks of
/// 200 adoption rows against the 300-row session budget: 200 accepted, then
/// 100 accepted and 100 suppressed.
#[test]
fn an_event_session_keeps_its_300_row_budget() {
    let _env = Env::install();
    let state = state();
    let budget = LauncherBudget::new(300, 3_000);
    let rows: Vec<String> = (0..200).map(event_row).collect();
    let send = |sid: &str| {
        run(chunk_inner_with(
            &state,
            &UploadPolicy::defaults(),
            PEER,
            launcher_chunk_request(sid, body(&rows)),
            Instant::now(),
            &budget,
        ))
        .expect("the event chunk is accepted")
    };
    let first = send("sess-event-300");
    assert_eq!((first.accepted, first.suppressed), (200, 0));
    let second = send("sess-event-300");
    assert_eq!((second.accepted, second.suppressed), (100, 100));
}
