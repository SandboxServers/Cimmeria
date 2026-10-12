//! LX-17b: a chaos run through the lanes: the run's own header reaches the
//! summary, the golden diff reports chaos_divergence, the record keeps the header.

mod uat_support;

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use cimmeria_lab_cli::batch::{read_records, PlanLane, BATCH_JSONL};
use cimmeria_lab_cli::commands::uat::batch_run::{
    finish_with, run_batch, BatchInput, BatchOptions, BatchOutcome, Hooks,
};
use cimmeria_lab_cli::commands::uat::golden::DiffArgs;
use cimmeria_lab_cli::commands::uat::plan::RunCall;
use cimmeria_lab_cli::commands::uat::progress::ProgressMode;
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::ui::Ui;
use serde_json::{json, Value};

const PLAN_HEADER: &str = "chaos=cpu:safe seed=<per run>";
const RUN_HEADER: &str = "chaos=cpu:safe seed=4242";
const DIVERGENCE: &str = "row FS-01: expected `a`, got `b`";

/// One run of FS-01 that passes, answers the runner's own header, and whose
/// golden diff answers `golden_reply`.
fn one_chaos_run(root: &Path, golden_reply: Value) -> BatchOutcome {
    let call = |_: &str, _: Duration| -> Result<Value, McpError> {
        Ok(
            json!({"run_dir": r"C:\runs\a", "rows": [{"row": "FS-01", "result": "PASS"}], "chaos": [RUN_HEADER]}),
        )
    };
    let golden = move |_: Value| -> Result<Value, McpError> { Ok(golden_reply.clone()) };
    let diff = DiffArgs::new("first-session", None);
    let finish = finish_with(Some(&diff), &golden);
    let wait = |_: &str, _: Duration| true;
    let sleep = |_: Duration| {};
    let stop = AtomicBool::new(false);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &finish,
        stop: &stop,
    };
    let input = BatchInput {
        section: "first-session".to_string(),
        row_ids: vec!["FS-01".to_string()],
        lanes: vec![PlanLane {
            lane: 1,
            instance: "default".to_string(),
        }],
        specs_dir: None,
        call: RunCall::new(
            "first-session",
            &[],
            None,
            None,
            None,
            &["cpu:safe".to_string()],
            true,
        ),
        chaos: Some(PLAN_HEADER.to_string()),
        diff_golden: true,
    };
    let opts = BatchOptions {
        runs_per_lease: 1,
        stagger_seconds: 0,
        run_timeout_minutes: 15,
        max_consecutive_failures: 3,
        mode: ProgressMode::None,
        quiet: false,
        json: false,
    };
    let emit = |_: &str| {};
    run_batch(&input, &opts, &Ui::plain(), root, &hooks, &emit).unwrap()
}

/// tc:315 'a real run's summary carries the run's header, not the plan's'
#[test]
fn a_real_runs_summary_carries_the_runs_header_not_the_plans() {
    let root = tempfile::tempdir().unwrap();
    let out = one_chaos_run(root.path(), json!({"ok": true, "match": true}));
    assert_eq!(out.summary.chaos.as_deref(), Some(RUN_HEADER));
    assert!(out.md.starts_with(RUN_HEADER));
    assert_eq!(out.exit, 0);
}

/// tc:316 'a real chaos run is diffed and reports chaos_divergence (the header as
/// the chaos key), exit 0'
#[test]
fn a_real_chaos_run_is_diffed_and_reports_chaos_divergence() {
    let root = tempfile::tempdir().unwrap();
    let out = one_chaos_run(
        root.path(),
        json!({"ok": true, "match": false, "first": DIVERGENCE}),
    );
    assert!(out.summary.golden.is_none());
    let cd = out.summary.chaos_divergence.as_ref().unwrap();
    assert_eq!((cd.diverged, cd.groups[0].line.as_str()), (1, DIVERGENCE));
    assert_eq!(cd.groups[0].runs, ["default#1 seed=4242"]);
    assert_eq!(
        out.exit, 0,
        "a chaos divergence is what a chaos run reports"
    );
}

/// tc:319 'the lane writes the run header into its batch.jsonl record'
#[test]
fn the_lane_writes_the_run_header_into_its_record() {
    let root = tempfile::tempdir().unwrap();
    let out = one_chaos_run(
        root.path(),
        json!({"ok": true, "match": false, "first": DIVERGENCE}),
    );
    let records = read_records(&out.dir.join(BATCH_JSONL)).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].chaos.as_deref(), Some(RUN_HEADER));
    assert_eq!(records[0].golden.as_deref(), Some(DIVERGENCE));
}
