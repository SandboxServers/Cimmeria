//! LX-17b: heal hints and the heal pass (stubbed calibrate call, real batch
//! files). Ports the heal checks of the removed PowerShell test file that LX-12 did not (hints,
//! the pass, Ctrl+C mid-heal, the 240-character budget is in uat_json.rs).

mod uat_support;

use std::cell::RefCell;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;

use chrono::Local;
use cimmeria_lab_cli::batch::{append_record, read_records, RunRecord};
use cimmeria_lab_cli::commands::uat::heal::{heal_hints, heal_pass, HealIo};
use cimmeria_lab_cli::commands::uat::summary::{
    exit_code, write_batch_summary, BatchMeta, Summary,
};
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::{json, Value};
use tempfile::TempDir;
use uat_support::batch_fx::*;

const DIFF_TEXT: &str = "--- a/docs/guides/uat-specs/first-session.toml\n+++ b/docs/guides/uat-specs/first-session.toml\n@@ -1 +1 @@\n-pose frost = { bearing_deg = 322.5 }\n+pose frost = { bearing_deg = 330.0 }\n";

fn heal_runs() -> Vec<RunRecord> {
    let mut bridge = run("default", 4, false, vec![]);
    bridge.error = Some("lab_uat_run failed: bridge down".into());
    vec![
        run(
            "default",
            1,
            false,
            vec![
                heal_row("FS-P1", "PASS", None),
                heal_row("FS-P3", "FAIL", Some("frost")),
                heal_row("FS-P4", "FAIL", None),
            ],
        ),
        run(
            "default",
            2,
            false,
            vec![
                heal_row("FS-P3", "FAIL", Some("frost")),
                heal_row("FS-P5", "BLOCKED", Some("supply")),
            ],
        ),
        // A PASS row never carries a hint; one that somehow does is ignored.
        run(
            "default",
            3,
            true,
            vec![heal_row("FS-P3", "PASS", Some("guard"))],
        ),
        bridge,
    ]
}

fn pairs(runs: &[RunRecord]) -> Vec<(String, String)> {
    heal_hints(runs)
        .into_iter()
        .map(|h| (h.row, h.pose))
        .collect()
}

/// the removed PowerShell test file [372] 'heal hints: two runs' identical hint once, rows without
/// one and PASS rows ignored'
#[test]
fn heal_hints_list_each_pair_once_in_first_seen_order() {
    assert_eq!(
        pairs(&heal_runs()),
        [
            ("FS-P3".to_string(), "frost".to_string()),
            ("FS-P5".to_string(), "supply".to_string())
        ]
    );
}

/// [373] 'a failed batch without hints has none'
#[test]
fn a_failed_batch_without_hints_has_none() {
    assert!(pairs(&[run(
        "default",
        1,
        false,
        vec![heal_row("FS-P3", "FAIL", None)]
    )])
    .is_empty());
}

/// [374-379] 'heal hints read back from batch.jsonl like live ones'
#[test]
fn heal_hints_read_back_from_batch_jsonl_like_live_ones() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("batch.jsonl");
    for r in heal_runs() {
        append_record(&path, &r).unwrap();
    }
    let back = read_records(&path).unwrap();
    let got = pairs(&back);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].1, "frost");
}

/// New: a PASS in any letter case is not a miss.
#[test]
fn a_pass_row_in_any_case_is_ignored() {
    assert!(pairs(&[run(
        "default",
        1,
        false,
        vec![heal_row("FS-P3", "pass", Some("frost"))]
    )])
    .is_empty());
}

// ---- the pass ----

/// Every file under a folder with its bytes, sorted: a write, a new file or a
/// deleted one changes it.
fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push((path.display().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

struct Fixture {
    _tmp: TempDir,
    batch: std::path::PathBuf,
    run_dir: std::path::PathBuf,
    specs: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let tmp = TempDir::new().unwrap();
    let batch = tmp.path().join("batch");
    let run_dir = tmp.path().join("run");
    let specs = tmp.path().join("specs");
    for d in [&batch, &run_dir, &specs, &specs.join("golden")] {
        fs::create_dir_all(d).unwrap();
    }
    fs::write(
        specs.join("first-session.toml"),
        r#"pose frost = { tag = "frost", radius_m = 3.75, bearing_deg = 322.5 }"#,
    )
    .unwrap();
    fs::write(specs.join("abilities.toml"), "[section]").unwrap();
    fs::write(
        specs.join("golden").join("first-session.json"),
        r#"{"schema":1}"#,
    )
    .unwrap();
    Fixture {
        _tmp: tmp,
        batch,
        run_dir,
        specs,
    }
}

/// The owned half of a `BatchMeta` (which borrows); `.get()` is the meta.
struct Meta {
    dir: std::path::PathBuf,
    rows: Vec<String>,
}

impl Meta {
    fn get(&self) -> BatchMeta<'_> {
        BatchMeta {
            dir: &self.dir,
            title: "lab uat first-session",
            section: "first-session",
            row_ids: &self.rows,
            lane_count: 1,
            runs_per_lease: 3,
            started: Local::now(),
        }
    }
}

fn info(dir: &Path) -> Meta {
    Meta {
        dir: dir.to_path_buf(),
        rows: ["FS-P1", "FS-P3", "FS-P4", "FS-P5"]
            .map(String::from)
            .to_vec(),
    }
}

fn base_summary() -> Summary {
    Summary {
        ok: false,
        runs: 4,
        passed: 1,
        failed: 3,
        lanes: vec![ls(1, "default", 4, 1, false)],
        ..Default::default()
    }
}

/// The stub of lab_uat_calibrate: `frost` proposes a diff file in `run_dir`,
/// any other pose finds none.
fn stub<'a>(
    run_dir: &'a Path,
    calls: &'a RefCell<Vec<Value>>,
) -> impl FnMut(Value) -> Result<Value, McpError> + 'a {
    move |a: Value| {
        calls.borrow_mut().push(a.clone());
        let dir = run_dir.to_string_lossy().to_string();
        if a["pose"] == "frost" {
            let d = run_dir.join("first-session.diff");
            fs::write(&d, DIFF_TEXT).unwrap();
            Ok(
                json!({"run_dir": dir, "outcomes": [{"ok": true, "section": a["section"], "row": a["row"],
                "pose": "frost", "score": 8312, "confirmed": 3, "changed": {"bearing_deg": [322.5, 330.0]},
                "diff_path": d.to_string_lossy(), "probes": 41}]}),
            )
        } else {
            Ok(
                json!({"run_dir": dir, "outcomes": [{"ok": false, "section": a["section"], "row": a["row"],
                "pose": a["pose"], "confirmed": 0, "probes": 108, "reason": "no candidate hovered the target"}]}),
            )
        }
    }
}

/// [462-482] the pass: one call per hint on the batch instance, the lines it
/// says, summary.md and batch.json rewritten with the proposal, exit 8, and
/// nothing written under the spec folder.
#[test]
fn the_pass_recalibrates_each_hint_and_rewrites_the_summary() {
    let fx = fixture();
    let runs = heal_runs();
    let mut summary = base_summary();
    let batch = info(&fx.batch);
    write_batch_summary(&summary, &batch.get(), Local::now()).unwrap();
    assert!(!fs::read_to_string(fx.batch.join("summary.md"))
        .unwrap()
        .contains("Heal proposals"));
    assert_eq!(exit_code(&summary), 1); // [471] before the heal

    let specs_before = tree(&fx.specs);
    let calls = RefCell::new(Vec::<Value>::new());
    let said = RefCell::new(Vec::<String>::new());
    let waits = RefCell::new(Vec::<String>::new());
    let mut call = stub(&fx.run_dir, &calls);
    let mut say = |m: &str| said.borrow_mut().push(m.to_string());
    let mut wait = |i: &str| {
        waits.borrow_mut().push(i.to_string());
        true
    };
    let specs_text = fx.specs.display().to_string();
    let md = heal_pass(
        &runs,
        &mut summary,
        &batch.get(),
        &specs_text,
        "default",
        HealIo {
            call: &mut call,
            say: &mut say,
            wait: Some(&mut wait),
            print_diff: None,
        },
    )
    .expect("a proposal was written");

    // [473-474] one call per hint, on the batch instance, with the spec folder
    let calls = calls.borrow();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        (calls[0]["pose"].as_str(), calls[1]["pose"].as_str()),
        (Some("frost"), Some("supply"))
    );
    assert_eq!(calls[0]["instance"], "default");
    assert_eq!(calls[0]["specs_dir"], specs_text.as_str());
    // [475] what it says
    let copy = fx.batch.join("heal-FS-P3-frost.diff");
    let said = said.borrow();
    assert_eq!(said.len(), 3);
    assert_eq!(
        said[0],
        "heal: recalibrating 2 pose(s) on default: FS-P3 frost, FS-P5 supply"
    );
    assert_eq!(
        said[1],
        format!(
            "heal FS-P3 frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff {}",
            copy.display()
        )
    );
    assert_eq!(
        said[2],
        "heal FS-P5 supply: no proposal: no candidate hovered the target"
    );
    assert!(waits.borrow().is_empty(), "calls that return need no wait");
    // [476-480] summary.md, batch.json and the returned Markdown
    let line = format!("- FS-P3 frost: {}", copy.display());
    assert!(copy.exists());
    assert!(fs::read_to_string(fx.batch.join("summary.md"))
        .unwrap()
        .contains(&line));
    assert!(md.contains(&line));
    let bj: Value =
        serde_json::from_str(&fs::read_to_string(fx.batch.join("batch.json")).unwrap()).unwrap();
    assert_eq!(bj["summary"]["heal"].as_array().unwrap().len(), 1);
    assert_eq!(bj["summary"]["heal"][0]["row"], "FS-P3");
    assert_eq!(bj["summary"]["heal"][0]["pose"], "frost");
    assert_eq!(
        bj["summary"]["heal"][0]["diff"],
        copy.display().to_string().as_str()
    );
    assert_eq!(bj["section"], "first-session");
    assert_eq!(bj["runs_per_lease"], 3);
    // [481] the healed batch exits 8; [482] the spec folder is untouched
    assert_eq!(exit_code(&summary), 8);
    assert_eq!(tree(&fx.specs), specs_before);
}

/// [432-433] 'summary.md lists the heal proposal' ("nothing was applied")
#[test]
fn summary_md_says_nothing_was_applied() {
    let fx = fixture();
    let mut summary = base_summary();
    let calls = RefCell::new(Vec::<Value>::new());
    let mut call = stub(&fx.run_dir, &calls);
    let mut say = |_: &str| {};
    let md = heal_pass(
        &heal_runs(),
        &mut summary,
        &info(&fx.batch).get(),
        "",
        "default",
        HealIo {
            call: &mut call,
            say: &mut say,
            wait: None,
            print_diff: None,
        },
    )
    .unwrap();
    assert!(md.contains("Heal proposals (spec diffs to review; nothing was applied):"));
}

/// New: the proposal's text goes to `print_diff` when one is given (host mode).
#[test]
fn a_proposal_is_printed_when_a_printer_is_given() {
    let fx = fixture();
    let mut summary = base_summary();
    let calls = RefCell::new(Vec::<Value>::new());
    let printed = RefCell::new(Vec::<String>::new());
    let mut call = stub(&fx.run_dir, &calls);
    let mut say = |_: &str| {};
    let mut print = |t: &str| printed.borrow_mut().push(t.to_string());
    heal_pass(
        &heal_runs(),
        &mut summary,
        &info(&fx.batch).get(),
        "",
        "default",
        HealIo {
            call: &mut call,
            say: &mut say,
            wait: None,
            print_diff: Some(&mut print),
        },
    );
    assert_eq!(*printed.borrow(), [DIFF_TEXT.to_string()]);
}

/// [484-493] 'no hints: no call, the summary files untouched, the batch code
/// kept'; and [557-562] '-Heal with no hints calls nothing and exits as before'
#[test]
fn no_hints_no_call_and_the_files_are_untouched() {
    let fx = fixture();
    let runs = vec![run("default", 1, false, frost_rows())];
    let mut summary = Summary {
        ok: false,
        runs: 1,
        failed: 1,
        lanes: vec![ls(1, "default", 1, 0, false)],
        ..Default::default()
    };
    let batch = info(&fx.batch);
    write_batch_summary(&summary, &batch.get(), Local::now()).unwrap();
    let before = tree(&fx.batch);
    let calls = RefCell::new(Vec::<Value>::new());
    let said = RefCell::new(Vec::<String>::new());
    let mut call = stub(&fx.run_dir, &calls);
    let mut say = |m: &str| said.borrow_mut().push(m.to_string());
    let md = heal_pass(
        &runs,
        &mut summary,
        &batch.get(),
        "",
        "default",
        HealIo {
            call: &mut call,
            say: &mut say,
            wait: None,
            print_diff: None,
        },
    );
    assert_eq!(md, None);
    assert!(calls.borrow().is_empty());
    assert!(said.borrow()[0].starts_with("heal: no failed row carries a heal hint"));
    assert_eq!(tree(&fx.batch), before);
    assert_eq!(exit_code(&summary), 1);
    assert!(!fs::read_to_string(fx.batch.join("summary.md"))
        .unwrap()
        .contains("Heal"));
}

/// New: no proposal (the only pose found none) leaves the batch files and the
/// exit code alone.
#[test]
fn a_pass_with_no_proposal_leaves_the_files_alone() {
    let fx = fixture();
    let runs = vec![run(
        "default",
        1,
        false,
        vec![heal_row("FS-P5", "FAIL", Some("supply"))],
    )];
    let mut summary = base_summary();
    let batch = info(&fx.batch);
    write_batch_summary(&summary, &batch.get(), Local::now()).unwrap();
    let before = tree(&fx.batch);
    let calls = RefCell::new(Vec::<Value>::new());
    let mut call = stub(&fx.run_dir, &calls);
    let mut say = |_: &str| {};
    let md = heal_pass(
        &runs,
        &mut summary,
        &batch.get(),
        "",
        "default",
        HealIo {
            call: &mut call,
            say: &mut say,
            wait: None,
            print_diff: None,
        },
    );
    assert_eq!(md, None);
    assert_eq!(tree(&fx.batch), before);
    assert!(summary.heal.is_empty());
}

/// [495-530] 'an interrupted heal still records the diff already written in
/// batch.json and summary.md' (decision 6: the stub panics on the second hint,
/// as Ctrl+C stopped the pipeline)
#[test]
fn an_interrupted_heal_still_records_the_diff_already_written() {
    let fx = fixture();
    let runs = vec![run(
        "default",
        1,
        false,
        vec![
            heal_row("FS-P3", "FAIL", Some("frost")),
            heal_row("FS-P4", "FAIL", Some("guard")),
        ],
    )];
    let mut summary = base_summary();
    let batch = info(&fx.batch);
    write_batch_summary(&summary, &batch.get(), Local::now()).unwrap();
    let count = RefCell::new(0u32);
    let run_dir = fx.run_dir.clone();
    let mut call = |a: Value| -> Result<Value, McpError> {
        *count.borrow_mut() += 1;
        if *count.borrow() > 1 {
            panic!("the pipeline was stopped");
        }
        let d = run_dir.join("first-session.diff");
        fs::write(&d, "--- a/x\n+++ b/x\n").unwrap();
        Ok(
            json!({"outcomes": [{"ok": true, "pose": a["pose"], "score": 9000, "confirmed": 3,
            "diff_path": d.to_string_lossy()}]}),
        )
    };
    let mut say = |_: &str| {};
    let result = catch_unwind(AssertUnwindSafe(|| {
        heal_pass(
            &runs,
            &mut summary,
            &batch.get(),
            "",
            "default",
            HealIo {
                call: &mut call,
                say: &mut say,
                wait: None,
                print_diff: None,
            },
        )
    }));
    assert!(result.is_err(), "the stub stopped the pass mid-heal");
    let copy = fx.batch.join("heal-FS-P3-frost.diff");
    let bj: Value =
        serde_json::from_str(&fs::read_to_string(fx.batch.join("batch.json")).unwrap()).unwrap();
    assert_eq!(bj["summary"]["heal"].as_array().unwrap().len(), 1);
    assert_eq!(bj["summary"]["heal"][0]["pose"], "frost");
    assert_eq!(
        bj["summary"]["heal"][0]["diff"],
        copy.display().to_string().as_str()
    );
    assert!(fs::read_to_string(fx.batch.join("summary.md"))
        .unwrap()
        .contains(&format!("- FS-P3 frost: {}", copy.display())));
    assert!(!fx.batch.join("heal-FS-P4-guard.diff").exists());
}
