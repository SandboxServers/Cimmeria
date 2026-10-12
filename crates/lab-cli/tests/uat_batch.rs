mod uat_support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use cimmeria_lab_cli::batch::{
    find_latest_batch, read_plan, read_records, BATCH_JSON, BATCH_JSONL, PLAN_JSON,
};
use cimmeria_lab_cli::commands::uat::batch_run::{run_batch, BatchInput, BatchOptions, Hooks};
use cimmeria_lab_cli::commands::uat::lanes::no_finish;
use cimmeria_lab_cli::commands::uat::plan::RunCall;
use cimmeria_lab_cli::commands::uat::progress::ProgressMode;
use cimmeria_lab_cli::ui::Ui;
use serde_json::Value;
use uat_support::rig::{frost_reply, pass_reply, scripted, Reply};
use uat_support::{ids, lane, lanes2};

fn input(lanes: Vec<cimmeria_lab_cli::batch::PlanLane>) -> BatchInput {
    BatchInput {
        section: "first-session".to_string(),
        row_ids: ids(&["FS-P1", "FS-P2"]),
        lanes,
        specs_dir: None,
        call: RunCall::new("first-session", &[], None, None, None, &[], false),
        chaos: None,
        diff_golden: false,
    }
}

fn opts(runs: u32, max: u32, mode: ProgressMode) -> BatchOptions {
    BatchOptions {
        runs_per_lease: runs,
        stagger_seconds: 0,
        run_timeout_minutes: 15,
        max_consecutive_failures: max,
        mode,
        quiet: false,
        json: false,
    }
}

fn pass2() -> Reply {
    pass_reply("C:\\runs\\a", &["FS-P1", "FS-P2"])
}

fn plain() -> Ui {
    Ui::plain()
}

#[test]
fn a_two_lane_batch_writes_plan_first_then_records_then_the_summary() {
    let root = tempfile::tempdir().unwrap();
    let inner = scripted(vec![
        ("default", vec![pass2(), pass2()]),
        ("p2", vec![frost_reply(), pass2()]),
    ]);
    let plan_first = AtomicBool::new(false);
    // Held across the first call's check: no lane can append a record (which
    // needs its call to return) until the check has looked at the folder.
    let checked = Mutex::new(false);
    let root_path = root.path().to_path_buf();
    let call = |instance: &str, timeout: Duration| {
        {
            let mut done = checked.lock().unwrap();
            if !*done {
                *done = true;
                let dir = find_latest_batch(&root_path).unwrap().unwrap();
                plan_first.store(
                    dir.join(PLAN_JSON).exists() && !dir.join(BATCH_JSONL).exists(),
                    Ordering::SeqCst,
                );
            }
        }
        inner(instance, timeout)
    };
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let stop = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
    };
    let lines = Mutex::new(Vec::<String>::new());
    let emit = |l: &str| lines.lock().unwrap().push(l.to_string());

    let out = run_batch(
        &input(lanes2()),
        &opts(2, 3, ProgressMode::Host),
        &plain(),
        root.path(),
        &hooks,
        &emit,
    )
    .unwrap();

    assert!(
        plan_first.load(Ordering::SeqCst),
        "plan.json must exist before the first run"
    );
    assert_eq!(out.dir.parent().unwrap(), root.path());
    let name = out.dir.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        regex::Regex::new(r"^batch-\d{8}-\d{6}$")
            .unwrap()
            .is_match(&name),
        "{name}"
    );

    let plan = read_plan(&out.dir.join(PLAN_JSON)).unwrap();
    assert_eq!(
        (plan.schema, plan.section.as_str(), plan.runs_per_lease),
        (1, "first-session", 2)
    );
    assert_eq!(plan.rows, ["FS-P1", "FS-P2"]);
    assert_eq!(plan.lanes.len(), 2);
    assert_eq!(plan.pid, Some(std::process::id()));
    assert!(
        regex::Regex::new(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{7}Z$")
            .unwrap()
            .is_match(&plan.started),
        "{}",
        plan.started
    );

    assert_eq!(read_records(&out.dir.join(BATCH_JSONL)).unwrap().len(), 4);
    assert_eq!(out.exit, 1);
    assert_eq!(
        (
            out.summary.runs,
            out.summary.passed,
            out.summary.failed_known
        ),
        (4, 3, 1)
    );

    let dir = out.dir.display().to_string();
    let mut expected: Vec<String> = [
        "# lab uat first-session: FAIL (3/4 runs passed)",
        "Failed: 1 (1 known issue, 0 new)",
        "",
        "| Lane | Instance | Passed | Stopped early |",
        "|---|---|---|---|",
        "| 1 | default | 2/2 |  |",
        "| 2 | p2 | 1/2 |  |",
        "",
        "| Row | Passed |",
        "|---|---|",
        "| FS-P1 | 4/4 |",
        "| FS-P2 | 3/4 |",
        "",
        "Failures, grouped by the first failing row:",
        "- 1x FS-P2 FAIL: clause movie-over failed (known: #1341); runs: p2#1",
        "",
    ]
    .iter()
    .map(|l| l.to_string())
    .collect();
    expected.push(format!("Batch: {dir}"));
    let md: String = expected.iter().map(|l| format!("{l}\r\n")).collect();
    assert_eq!(out.md, md);
    assert_eq!(
        std::fs::read_to_string(out.dir.join("summary.md")).unwrap(),
        format!("{md}\r\n")
    );
    let batch: Value =
        serde_json::from_str(&std::fs::read_to_string(out.dir.join(BATCH_JSON)).unwrap()).unwrap();
    assert_eq!(batch["summary"]["failed"], 1);

    let mut got = lines.lock().unwrap().clone();
    assert_eq!(
        got.remove(0),
        "lab uat first-session: 2 lease(s) x 2 run(s) = 4 run(s) on default, p2; rows FS-P1, FS-P2"
    );
    got.sort();
    assert_eq!(
        got,
        [
            "[default 1/2] PASS (0s)",
            "[default 2/2] PASS (0s)",
            "[p2 1/2] FAIL FS-P2: clause movie-over failed [known #1341] (0s)",
            "[p2 2/2] PASS (0s)",
        ]
    );
}

#[test]
fn a_braked_lane_exits_4_marks_the_lane_and_says_so() {
    let root = tempfile::tempdir().unwrap();
    let fails: Vec<Reply> = (0..3).map(|_| frost_reply()).collect();
    let call = scripted(vec![("default", fails)]);
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let stop = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
    };
    let lines = Mutex::new(Vec::<String>::new());
    let emit = |l: &str| lines.lock().unwrap().push(l.to_string());
    let out = run_batch(
        &input(vec![lane(1, "default")]),
        &opts(3, 2, ProgressMode::Host),
        &plain(),
        root.path(),
        &hooks,
        &emit,
    )
    .unwrap();
    assert_eq!(out.exit, 4);
    assert!(out.summary.lanes[0].braked);
    assert!(
        out.md.contains("| 1 | default | 0/2 | yes |\r\n"),
        "{}",
        out.md
    );
    assert!(lines
        .lock()
        .unwrap()
        .contains(&"[default] stopped after 2 failed runs in a row".to_string()));
}

#[test]
fn ctrl_c_still_writes_the_summary_from_the_runs_that_finished() {
    let root = tempfile::tempdir().unwrap();
    let stop = AtomicBool::new(false);
    let inner = scripted(vec![("default", vec![pass2(), pass2(), pass2()])]);
    let call = |instance: &str, timeout: Duration| {
        let reply = inner(instance, timeout);
        stop.store(true, Ordering::SeqCst); // Ctrl+C during run 1
        reply
    };
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
    };
    let emit = |_: &str| {};
    let out = run_batch(
        &input(vec![lane(1, "default")]),
        &opts(3, 3, ProgressMode::Host),
        &plain(),
        root.path(),
        &hooks,
        &emit,
    )
    .unwrap();
    assert_eq!((out.summary.runs, out.summary.passed, out.exit), (1, 1, 0));
    assert!(out
        .md
        .starts_with("# lab uat first-session: PASS (1/1 runs passed)"));
    assert!(out.dir.join("summary.md").exists() && out.dir.join(BATCH_JSON).exists());
}

#[test]
fn an_unwritable_batch_root_is_an_error_before_any_run() {
    let root = tempfile::NamedTempFile::new().unwrap(); // a file, not a folder
    let calls = Mutex::new(0u32);
    let call = |_: &str, _: Duration| -> Reply {
        *calls.lock().unwrap() += 1;
        pass2()
    };
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let stop = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
    };
    let emit = |_: &str| {};
    let err = run_batch(
        &input(vec![lane(1, "default")]),
        &opts(1, 3, ProgressMode::Host),
        &plain(),
        root.path(),
        &hooks,
        &emit,
    )
    .err()
    .expect("an error");
    assert!(err.starts_with("cannot write the batch folder "), "{err}");
    assert_eq!(*calls.lock().unwrap(), 0);
}

#[test]
fn none_mode_prints_nothing_and_stderr_mode_still_prints() {
    for (mode, expect_lines) in [(ProgressMode::None, false), (ProgressMode::Stderr, true)] {
        let root = tempfile::tempdir().unwrap();
        let call = scripted(vec![("default", vec![pass2()])]);
        let wait = |_: &str, _: Duration| true;
        let sleep = |_: Duration| {};
        let stop = AtomicBool::new(false);
        let hooks = Hooks {
            call: &call,
            wait_lease_free: &wait,
            sleep: &sleep,
            finish: &no_finish,
            stop: &stop,
        };
        let lines = Mutex::new(Vec::<String>::new());
        let emit = |l: &str| lines.lock().unwrap().push(l.to_string());
        run_batch(
            &input(vec![lane(1, "default")]),
            &opts(1, 3, mode),
            &plain(),
            root.path(),
            &hooks,
            &emit,
        )
        .unwrap();
        assert_eq!(!lines.lock().unwrap().is_empty(), expect_lines, "{mode:?}");
    }
}

#[test]
fn fancy_mode_prints_the_mock_up_lines() {
    let root = tempfile::tempdir().unwrap();
    let call = scripted(vec![("default", vec![pass2()])]);
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let stop = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &no_finish,
        stop: &stop,
    };
    let lines = Mutex::new(Vec::<String>::new());
    let emit = |l: &str| lines.lock().unwrap().push(l.to_string());
    let ui = Ui {
        color: false,
        fancy: true,
        width: None,
    };
    run_batch(
        &input(vec![lane(1, "default")]),
        &opts(1, 3, ProgressMode::Host),
        &ui,
        root.path(),
        &hooks,
        &emit,
    )
    .unwrap();
    let got = lines.lock().unwrap();
    assert!(
        got[0].starts_with("▶ first-session · 2 rows · 1 lane × 1 run · batch-"),
        "{}",
        got[0]
    );
    assert_eq!(got[1], "  default 1/1  ✓ pass  2/2 rows  0s");
}
