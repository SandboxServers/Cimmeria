//! LX-12a: heal_file_name and invoke_heal with stubbed MCP and lease wait.
//! Ports the heal checks of the removed PowerShell test file that exercise
//! Get-UatHealFileName and Invoke-UatHeal (LX-17 does not repeat them).

use std::cell::RefCell;
use std::fs;

use cimmeria_lab_cli::commands::calibrate::heal::{
    heal_file_name, invoke_heal, HealHint, HealJob, HealRecord,
};
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::{json, Value};
use tempfile::TempDir;

const DIFF_TEXT: &str = "--- a/docs/guides/uat-specs/first-session.toml\n+++ b/docs/guides/uat-specs/first-session.toml\n@@ -1 +1 @@\n-a\n+b\n";

fn hint(row: &str, pose: &str) -> HealHint {
    HealHint {
        row: row.into(),
        pose: pose.into(),
    }
}

fn reply(src: &str) -> Value {
    json!({"run_dir": "x", "outcomes": [
        {"ok": true, "pose": "frost", "score": 8312, "confirmed": 3, "changed": {"bearing_deg": [322.5, 330.0]}, "diff_path": src},
        {"ok": false, "pose": "supply", "confirmed": 0, "reason": "no candidate hovered the target"}]})
}

// the removed PowerShell test file: 'heal file names are safe'
#[test]
fn heal_file_names_are_safe() {
    assert_eq!(heal_file_name("FS-P3", "frost"), "heal-FS-P3-frost.diff");
    assert_eq!(heal_file_name("a/b", "c d"), "heal-a_b-c_d.diff");
}

// the removed PowerShell test file: the call names the section, row, pose, instance and spec folder;
// the proposal is saved in the batch folder; the line names the batch copy; a
// pose with no proposal writes no file.
#[test]
fn a_proposal_is_copied_into_the_batch_folder() {
    let tmp = TempDir::new().unwrap();
    let batch = tmp.path().join("batch");
    fs::create_dir(&batch).unwrap();
    let src = tmp.path().join("first-session.diff");
    fs::write(&src, DIFF_TEXT).unwrap();
    let r = reply(&src.to_string_lossy());
    let calls = RefCell::new(Vec::<Value>::new());
    let mut call = |a: Value| -> Result<Value, McpError> {
        calls.borrow_mut().push(a);
        Ok(r.clone())
    };
    let job = HealJob {
        batch_dir: &batch,
        section: "first-session",
        specs_dir: "C:\\specs",
        instance: "p2",
    };
    let recs = invoke_heal(
        &[hint("FS-P3", "frost"), hint("FS-P5", "supply")],
        &job,
        &mut call,
        None,
        None,
    );
    assert_eq!(calls.borrow().len(), 2);
    assert_eq!(
        calls.borrow()[0],
        json!({"section": "first-session", "row": "FS-P3", "pose": "frost", "instance": "p2", "specs_dir": "C:\\specs"})
    );
    let copy = batch.join("heal-FS-P3-frost.diff");
    assert_eq!(recs[0].diff.as_deref(), Some(copy.as_path()));
    assert_eq!(fs::read_to_string(&copy).unwrap(), DIFF_TEXT);
    assert_eq!(recs[0].text.as_deref(), Some(DIFF_TEXT));
    assert_eq!(
        recs[0].line,
        format!(
            "heal FS-P3 frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff {}",
            copy.display()
        )
    );
    assert_eq!(recs[1].diff, None);
    assert_eq!(
        recs[1].line,
        "heal FS-P5 supply: no proposal: no candidate hovered the target"
    );
    assert!(!batch.join("heal-FS-P5-supply.diff").exists());
}

// the removed PowerShell test file: 'a failed call proposes nothing' (the tool's text is bare in Rust)
#[test]
fn a_failed_call_proposes_nothing() {
    let tmp = TempDir::new().unwrap();
    let mut call = |_: Value| -> Result<Value, McpError> {
        Err(McpError::Tool("lease-0123456789abcdef held".into()))
    };
    let job = HealJob {
        batch_dir: tmp.path(),
        section: "first-session",
        specs_dir: "",
        instance: "default",
    };
    let recs = invoke_heal(&[hint("FS-P3", "frost")], &job, &mut call, None, None);
    assert_eq!(recs[0].diff, None);
    assert_eq!(
        recs[0].line,
        "heal FS-P3 frost: lab_uat_calibrate failed: lease-<redacted> held"
    );
}

// the removed PowerShell test file: a reply with no outcome for the pose; no instance or specs_dir keys when empty
#[test]
fn a_reply_with_no_outcome_for_the_pose() {
    let tmp = TempDir::new().unwrap();
    let seen = RefCell::new(Vec::<Value>::new());
    let mut call = |a: Value| -> Result<Value, McpError> {
        seen.borrow_mut().push(a);
        Ok(json!({"run_dir": "x", "outcomes": []}))
    };
    let job = HealJob {
        batch_dir: tmp.path(),
        section: "first-session",
        specs_dir: "",
        instance: "",
    };
    let recs = invoke_heal(&[hint("FS-P3", "frost")], &job, &mut call, None, None);
    assert_eq!(
        recs[0].line,
        "heal FS-P3 frost: lab_uat_calibrate returned no outcome for the pose"
    );
    assert_eq!(
        seen.borrow()[0],
        json!({"section": "first-session", "row": "FS-P3", "pose": "frost"})
    );
}

fn failing_call(count: &RefCell<usize>) -> impl FnMut(Value) -> Result<Value, McpError> + '_ {
    move |_| {
        *count.borrow_mut() += 1;
        Err(McpError::Unreachable("boom".into()))
    }
}

// the removed PowerShell test file: 'a lease that stays held stops healing'
#[test]
fn a_lease_that_stays_held_stops_healing() {
    let tmp = TempDir::new().unwrap();
    let count = RefCell::new(0usize);
    let mut call = failing_call(&count);
    let mut wait = |_: &str| false;
    let job = HealJob {
        batch_dir: tmp.path(),
        section: "first-session",
        specs_dir: "",
        instance: "p2",
    };
    let recs = invoke_heal(
        &[hint("FS-P3", "frost"), hint("FS-P5", "supply")],
        &job,
        &mut call,
        None,
        Some(&mut wait),
    );
    assert_eq!(*count.borrow(), 1);
    assert_eq!(recs.len(), 2);
    assert_eq!(recs[1].row, None);
    assert_eq!(
        recs[1].line,
        "heal stopped: the lease on p2 did not clear after the failed call; not recalibrated: FS-P5 supply"
    );
}

// the removed PowerShell test file: the wait runs after a failed call and not after the last hint
#[test]
fn the_wait_runs_after_a_failed_call_but_not_after_the_last() {
    let tmp = TempDir::new().unwrap();
    let count = RefCell::new(0usize);
    let mut call = failing_call(&count);
    let waits = RefCell::new(Vec::<String>::new());
    let mut wait = |i: &str| {
        waits.borrow_mut().push(i.to_string());
        true
    };
    let job = HealJob {
        batch_dir: tmp.path(),
        section: "first-session",
        specs_dir: "",
        instance: "p2",
    };
    let recs = invoke_heal(
        &[hint("FS-P3", "frost"), hint("FS-P5", "supply")],
        &job,
        &mut call,
        None,
        Some(&mut wait),
    );
    assert_eq!(*count.borrow(), 2);
    assert_eq!(recs.len(), 2);
    assert_eq!(*waits.borrow(), vec!["p2".to_string()]);
}

// New: on_record sees every record as it is made, the stop record included.
#[test]
fn on_record_sees_every_record_in_order() {
    let tmp = TempDir::new().unwrap();
    let count = RefCell::new(0usize);
    let mut call = failing_call(&count);
    let mut wait = |_: &str| false;
    let lines = RefCell::new(Vec::<String>::new());
    let mut on = |r: &HealRecord| lines.borrow_mut().push(r.line.clone());
    let job = HealJob {
        batch_dir: tmp.path(),
        section: "s",
        specs_dir: "",
        instance: "p2",
    };
    let recs = invoke_heal(
        &[hint("R1", "a"), hint("R2", "b")],
        &job,
        &mut call,
        Some(&mut on),
        Some(&mut wait),
    );
    assert_eq!(lines.borrow().len(), 2);
    assert_eq!(
        *lines.borrow(),
        recs.iter().map(|r| r.line.clone()).collect::<Vec<_>>()
    );
}
