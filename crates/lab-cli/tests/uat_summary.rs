mod uat_support;

use chrono::{Duration, Local, TimeZone};
use cimmeria_lab_cli::batch::{append_record, read_records, RunRecord};
use cimmeria_lab_cli::commands::uat::summary::{
    exit_code, format_markdown, merge, write_batch_summary, BatchMeta, RowStat,
};
use uat_support::{frost, good, ids, lane, lanes2, rec, row};

/// The PowerShell test's `$runs`: default#1 passes, default#2 and p2#1 fail on
/// FS-P2 (known #1341), p2#2 errors.
fn four() -> Vec<RunRecord> {
    vec![
        rec("default", 1, true, good(), None),
        rec("default", 2, false, frost(), None),
        rec("p2", 1, false, frost(), None),
        rec(
            "p2",
            2,
            false,
            vec![],
            Some("lab_uat_run failed: bridge down"),
        ),
    ]
}

#[test]
fn totals() {
    // test-uat.ps1: 'totals'
    let s = merge(&four(), &lanes2(), &["p2".to_string()]);
    assert_eq!((s.runs, s.passed, s.failed), (4, 1, 3));
    assert!(!s.ok);
}

#[test]
fn known_and_new_failures_are_counted_apart() {
    // test-uat.ps1: 'known (#1341) and new failures are counted apart'
    let s = merge(&four(), &lanes2(), &[]);
    assert_eq!((s.failed_known, s.failed_new), (2, 1));
}

#[test]
fn per_row_pass_counts_an_errored_run_has_no_rows() {
    // test-uat.ps1: 'per-row pass counts (an errored run has no rows)'
    let s = merge(&four(), &lanes2(), &[]);
    assert_eq!(s.rows.get("FS-P2"), Some(&RowStat { pass: 1, total: 3 }));
    assert_eq!(s.rows.get("FS-P1"), Some(&RowStat { pass: 3, total: 3 }));
}

#[test]
fn identical_failures_group_with_their_runs() {
    // test-uat.ps1: 'identical failures group with their runs'
    let s = merge(&four(), &lanes2(), &[]);
    assert_eq!(s.failures.len(), 2);
    let g = &s.failures[0];
    assert_eq!(g.signature, "FS-P2 FAIL: clause movie-over failed");
    assert_eq!((g.count, g.issue.as_deref()), (2, Some("#1341")));
    assert_eq!(g.runs, ["default#2", "p2#1"]);
    let e = &s.failures[1];
    assert_eq!(e.signature, "error: lab_uat_run failed: bridge down");
    assert_eq!((e.count, e.issue.as_deref()), (1, None));
    assert_eq!(e.runs, ["p2#2"]);
}

#[test]
fn lane_totals_and_a_braked_lane_is_marked() {
    // test-uat.ps1: 'a braked lane is marked'
    let s = merge(&four(), &lanes2(), &["p2".to_string()]);
    assert_eq!(
        (s.lanes[0].runs, s.lanes[0].passed, s.lanes[0].braked),
        (2, 1, false)
    );
    assert_eq!(
        (s.lanes[1].runs, s.lanes[1].passed, s.lanes[1].braked),
        (2, 0, true)
    );
}

#[test]
fn a_braked_lane_exits_4() {
    // test-uat.ps1: 'a braked lane exits 4'
    assert_eq!(
        exit_code(&merge(&four(), &lanes2(), &["p2".to_string()])),
        4
    );
}

#[test]
fn a_failed_run_exits_1() {
    // test-uat.ps1: 'a failed run exits 1'
    let s = merge(
        &[rec("default", 1, false, frost(), None)],
        &[lane(1, "default")],
        &[],
    );
    assert_eq!(exit_code(&s), 1);
}

#[test]
fn all_passed_exits_0() {
    // test-uat.ps1: 'all passed exits 0'
    let s = merge(
        &[rec("default", 1, true, good(), None)],
        &[lane(1, "default")],
        &[],
    );
    assert!(s.ok);
    assert_eq!(exit_code(&s), 0);
}

#[test]
fn no_finished_runs_is_not_a_pass() {
    // test-uat.ps1: 'no finished runs is not a pass'
    let s = merge(&[], &[lane(1, "default")], &[]);
    assert_eq!(s.runs, 0);
    assert!(!s.ok);
    assert_eq!(exit_code(&s), 1);
}

#[test]
fn started_and_ended_do_not_change_the_aggregate() {
    // test-uat.ps1: 'Started and Ended do not change the aggregate'
    let plain = rec("default", 1, false, frost(), None);
    let mut timed = plain.clone();
    timed.started = Some("2026-10-11T01:39:06.0000000Z".to_string());
    timed.ended = Some("2026-10-11T01:41:00.0000000Z".to_string());
    let a = merge(&[plain], &[lane(1, "default")], &[]);
    let b = merge(&[timed], &[lane(1, "default")], &[]);
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}

#[test]
fn records_read_back_from_jsonl_aggregate_like_live_ones() {
    // test-uat.ps1: 'records read back from JSONL aggregate like live ones' and
    // 'a missing JSONL file reads as no runs'
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("batch.jsonl");
    assert!(read_records(&path).unwrap().is_empty());
    append_record(&path, &rec("default", 1, false, frost(), None)).unwrap();
    let mut death = rec("default", 0, false, vec![], Some("lane: boom"));
    death.braked = true;
    append_record(&path, &death).unwrap();
    let back = read_records(&path).unwrap();
    assert_eq!(back.len(), 2);
    let s = merge(&back, &[lane(1, "default")], &["default".to_string()]);
    assert_eq!((s.runs, s.failed_known, s.failed_new), (2, 1, 1));
}

#[test]
fn rows_keep_their_first_seen_order_in_the_json() {
    // A serde_json::Map would sort ZZ after AA.
    let s = merge(
        &[rec(
            "default",
            1,
            true,
            vec![row("ZZ", "PASS", ""), row("AA", "PASS", "")],
            None,
        )],
        &[lane(1, "default")],
        &[],
    );
    let json = serde_json::to_string(&s).unwrap();
    assert!(
        json.find(r#""ZZ""#).unwrap() < json.find(r#""AA""#).unwrap(),
        "{json}"
    );
    assert!(json.starts_with(
        r#"{"ok":true,"runs":1,"passed":1,"failed":0,"failed_known":0,"failed_new":0,"rows":{"ZZ":{"pass":1,"total":1},"AA":{"pass":1,"total":1}},"lanes":[{"lane":1,"instance":"default","runs":1,"passed":1,"braked":false}],"failures":[]}"#
    ), "{json}");
}

fn crlf(lines: &[&str]) -> String {
    lines.iter().map(|l| format!("{l}\r\n")).collect()
}

#[test]
fn the_markdown_summary_is_exactly_the_powershell_text() {
    // test-uat.ps1: 'the Markdown summary names the verdict and the known issue',
    // and 'a batch without Golden fields prints no golden'
    let s = merge(&four(), &lanes2(), &["p2".to_string()]);
    let md = format_markdown(&s, "lab uat first-session", "C:\\b");
    assert_eq!(
        md,
        crlf(&[
            "# lab uat first-session: FAIL (1/4 runs passed)",
            "Failed: 3 (2 known issue, 1 new)",
            "",
            "| Lane | Instance | Passed | Stopped early |",
            "|---|---|---|---|",
            "| 1 | default | 1/2 |  |",
            "| 2 | p2 | 0/2 | yes |",
            "",
            "| Row | Passed |",
            "|---|---|",
            "| FS-P1 | 3/3 |",
            "| FS-P2 | 1/3 |",
            "",
            "Failures, grouped by the first failing row:",
            "- 2x FS-P2 FAIL: clause movie-over failed (known: #1341); runs: default#2, p2#1",
            "- 1x error: lab_uat_run failed: bridge down; runs: p2#2",
            "",
            "Batch: C:\\b",
        ])
    );
    assert!(!md.contains("Golden"));
}

#[test]
fn a_clean_batch_has_no_failed_line_and_no_failure_block() {
    let s = merge(
        &[rec("default", 1, true, good(), None)],
        &[lane(1, "default")],
        &[],
    );
    let md = format_markdown(&s, "t", "C:\\b");
    assert!(
        md.starts_with("# t: PASS (1/1 runs passed)\r\n\r\n| Lane"),
        "{md:?}"
    );
    assert!(!md.contains("Failed:") && !md.contains("Failures,"));
}

#[test]
fn failure_groups_list_the_most_common_first_and_ties_keep_first_seen_order() {
    let runs = vec![
        rec(
            "p2",
            2,
            false,
            vec![],
            Some("lab_uat_run failed: bridge down"),
        ),
        rec("default", 2, false, frost(), None),
        rec("p2", 1, false, frost(), None),
    ];
    let md = format_markdown(&merge(&runs, &lanes2(), &[]), "t", "C:\\b");
    let frost_at = md.find("- 2x FS-P2 FAIL").unwrap();
    let error_at = md.find("- 1x error:").unwrap();
    assert!(frost_at < error_at, "{md}");
}

#[test]
fn the_batch_files_are_written_summary_md_first_with_a_trailing_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let started = Local
        .with_ymd_and_hms(2026, 10, 11, 12, 39, 6)
        .single()
        .unwrap();
    let s = merge(&four(), &lanes2(), &["p2".to_string()]);
    let rows = ids(&["FS-P1", "FS-P2"]);
    let meta = BatchMeta {
        dir: dir.path(),
        title: "lab uat first-session",
        section: "first-session",
        row_ids: &rows,
        lane_count: 2,
        runs_per_lease: 2,
        started,
    };
    let md = write_batch_summary(&s, &meta, started + Duration::seconds(180)).unwrap();
    let summary_md = std::fs::read_to_string(dir.path().join("summary.md")).unwrap();
    assert_eq!(summary_md, format!("{md}\r\n"));
    assert!(summary_md.ends_with("\r\n\r\n"));
    let text = std::fs::read_to_string(dir.path().join("batch.json")).unwrap();
    assert!(text.ends_with("\r\n"));
    let at = |key: &str| {
        text.find(&format!("\"{key}\":"))
            .unwrap_or_else(|| panic!("{key}"))
    };
    let order = [
        "section",
        "rows",
        "leases",
        "runs_per_lease",
        "started",
        "minutes",
        "summary",
    ];
    assert!(order.windows(2).all(|w| at(w[0]) < at(w[1])), "{text}");
    assert!(
        text.contains("\"minutes\": 3,"),
        "a whole number prints as 3, not 3.0: {text}"
    );
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["section"], "first-session");
    assert_eq!(v["leases"], 2);
    assert_eq!(v["rows"], serde_json::json!(["FS-P1", "FS-P2"]));
    assert_eq!(v["summary"]["runs"], 4);
    let started_text = v["started"].as_str().unwrap();
    assert!(
        regex::Regex::new(r"^2026-10-11T12:39:06\.0000000[+-]\d\d:\d\d$")
            .unwrap()
            .is_match(started_text),
        "{started_text}"
    );
}

#[test]
fn minutes_round_to_one_decimal_with_halves_to_even() {
    let dir = tempfile::tempdir().unwrap();
    let started = Local
        .with_ymd_and_hms(2026, 10, 11, 12, 39, 6)
        .single()
        .unwrap();
    let s = merge(&[], &[lane(1, "default")], &[]);
    let meta = BatchMeta {
        dir: dir.path(),
        title: "t",
        section: "s",
        row_ids: &[],
        lane_count: 1,
        runs_per_lease: 1,
        started,
    };
    write_batch_summary(&s, &meta, started + Duration::seconds(150)).unwrap();
    let text = std::fs::read_to_string(dir.path().join("batch.json")).unwrap();
    assert!(text.contains("\"minutes\": 2.5,"), "{text}");
}
