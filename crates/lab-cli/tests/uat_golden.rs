//! LX-17a: golden values, the golden merge and the per-run diff. Ports the
//! `-DiffGolden` checks of test-uat.ps1 (200 to 302). The golden file check and
//! the pre-check (304 to 352) are LX-15's.

mod uat_support;

use std::cell::RefCell;

use cimmeria_lab_cli::batch::{append_record, read_records, RunRecord};
use cimmeria_lab_cli::commands::uat::golden::{
    default_run_ref, golden_value, merge_golden, run_golden, DiffArgs,
};
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::{json, Value};
use tempfile::TempDir;
use uat_support::batch_fx::*;

fn g(i: u32, golden: &str) -> RunRecord {
    with_golden(run("default", i, true, good_rows()), golden)
}

// ---- golden_value ----

/// test-uat.ps1 [200] 'a matching diff stores match'
#[test]
fn a_matching_diff_stores_match() {
    assert_eq!(
        golden_value(Some(&json!({"ok": true, "match": true})), None),
        "match"
    );
}

/// [201] 'a divergence stores its first line'
#[test]
fn a_divergence_stores_its_first_line() {
    let reply = json!({"ok": true, "match": false, "first": DIV_A, "diverged": 2});
    assert_eq!(golden_value(Some(&reply), None), DIV_A);
}

/// [202-203] 'a cannot_diff refusal is an error, not a match'
#[test]
fn a_cannot_diff_refusal_is_an_error_not_a_match() {
    let reply =
        json!({"ok": false, "code": "cannot_diff", "error": r"run C:\r\x: no fingerprint.json"});
    assert_eq!(
        golden_value(Some(&reply), None),
        r"error: cannot_diff: run C:\r\x: no fingerprint.json"
    );
}

/// [204] 'a diff call that threw is an error'
#[test]
fn a_diff_call_that_threw_is_an_error() {
    assert_eq!(
        golden_value(None, Some("lab_golden_diff failed: connection refused")),
        "error: lab_golden_diff failed: connection refused"
    );
}

/// [205-206] 'a reply with no verdict is an error, not a match'; 'a divergence
/// without a line is an error'
#[test]
fn a_reply_with_no_verdict_is_an_error() {
    let none = "error: lab_golden_diff gave no verdict";
    assert_eq!(golden_value(Some(&json!({"ok": true})), None), none);
    assert_eq!(
        golden_value(Some(&json!({"ok": true, "match": false})), None),
        none
    );
}

/// [207-208] 'a stored line is cut to 300 characters with lease ids hidden'
#[test]
fn a_stored_line_is_cut_to_300_characters_with_lease_ids_hidden() {
    let first = format!("row FS-P3: lease-0123456789abcdef {}", "y".repeat(400));
    let v = golden_value(
        Some(&json!({"ok": true, "match": false, "first": first})),
        None,
    );
    assert_eq!(v.chars().count(), 300);
    assert!(v.ends_with("..."));
    assert!(v.starts_with("row FS-P3: lease-<redacted> yyyy"));
    assert!(!v.contains("0123456789abcdef"));
}

/// New: newlines in a reply become one space each run (the PowerShell
/// `-replace '[\r\n]+', ' '`).
#[test]
fn newlines_become_one_space() {
    let reply = json!({"ok": false, "error": "a\r\n\r\nb\nc"});
    assert_eq!(golden_value(Some(&reply), None), "error: a b c");
}

// ---- merge_golden ----

/// [219] 'golden_match_and_divergence_are_counted'
#[test]
fn golden_match_and_divergence_are_counted() {
    let runs = vec![
        g(1, "match"),
        g(2, DIV_A),
        g(3, "match"),
        g(4, DIV_A),
        g(5, DIV_B),
    ];
    let s = merge_golden(&runs, &default_run_ref).unwrap();
    assert_eq!((s.matched, s.diverged, s.errors), (2, 3, 0));
}

/// [220-221] 'divergence lines group with counts and run refs, most common
/// first'
#[test]
fn divergence_lines_group_with_counts_and_run_refs_most_common_first() {
    let runs = vec![
        g(1, "match"),
        g(2, DIV_A),
        g(3, "match"),
        g(4, DIV_A),
        g(5, DIV_B),
    ];
    let s = merge_golden(&runs, &default_run_ref).unwrap();
    assert_eq!(s.groups.len(), 2);
    assert_eq!((s.groups[0].line.as_str(), s.groups[0].count), (DIV_A, 2));
    assert_eq!(s.groups[0].runs, ["default#2", "default#4"]);
    assert_eq!((s.groups[1].line.as_str(), s.groups[1].count), (DIV_B, 1));
}

/// [222-223] 'a group keeps its full count but at most 5 run refs'
#[test]
fn a_group_keeps_its_full_count_but_at_most_5_run_refs() {
    let runs: Vec<RunRecord> = (1..=7).map(|i| g(i, DIV_B)).collect();
    let s = merge_golden(&runs, &default_run_ref).unwrap();
    assert_eq!((s.groups[0].runs.len(), s.groups[0].count), (5, 7));
}

/// [229-230] errors are counted apart and are not matches
#[test]
fn errors_are_counted_apart() {
    let s = merge_golden(&[g(1, "match"), g(2, CANNOT)], &default_run_ref).unwrap();
    assert_eq!((s.matched, s.diverged, s.errors), (1, 0, 1));
    assert_eq!(s.groups[0].line, CANNOT);
}

/// [321] 'a batch without Golden fields has no golden summary'
#[test]
fn runs_without_a_golden_value_have_no_golden_block() {
    let runs = vec![run("default", 1, true, good_rows())];
    assert_eq!(merge_golden(&runs, &default_run_ref), None);
    assert_eq!(merge_golden(&[], &default_run_ref), None);
}

/// New: `Sort-Object -Stable` (decision 4): equal counts keep first-seen order.
#[test]
fn equal_counts_keep_the_order_first_seen() {
    let s = merge_golden(&[g(1, DIV_B), g(2, DIV_A)], &default_run_ref).unwrap();
    assert_eq!(s.groups[0].line, DIV_B);
}

/// Review fix: `Merge-UatGolden` keys its groups in an ordered dictionary,
/// which ignores case: lines that differ only in case are one group, under the
/// casing seen first (checked in pwsh 7).
#[test]
fn lines_that_differ_only_in_case_are_one_group() {
    let s = merge_golden(
        &[g(1, "row A: x"), g(2, "ROW A: x"), g(3, "match")],
        &default_run_ref,
    )
    .unwrap();
    assert_eq!((s.matched, s.diverged, s.groups.len()), (1, 2, 1));
    assert_eq!(s.groups[0].line, "row A: x");
    assert_eq!(s.groups[0].count, 2);
    assert_eq!(s.groups[0].runs, ["default#1", "default#2"]);
}

/// New: `-ceq 'match'`: only the exact word is a match.
#[test]
fn only_the_exact_word_match_is_a_match() {
    let s = merge_golden(&[g(1, "Match")], &default_run_ref).unwrap();
    assert_eq!((s.matched, s.diverged), (0, 1));
}

/// New: the caller names the runs (lab-chaos passes its seed refs).
#[test]
fn the_run_ref_function_names_the_runs() {
    let s = merge_golden(&[g(1, DIV_A)], &|r: &RunRecord| format!("{}!", r.index)).unwrap();
    assert_eq!(s.groups[0].runs, ["1!"]);
}

/// [328-333] 'Golden fields read back from JSONL aggregate like live ones'
#[test]
fn golden_fields_read_back_from_jsonl_aggregate_like_live_ones() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("batch.jsonl");
    for r in [
        g(1, "match"),
        g(2, DIV_A),
        g(3, "match"),
        g(4, DIV_A),
        g(5, DIV_B),
    ] {
        append_record(&path, &r).unwrap();
    }
    let back = read_records(&path).unwrap();
    let s = merge_golden(&back, &default_run_ref).unwrap();
    assert_eq!((s.matched, s.diverged), (2, 3));
}

// ---- run_golden ----

fn diff_args() -> DiffArgs {
    DiffArgs {
        section: "first-session".into(),
        specs_dir: Some(r"C:\specs".into()),
    }
}

fn pass_rec() -> RunRecord {
    RunRecord {
        instance: "default".into(),
        index: 1,
        ok: true,
        run_dir: Some(r"C:\runs\r1".into()),
        ..Default::default()
    }
}

fn ok_match(_: Value) -> Result<Value, McpError> {
    Ok(json!({"ok": true, "match": true}))
}

/// [280] 'run_golden: a passing run that matches is match'
#[test]
fn run_golden_a_passing_run_that_matches_is_match() {
    let d = diff_args();
    assert_eq!(
        run_golden(&pass_rec(), Some(&d), &mut ok_match).as_deref(),
        Some("match")
    );
}

/// [281] 'run_golden: the call gets the run folder; the shared arguments are
/// not changed'
#[test]
fn run_golden_the_call_gets_the_run_folder() {
    let d = diff_args();
    let seen = RefCell::new(Vec::<Value>::new());
    let mut call = |a: Value| -> Result<Value, McpError> {
        seen.borrow_mut().push(a);
        Ok(json!({"ok": true, "match": true}))
    };
    run_golden(&pass_rec(), Some(&d), &mut call);
    assert_eq!(
        seen.borrow()[0],
        json!({"section": "first-session", "specs_dir": r"C:\specs", "run_dir": r"C:\runs\r1"})
    );
    assert_eq!(d, diff_args());
}

/// New: no local spec folder, no `specs_dir` key in the call.
#[test]
fn run_golden_without_a_spec_folder_sends_no_specs_dir() {
    let d = DiffArgs::new("first-session", None);
    let seen = RefCell::new(Vec::<Value>::new());
    let mut call = |a: Value| -> Result<Value, McpError> {
        seen.borrow_mut().push(a);
        Ok(json!({"ok": true, "match": true}))
    };
    run_golden(&pass_rec(), Some(&d), &mut call);
    assert_eq!(
        seen.borrow()[0],
        json!({"section": "first-session", "run_dir": r"C:\runs\r1"})
    );
}

/// [282-283] 'run_golden: a divergence is its first line'
#[test]
fn run_golden_a_divergence_is_its_first_line() {
    let d = diff_args();
    let mut call = |_: Value| -> Result<Value, McpError> {
        Ok(json!({"ok": true, "match": false, "first": DIV_A}))
    };
    assert_eq!(
        run_golden(&pass_rec(), Some(&d), &mut call).as_deref(),
        Some(DIV_A)
    );
}

/// [284-285] 'run_golden: a call that throws is an error' (first line only)
#[test]
fn run_golden_a_call_that_throws_is_an_error() {
    let d = diff_args();
    let mut call = |_: Value| -> Result<Value, McpError> {
        Err(McpError::Tool("connection refused\nmore".into()))
    };
    assert_eq!(
        run_golden(&pass_rec(), Some(&d), &mut call).as_deref(),
        Some("error: lab_golden_diff failed: connection refused")
    );
}

/// [286-289] 'run_golden: a passing run with no folder is an error, with no
/// call'
#[test]
fn run_golden_a_passing_run_with_no_folder_is_an_error_with_no_call() {
    let d = diff_args();
    let calls = RefCell::new(0u32);
    let mut call = |_: Value| -> Result<Value, McpError> {
        *calls.borrow_mut() += 1;
        Ok(json!({"ok": true, "match": true}))
    };
    let mut rec = pass_rec();
    rec.run_dir = None;
    assert_eq!(
        run_golden(&rec, Some(&d), &mut call).as_deref(),
        Some("error: the run named no run folder")
    );
    assert_eq!(*calls.borrow(), 0);
}

/// [290-292] 'run_golden: a failed run is not compared'; 'without -DiffGolden
/// there is no golden and no call'
#[test]
fn run_golden_a_failed_run_or_no_diff_golden_makes_no_call() {
    let d = diff_args();
    let calls = RefCell::new(0u32);
    let mut call = |_: Value| -> Result<Value, McpError> {
        *calls.borrow_mut() += 1;
        Ok(json!({"ok": true, "match": true}))
    };
    let mut failed = pass_rec();
    failed.ok = false;
    assert_eq!(run_golden(&failed, Some(&d), &mut call), None);
    assert_eq!(run_golden(&pass_rec(), None, &mut call), None);
    assert_eq!(*calls.borrow(), 0);
}

/// [293-302] 'run_golden: works in a parallel lane runspace': two lane
/// threads at once each get their own folder back (no shared state).
#[test]
fn run_golden_works_from_two_lane_threads_at_once() {
    let d = diff_args();
    let got: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = ["p1", "p2"]
            .into_iter()
            .map(|lane| {
                let d = &d;
                scope.spawn(move || {
                    let mut rec = pass_rec();
                    rec.run_dir = Some(format!(r"C:\runs\{lane}"));
                    let mut call = |a: Value| -> Result<Value, McpError> {
                        Ok(json!({"ok": true, "match": false,
                            "first": format!("row X: http://{lane} {}", a["run_dir"].as_str().unwrap())}))
                    };
                    run_golden(&rec, Some(d), &mut call).unwrap()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(
        got,
        [
            r"row X: http://p1 C:\runs\p1",
            r"row X: http://p2 C:\runs\p2"
        ]
    );
}
