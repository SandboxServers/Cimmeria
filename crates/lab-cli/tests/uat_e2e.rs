//! LX-18c2 (F-LX12): `lab uat` end to end through the real binary: the lab home
//! names a scripted MCP stub, the batch folder is read back from disk.

mod support;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use cimmeria_lab_cli::batch::{read_records, RunRecord, BATCH_JSONL};
use serde_json::{json, Value};
use support::{lab_home_for, run_lab_in, StubServer, ToolReply, STUB_TOKEN};
use tempfile::TempDir;

/// One hosted, free instance.
const STATUS: &str = r#"{"daemon":{"pid":1,"version":"x"},"instances":[{"instance":"default","client_pid":null,"lease":{"held":false}}]}"#;

/// True when an `SGW.exe` runs on this machine: the pre-flight then refuses to
/// drive (exit 3), so the tests that drive skip.
fn sgw_running() -> bool {
    if !cfg!(windows) {
        return false;
    }
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq SGW.exe", "/NH"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_ascii_lowercase()
                .contains("sgw.exe")
        })
        .unwrap_or(false)
}

macro_rules! need_no_sgw {
    () => {
        if sgw_running() {
            eprintln!("skipped: an SGW.exe runs, so the pre-flight would refuse to drive");
            return;
        }
    };
}

/// The plan call's reply: every row ready.
fn plan(rows: &[&str]) -> ToolReply {
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| json!({"row": r, "result": "SKIPPED", "reasons": []}))
        .collect();
    ToolReply::Json(json!({ "rows": rows }))
}

/// A run where every row passed.
fn pass(rows: &[&str]) -> ToolReply {
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| json!({"row": r, "result": "PASS"}))
        .collect();
    ToolReply::Json(json!({"run_dir": "C:\\runs\\a", "rows": rows}))
}

/// A run where FS-P1 passed and `row` failed with `reason`.
fn fail(row: &str, reason: &str) -> ToolReply {
    ToolReply::Json(json!({"run_dir": "C:\\runs\\f", "rows": [
        {"row": "FS-P1", "result": "PASS"},
        {"row": row, "result": "FAIL", "reasons": [reason]}]}))
}

struct Ran {
    out: Output,
    home: TempDir,
    stub: StubServer,
}

impl Ran {
    fn code(&self) -> Option<i32> {
        self.out.status.code()
    }

    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.out.stdout).into_owned()
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.out.stderr).into_owned()
    }

    /// stdout and stderr, for a failure message and for text that may go to either.
    fn both(&self) -> String {
        format!("{}\n{}", self.stdout(), self.stderr())
    }

    /// The one batch folder under `<home>\uat-runs`.
    fn batch_dir(&self) -> PathBuf {
        let root = self.home.path().join("uat-runs");
        let dirs: Vec<PathBuf> = fs::read_dir(&root)
            .unwrap_or_else(|e| panic!("no {}: {e}\n{}", root.display(), self.both()))
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect();
        assert_eq!(dirs.len(), 1, "one batch folder, got {dirs:?}");
        dirs.into_iter().next().unwrap()
    }

    fn records(&self) -> Vec<RunRecord> {
        read_records(&self.batch_dir().join(BATCH_JSONL)).unwrap()
    }

    fn batch_json(&self) -> Value {
        let text = fs::read_to_string(self.batch_dir().join("batch.json")).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn summary_md(&self) -> String {
        fs::read_to_string(self.batch_dir().join("summary.md")).unwrap()
    }

    fn calls_to(&self, tool: &str) -> Vec<Value> {
        self.stub
            .tool_calls()
            .into_iter()
            .filter(|(name, _)| name == tool)
            .map(|(_, args)| args)
            .collect()
    }
}

/// Runs `lab uat first-session --stagger-seconds 0 <args>` against a stub whose
/// tools answer from `tools`.
fn run_uat(tools: Vec<(&str, Vec<ToolReply>)>, args: &[&str]) -> Ran {
    let (stub, ep) = StubServer::start_mcp(STATUS, tools);
    let bind = ep.base_url.trim_start_matches("http://").to_string();
    let home = lab_home_for(&bind);
    let mut all = vec!["uat", "first-session", "--stagger-seconds", "0"];
    all.extend_from_slice(args);
    let out = run_lab_in(home.path(), &all);
    Ran { out, home, stub }
}

// ---- 1: a one-lane, two-row run ----

#[test]
fn a_one_lane_two_row_run_writes_the_batch_files_and_exits_0() {
    need_no_sgw!();
    let r = run_uat(
        vec![(
            "lab_uat_run",
            vec![plan(&["FS-P1", "FS-P2"]), pass(&["FS-P1", "FS-P2"])],
        )],
        &["--rows", "FS-P1,FS-P2"],
    );
    assert_eq!(r.code(), Some(0), "{}", r.both());
    let dir = r.batch_dir();
    for file in ["plan.json", "batch.jsonl", "batch.json", "summary.md"] {
        assert!(dir.join(file).is_file(), "{file} is missing in {dir:?}");
    }
    let plan: Value = serde_json::from_str(&fs::read_to_string(dir.join("plan.json")).unwrap())
        .expect("plan.json is JSON");
    assert_eq!(plan["section"], "first-session");
    assert_eq!(plan["rows"], json!(["FS-P1", "FS-P2"]));
    assert_eq!(plan["lanes"][0]["instance"], "default");
    let records = r.records();
    assert_eq!(records.len(), 1);
    assert!(records[0].ok && !records[0].braked);
    assert_eq!(records[0].rows.len(), 2);
    let batch = r.batch_json();
    assert_eq!(batch["section"], "first-session");
    assert_eq!(batch["summary"]["ok"], true);
    assert_eq!(batch["summary"]["runs"], 1);
    assert_eq!(batch["summary"]["passed"], 1);
    assert!(r.summary_md().contains("FS-P1"));

    // The wire: a plan call, then the run, each on its own session with the token.
    let calls = r.calls_to("lab_uat_run");
    assert_eq!(calls.len(), 2, "plan call and one run");
    assert_eq!(calls[0]["plan_only"], true);
    assert_eq!(calls[0]["instance"], "default");
    assert!(calls[1].get("plan_only").is_none(), "the run is not a plan");
    assert_eq!(calls[1]["instance"], "default");
    assert_eq!(calls[1]["sections"], json!(["first-session"]));
    let requests = r.stub.requests();
    let sessions_closed = requests.iter().filter(|q| q.method == "DELETE").count();
    assert_eq!(sessions_closed, 2, "both sessions are closed");
    for q in requests.iter().filter(|q| q.body.contains("tools/call")) {
        assert_eq!(q.header("mcp-session-id"), Some("stub-session"));
        assert_eq!(
            q.header("authorization"),
            Some(format!("Bearer {STUB_TOKEN}").as_str())
        );
    }
}

// ---- 2: --json prints one compact line ----

#[test]
fn json_prints_exactly_one_compact_line() {
    need_no_sgw!();
    let r = run_uat(
        vec![(
            "lab_uat_run",
            vec![plan(&["FS-P1", "FS-P2"]), pass(&["FS-P1", "FS-P2"])],
        )],
        &["--rows", "FS-P1,FS-P2", "--json"],
    );
    assert_eq!(r.code(), Some(0), "{}", r.both());
    let stdout = r.stdout();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout:?}");
    assert!(stdout.ends_with('\n'));
    let value: Value = serde_json::from_str(lines[0]).expect("the line is JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["runs"], 1);
    assert_eq!(value["passed"], 1);
    assert!(
        !lines[0].contains(": "),
        "compact, not pretty: {}",
        lines[0]
    );
    assert!(
        r.stderr().is_empty(),
        "no progress under --json: {}",
        r.stderr()
    );
}

// ---- 3: a failing row exits 1 and is grouped ----

#[test]
fn a_failing_row_exits_1_and_the_summary_groups_it() {
    need_no_sgw!();
    let r = run_uat(
        vec![(
            "lab_uat_run",
            vec![
                plan(&["FS-P1", "FS-P2"]),
                fail("FS-P2", "clause movie-over failed"),
            ],
        )],
        &["--rows", "FS-P1,FS-P2"],
    );
    assert_eq!(r.code(), Some(1), "{}", r.both());
    let records = r.records();
    assert_eq!(records.len(), 1);
    assert!(!records[0].ok);
    assert_eq!(
        records[0].verdict.as_ref().and_then(|v| v.row.as_deref()),
        Some("FS-P2")
    );
    let md = r.summary_md();
    assert!(
        md.contains("Failures, grouped by the first failing row:"),
        "{md}"
    );
    assert!(md.contains("FS-P2"), "{md}");
    assert!(r
        .stdout()
        .contains("Failures, grouped by the first failing row:"));
    let batch = r.batch_json();
    assert_eq!(batch["summary"]["ok"], false);
    assert_eq!(batch["summary"]["failed"], 1);
    assert!(batch["summary"]["failures"][0]["signature"]
        .as_str()
        .unwrap()
        .contains("FS-P2"));
}

// ---- 4: --diff-golden records values and exits 5 on a divergence ----

#[test]
fn diff_golden_records_each_runs_value_and_exits_5_on_a_divergence() {
    need_no_sgw!();
    let specs = TempDir::new().unwrap();
    fs::create_dir_all(specs.path().join("golden")).unwrap();
    fs::write(
        specs.path().join("golden").join("first-session.json"),
        r#"{"schema":1,"section":"first-session","fingerprint":{}}"#,
    )
    .unwrap();
    let specs_arg = specs.path().display().to_string();
    let divergence =
        "row FS-P1: expected dialogDisplay(dialog=3995), got dialogDisplay(dialog=2982)";
    let r = run_uat(
        vec![
            (
                "lab_uat_run",
                vec![plan(&["FS-P1"]), pass(&["FS-P1"]), pass(&["FS-P1"])],
            ),
            (
                "lab_golden_diff",
                vec![
                    ToolReply::Json(json!({"ok": true, "match": false, "first": divergence})),
                    ToolReply::Json(json!({"ok": true, "match": true})),
                ],
            ),
        ],
        &[
            "--rows",
            "FS-P1",
            "--runs-per-lease",
            "2",
            "--diff-golden",
            "--specs-dir",
            &specs_arg,
        ],
    );
    assert_eq!(r.code(), Some(5), "{}", r.both());
    let records = r.records();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|rec| rec.ok), "verdicts do not change");
    assert_eq!(records[0].golden.as_deref(), Some(divergence));
    assert_eq!(records[1].golden.as_deref(), Some("match"));
    let golden = r.calls_to("lab_golden_diff");
    assert_eq!(golden.len(), 2);
    assert_eq!(golden[0]["section"], "first-session");
    assert_eq!(golden[0]["run_dir"], "C:\\runs\\a");
    assert_eq!(golden[0]["specs_dir"], specs_arg.as_str());
    // --diff-golden asks the runs for a fingerprint, plan call included.
    let runs = r.calls_to("lab_uat_run");
    assert_eq!(runs[0]["fingerprint"], true);
    assert_eq!(runs[1]["fingerprint"], true);
    let batch = r.batch_json();
    let summary = &batch["summary"];
    assert_eq!(summary["ok"], true);
    assert_eq!(summary["golden"]["match"], 1);
    assert_eq!(summary["golden"]["diverged"], 1);
}

// ---- 5: --heal writes the proposal and exits 8 ----

#[test]
fn heal_writes_the_proposed_diff_into_the_batch_and_exits_8() {
    need_no_sgw!();
    let scratch = TempDir::new().unwrap();
    let proposal = "--- a/spec.toml\n+++ b/spec.toml\n@@ -1 +1 @@\n-bearing_deg = 322.5\n+bearing_deg = 330.0\n";
    let proposal_path = scratch.path().join("proposal.diff");
    fs::write(&proposal_path, proposal).unwrap();
    let failed = ToolReply::Json(json!({"run_dir": "C:\\runs\\f", "rows": [
        {"row": "FS-P1", "result": "PASS"},
        {"row": "FS-P2", "result": "FAIL", "reasons": ["step action 2 failed"],
         "heal": {"pose": "frost", "step": "hover"}}]}));
    let calibrated = ToolReply::Json(json!({"outcomes": [{
        "ok": true, "pose": "frost", "score": 8312, "confirmed": 3,
        "changed": {"bearing_deg": [322.5, 330.0]},
        "diff_path": proposal_path.display().to_string()}]}));
    let r = run_uat(
        vec![
            ("lab_uat_run", vec![plan(&["FS-P1", "FS-P2"]), failed]),
            ("lab_uat_calibrate", vec![calibrated]),
        ],
        &["--rows", "FS-P1,FS-P2", "--heal", "--leases", "1"],
    );
    assert_eq!(r.code(), Some(8), "{}", r.both());
    let copy = r.batch_dir().join("heal-FS-P2-frost.diff");
    assert_eq!(fs::read_to_string(&copy).unwrap(), proposal);
    let calibrate = r.calls_to("lab_uat_calibrate");
    assert_eq!(calibrate.len(), 1);
    assert_eq!(calibrate[0]["section"], "first-session");
    assert_eq!(calibrate[0]["row"], "FS-P2");
    assert_eq!(calibrate[0]["pose"], "frost");
    assert_eq!(calibrate[0]["instance"], "default");
    let batch = r.batch_json();
    let heal = &batch["summary"]["heal"];
    assert_eq!(heal.as_array().unwrap().len(), 1);
    assert_eq!(heal[0]["row"], "FS-P2");
    assert_eq!(heal[0]["pose"], "frost");
    assert!(heal[0]["diff"]
        .as_str()
        .unwrap()
        .ends_with("heal-FS-P2-frost.diff"));
    assert!(r.summary_md().contains("Heal proposals"));
    assert!(r
        .both()
        .contains("heal: recalibrating 1 pose(s) on default: FS-P2 frost"));
}

// ---- 6: the brake stops a lane ----

#[test]
fn the_brake_stops_a_lane_after_n_failures_and_exits_4() {
    need_no_sgw!();
    let r = run_uat(
        vec![(
            "lab_uat_run",
            vec![
                plan(&["FS-P1", "FS-P2"]),
                fail("FS-P2", "clause aim failed"),
                fail("FS-P2", "clause aim failed"),
            ],
        )],
        &[
            "--rows",
            "FS-P1,FS-P2",
            "--runs-per-lease",
            "4",
            "--max-consecutive-failures",
            "2",
        ],
    );
    assert_eq!(r.code(), Some(4), "{}", r.both());
    let records = r.records();
    assert_eq!(records.len(), 2, "runs 3 and 4 never started");
    assert!(!records[0].braked);
    assert!(records[1].braked);
    assert_eq!(
        r.calls_to("lab_uat_run").len(),
        3,
        "the plan call and two runs"
    );
    assert!(
        r.both().contains("stopped after 2 failed runs in a row"),
        "{}",
        r.both()
    );
    let batch = r.batch_json();
    assert_eq!(batch["summary"]["lanes"][0]["braked"], true);
    assert_eq!(batch["summary"]["lanes"][0]["passed"], 0);
}
