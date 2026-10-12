//! The second Ctrl+C of a batch (the abort flag), and a `batch.jsonl` the summary
//! cannot read. No console: the flag is set by the test, as the handler would.

mod uat_support;

use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cimmeria_lab_cli::batch::{read_records, RunRecord, BATCH_JSON, BATCH_JSONL};
use cimmeria_lab_cli::commands::uat::batch_run::{
    batch_exit, give_up_on_abort, read_with_retry, run_batch, run_batch_reading, BatchInput,
    BatchOptions, Hooks, INTERRUPTED_ERROR,
};
use cimmeria_lab_cli::commands::uat::lanes::no_finish;
use cimmeria_lab_cli::commands::uat::plan::RunCall;
use cimmeria_lab_cli::commands::uat::progress::ProgressMode;
use cimmeria_lab_cli::commands::uat::summary::INTERRUPTED_LINE;
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::ui::Ui;
use serde_json::{json, Value};
use uat_support::rig::{pass_reply, Reply};
use uat_support::{ids, lane};

fn input() -> BatchInput {
    BatchInput {
        section: "first-session".to_string(),
        row_ids: ids(&["FS-P1", "FS-P2"]),
        lanes: vec![lane(1, "default")],
        specs_dir: None,
        call: RunCall::new("first-session", &[], None, None, None, &[], false),
        chaos: None,
        diff_golden: false,
    }
}

fn opts(runs: u32) -> BatchOptions {
    BatchOptions {
        runs_per_lease: runs,
        stagger_seconds: 0,
        run_timeout_minutes: 15,
        max_consecutive_failures: 0,
        mode: ProgressMode::None,
        quiet: false,
        json: false,
    }
}

fn pass2() -> Reply {
    pass_reply("C:\\runs\\a", &["FS-P1", "FS-P2"])
}

/// Run 1 passes; run 2 is under way on the daemon (a call that would take half
/// a minute) when the second Ctrl+C sets `abort`.
#[test]
fn a_second_ctrl_c_gives_up_on_the_run_and_still_writes_the_summary() {
    let root = tempfile::tempdir().unwrap();
    let (stop, abort) = (AtomicBool::new(false), AtomicBool::new(false));
    let calls = AtomicU32::new(0);
    let call = |_: &str, _: Duration| -> Reply {
        if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            return pass2();
        }
        stop.store(true, Ordering::SeqCst);
        abort.store(true, Ordering::SeqCst);
        give_up_on_abort(&abort, || {
            std::thread::sleep(Duration::from_secs(30));
            Ok(json!({"run_dir": "never", "rows": []}))
        })
    };
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
        abort: &abort,
    };
    let started = Instant::now();
    let out = run_batch(
        &input(),
        &opts(3),
        &Ui::plain(),
        root.path(),
        &hooks,
        &|_: &str| {},
    )
    .unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "the batch waited for the call"
    );

    // The files are on disk and say what happened.
    let md = std::fs::read_to_string(out.dir.join("summary.md")).unwrap();
    assert!(md.contains(INTERRUPTED_LINE), "{md}");
    let batch: Value =
        serde_json::from_str(&std::fs::read_to_string(out.dir.join(BATCH_JSON)).unwrap()).unwrap();
    assert_eq!(batch["summary"]["interrupted"], json!(true));
    let records = read_records(&out.dir.join(BATCH_JSONL)).unwrap();
    assert_eq!(records.len(), 2, "run 3 never started");
    assert!(records[0].ok);
    assert!(
        records[1]
            .error
            .as_deref()
            .is_some_and(|e| e.contains(INTERRUPTED_ERROR)),
        "{:?}",
        records[1].error
    );
    assert!(out.summary.interrupted);
    assert_eq!((out.summary.runs, out.summary.passed), (2, 1));
    // The exit code is 130 whatever the summary says (a pass-only batch included).
    assert_eq!(batch_exit(&out, true), 130);
    assert_eq!(batch_exit(&out, false), out.exit);
}

#[test]
fn a_batch_nobody_interrupted_is_not_marked() {
    let root = tempfile::tempdir().unwrap();
    let call = |_: &str, _: Duration| -> Reply { pass2() };
    let stop = AtomicBool::new(false);
    let abort = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &|_: &str, _: Duration| true,
        sleep: &|_: Duration| {},
        finish: &no_finish,
        stop: &stop,
        abort: &abort,
    };
    let out = run_batch(
        &input(),
        &opts(1),
        &Ui::plain(),
        root.path(),
        &hooks,
        &|_: &str| {},
    )
    .unwrap();
    assert!(!out.summary.interrupted);
    assert!(!out.md.contains("Interrupted"));
    assert_eq!(out.exit, 0);
}

#[test]
fn a_call_that_finishes_before_the_abort_returns_its_reply() {
    let abort = AtomicBool::new(false);
    let reply = give_up_on_abort(&abort, || Ok(json!({"ok": true}))).unwrap();
    assert_eq!(reply, json!({"ok": true}));
    let error = give_up_on_abort(&abort, || Err(McpError::Tool("boom".into()))).unwrap_err();
    assert!(matches!(error, McpError::Tool(m) if m == "boom"));
}

fn record() -> RunRecord {
    RunRecord {
        instance: "default".into(),
        index: 1,
        ok: true,
        ..RunRecord::default()
    }
}

#[test]
fn a_transient_read_error_is_retried() {
    let tries = AtomicU32::new(0);
    let read = || -> io::Result<Vec<RunRecord>> {
        if tries.fetch_add(1, Ordering::SeqCst) < 2 {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            Ok(vec![record()])
        }
    };
    let records = read_with_retry(&read, 5, Duration::from_millis(1)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(tries.load(Ordering::SeqCst), 3);
}

#[test]
fn a_read_that_keeps_failing_gives_up_after_the_attempts() {
    let tries = AtomicU32::new(0);
    let read = || -> io::Result<Vec<RunRecord>> {
        tries.fetch_add(1, Ordering::SeqCst);
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    };
    assert!(read_with_retry(&read, 4, Duration::from_millis(1)).is_err());
    assert_eq!(tries.load(Ordering::SeqCst), 4);
}

/// The file stays locked for the summary: the batch still ends with a summary of
/// the runs the lanes reported, not an error (exit 3).
#[test]
fn an_unreadable_batch_jsonl_still_gives_a_summary_of_the_reported_runs() {
    let root = tempfile::tempdir().unwrap();
    let calls = Mutex::new(0u32);
    let call = |_: &str, _: Duration| -> Reply {
        *calls.lock().unwrap() += 1;
        pass2()
    };
    let (stop, abort) = (AtomicBool::new(false), AtomicBool::new(false));
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &|_: &str, _: Duration| true,
        sleep: &|_: Duration| {},
        finish: &no_finish,
        stop: &stop,
        abort: &abort,
    };
    let locked = |_: &std::path::Path| -> io::Result<Vec<RunRecord>> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    };
    let out = run_batch_reading(
        &input(),
        &opts(2),
        &Ui::plain(),
        root.path(),
        &hooks,
        &|_: &str| {},
        &locked,
    )
    .expect("a locked batch.jsonl is not an error");
    assert_eq!((out.summary.runs, out.summary.passed, out.exit), (2, 2, 0));
    assert!(out.dir.join("summary.md").exists() && out.dir.join(BATCH_JSON).exists());
}
