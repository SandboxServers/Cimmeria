mod uat_support;

use chrono::{TimeZone, Timelike, Utc};
use cimmeria_lab_cli::batch::RunRecord;
use cimmeria_lab_cli::commands::uat::record::{
    apply_error, apply_reply, brake, error_text, is_timeout, lane_death, new_record, run_verdict,
    stop_lane, utc_stamp,
};
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::json;
use uat_support::{ids, row};

#[test]
fn all_pass_is_ok() {
    // test-uat.ps1: 'all PASS is ok'
    let v = run_verdict(&[row("FS-01", "PASS", ""), row("FS-P1", "PASS", "")], &[]);
    assert!(v.ok);
    assert_eq!(v.row, None);
    assert_eq!(v.issue, None);
}

#[test]
fn the_first_failing_row_decides_and_fs_p2_movie_over_is_known() {
    // test-uat.ps1: 'the first failing row decides; FS-P2 movie-over is known #1341'
    let v = run_verdict(
        &[
            row("FS-P1", "PASS", ""),
            row(
                "FS-P2",
                "FAIL",
                "clause movie-over failed: client_entity_find/count gte 1",
            ),
            row("FS-P3", "FAIL", "step action 1 failed"),
        ],
        &[],
    );
    assert!(!v.ok);
    assert_eq!(v.row.as_deref(), Some("FS-P2"));
    assert_eq!(v.result.as_deref(), Some("FAIL"));
    assert_eq!(v.issue.as_deref(), Some("#1341"));
}

#[test]
fn any_other_failure_is_new() {
    // test-uat.ps1: 'any other failure is new' and 'a row and reason no known issue covers have no issue'
    let v = run_verdict(&[row("FS-P3", "FAIL", "clause aim failed")], &[]);
    assert!(!v.ok);
    assert_eq!(v.issue, None);
}

#[test]
fn blocked_fails_a_run() {
    // test-uat.ps1: 'BLOCKED fails a run'
    let v = run_verdict(&[row("FS-P1", "BLOCKED", "setup failed")], &[]);
    assert!(!v.ok);
    assert_eq!(v.result.as_deref(), Some("BLOCKED"));
}

#[test]
fn the_empty_login_password_box_is_known_1343() {
    // test-uat.ps1: 'the empty login password box is known #1343'
    let v = run_verdict(
        &[row(
            "FS-01",
            "BLOCKED",
            "Login_PasswordEdit holds 0 characters after typing, expected 4",
        )],
        &[],
    );
    assert_eq!(v.issue.as_deref(), Some("#1343"));
}

#[test]
fn a_skipped_row_does_not_fail_a_run() {
    let v = run_verdict(&[row("FS-01", "SKIPPED", "")], &[]);
    assert!(v.ok);
}

#[test]
fn a_run_that_reports_fewer_rows_than_planned_fails() {
    // test-uat.ps1: 'a run that reports fewer rows than planned fails'
    let v = run_verdict(&[row("FS-01", "PASS", "")], &ids(&["FS-01", "FS-02"]));
    assert!(!v.ok);
    assert_eq!(v.result.as_deref(), Some("MISSING"));
    assert_eq!(v.row.as_deref(), Some("FS-02"));
    assert_eq!(
        v.reason.as_deref(),
        Some("the run reported no result for FS-02")
    );
    assert_eq!(v.issue, None);
}

#[test]
fn every_missing_row_is_named_and_the_first_one_is_the_row() {
    let v = run_verdict(
        &[row("FS-01", "PASS", "")],
        &ids(&["FS-01", "FS-02", "FS-03"]),
    );
    assert_eq!(v.row.as_deref(), Some("FS-02"));
    assert_eq!(
        v.reason.as_deref(),
        Some("the run reported no result for FS-02, FS-03")
    );
}

#[test]
fn a_failing_reason_has_its_lease_id_redacted() {
    // test-uat.ps1: 'a failing reason has its lease id redacted'
    let v = run_verdict(
        &[row(
            "FS-P3",
            "FAIL",
            "lease-0123456789abcdef is not the current lease",
        )],
        &[],
    );
    assert_eq!(
        v.reason.as_deref(),
        Some("lease-<redacted> is not the current lease")
    );
}

#[test]
fn a_pass_resets_the_failure_streak() {
    // test-uat.ps1: 'a pass resets the failure streak'
    assert!(!brake(&[false, false, true, false], 3));
}

#[test]
fn three_failures_in_a_row_brake_the_lane() {
    // test-uat.ps1: 'three failures in a row brake the lane'
    assert!(brake(&[true, false, false, false], 3));
}

#[test]
fn a_brake_of_0_never_stops() {
    // test-uat.ps1: 'a brake of 0 never stops'
    assert!(!brake(&[false, false, false, false], 0));
}

#[test]
fn a_lane_that_ran_every_run_did_not_stop_early() {
    // test-uat.ps1: 'a lane that ran every run did not stop early (no brake on the last run)'
    assert!(!stop_lane(3, 3, &[false, false, false], 3));
}

#[test]
fn a_lane_with_runs_left_stops_on_the_brake() {
    // test-uat.ps1: 'a lane with runs left stops on the brake'
    assert!(stop_lane(3, 5, &[false, false, false], 3));
}

#[test]
fn a_tool_error_reads_lab_uat_run_failed_first_line_only_lease_ids_hidden() {
    let e = McpError::Tool("bridge down lease-0123456789abcdef\nsecond line".to_string());
    assert_eq!(
        error_text(&e),
        "lab_uat_run failed: bridge down lease-<redacted>"
    );
}

#[test]
fn a_transport_error_keeps_its_own_text() {
    let e = McpError::Unreachable("operation timed out".to_string());
    assert_eq!(
        error_text(&e),
        "cannot reach the lab daemon: operation timed out"
    );
}

#[test]
fn timeouts_are_recognised_in_any_case() {
    assert!(is_timeout("operation timed out"));
    assert!(is_timeout("The request was canceled"));
    assert!(is_timeout("HttpClient.Timeout of 900 seconds"));
    assert!(is_timeout("TIMEOUT"));
    assert!(!is_timeout("lab_uat_run failed: bridge down"));
}

#[test]
fn utc_stamps_have_seven_fraction_digits_and_a_z() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    assert_eq!(utc_stamp(t), "2026-10-11T01:39:06.0000000Z");
    let t = t.with_nanosecond(123_456_700).unwrap();
    assert_eq!(utc_stamp(t), "2026-10-11T01:39:06.1234567Z");
}

#[test]
fn a_reply_fills_rows_run_dir_verdict_and_ok() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let mut rec = new_record(1, "default", 1, t);
    assert_eq!(rec.started.as_deref(), Some("2026-10-11T01:39:06.0000000Z"));
    let reply = json!({"run_dir": "C:\\runs\\r1", "rows": [
        {"row": "FS-01", "result": "PASS"}, null, {"row": "FS-02", "result": "PASS"}]});
    apply_reply(&mut rec, &reply, &ids(&["FS-01", "FS-02"]));
    assert!(rec.ok);
    assert_eq!(rec.rows.len(), 2);
    assert_eq!(rec.run_dir.as_deref(), Some("C:\\runs\\r1"));
    assert!(rec.verdict.unwrap().ok);
}

#[test]
fn a_reply_without_a_run_dir_stores_an_empty_one_and_a_non_object_reply_has_no_rows() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let mut rec = new_record(1, "default", 1, t);
    apply_reply(&mut rec, &json!({"rows": []}), &[]);
    assert_eq!(rec.run_dir.as_deref(), Some(""));
    let mut rec = new_record(1, "default", 2, t);
    apply_reply(&mut rec, &json!("not json"), &ids(&["FS-01"]));
    assert!(!rec.ok);
    assert!(rec.rows.is_empty());
    assert_eq!(rec.verdict.unwrap().result.as_deref(), Some("MISSING"));
}

#[test]
fn the_stored_rows_have_their_lease_ids_hidden_too() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let mut rec = new_record(1, "default", 1, t);
    let reply = json!({"run_dir": "d", "rows": [
        {"row": "FS-P3", "result": "FAIL",
         "reasons": ["lease-0123456789abcdef is not current", "second lease-FEDCBA9876543210"]}]});
    apply_reply(&mut rec, &reply, &ids(&["FS-P3"]));
    assert_eq!(
        rec.rows[0].reasons,
        ["lease-<redacted> is not current", "second lease-<redacted>"]
    );
    let line = serde_json::to_string(&rec).unwrap();
    assert!(!line.contains("0123456789abcdef"), "{line}");
}

#[test]
fn a_failed_call_stores_the_error_as_the_reason_and_says_whether_it_timed_out() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let mut rec = new_record(1, "p2", 2, t);
    let timed_out = apply_error(&mut rec, &McpError::Tool("bridge down".to_string()));
    assert!(!timed_out);
    assert!(!rec.ok);
    assert_eq!(
        rec.error.as_deref(),
        Some("lab_uat_run failed: bridge down")
    );
    let v = rec.verdict.unwrap();
    assert_eq!(v.reason.as_deref(), Some("lab_uat_run failed: bridge down"));
    assert_eq!((v.row, v.result, v.issue), (None, None, None));
    let mut rec = new_record(1, "p2", 3, t);
    assert!(apply_error(
        &mut rec,
        &McpError::Unreachable("operation timed out".to_string())
    ));
}

#[test]
fn a_lane_death_is_index_0_braked_with_a_lane_prefixed_error() {
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let d = lane_death(2, "p2", "boom\nmore", t);
    assert_eq!(
        (d.lane, d.index, d.ok, d.braked, d.seconds),
        (2, 0, false, true, 0)
    );
    assert_eq!(d.error.as_deref(), Some("lane: boom"));
    assert_eq!(d.verdict.unwrap().reason.as_deref(), Some("boom"));
    assert_eq!(d.started, d.ended);
}

#[test]
fn a_record_serialises_with_the_powershell_keys_in_order() {
    // The batch.jsonl line `lab watch` and the PowerShell read.
    let t = Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap();
    let mut rec: RunRecord = new_record(1, "default", 1, t);
    let reply = json!({"run_dir": "C:\\runs\\r1", "rows": [{"row": "FS-01", "result": "PASS"}]});
    apply_reply(&mut rec, &reply, &ids(&["FS-01"]));
    rec.seconds = 5;
    rec.ended = Some("2026-10-11T01:41:00.0000000Z".to_string());
    assert_eq!(
        serde_json::to_string(&rec).unwrap(),
        r#"{"Lane":1,"Instance":"default","Index":1,"Ok":true,"Rows":[{"row":"FS-01","result":"PASS"}],"Verdict":{"Ok":true,"Row":null,"Result":null,"Reason":null,"Issue":null},"RunDir":"C:\\runs\\r1","Error":null,"Seconds":5,"Braked":false,"Started":"2026-10-11T01:39:06.0000000Z","Ended":"2026-10-11T01:41:00.0000000Z"}"#
    );
}
