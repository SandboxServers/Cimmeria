//! LX-12a: argument checks, the pose line, exit codes and the --json objects.
//! Ports checks [1]-[24] and [28]-[32] of the removed PowerShell test file.

use cimmeria_lab_cli::commands::calibrate::outcome::*;
use cimmeria_lab_cli::commands::calibrate::render::{render_diff, render_line};
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::ui::{strip_ansi, Ui};
use serde_json::{json, Value};

const FROST_DIFF: &str = r"C:\lab\uat-runs\2026-10-11-1489a731-kqzxwbt\first-session.diff";
const LONG_DIFF: &str = r"C:\Users\lab-operator\AppData\Local\cimmeria-lab\uat-runs\2026-10-11-1489a731-kqzxwbt\first-session.diff";
const RUN_DIR_LONG: &str =
    r"C:\Users\lab-operator\AppData\Local\cimmeria-lab\uat-runs\2026-10-11-1489a731-kqzxwbt";

fn out(v: Value) -> Outcome {
    Outcome::from_value(&v)
}
fn frost() -> Outcome {
    out(
        json!({"ok": true, "section": "first-session", "row": "FS-P3", "pose": "frost", "score": 8312,
        "confirmed": 3, "changed": {"bearing_deg": [322.5, 330.0]}, "diff_path": FROST_DIFF, "probes": 41}),
    )
}
fn same() -> Outcome {
    out(json!({"ok": true, "pose": "supply", "score": 7400, "confirmed": 3, "probes": 30}))
}
fn miss() -> Outcome {
    out(
        json!({"ok": false, "pose": "frost", "confirmed": 0, "probes": 120, "reason": "no candidate hovered the target"}),
    )
}
fn long_ok() -> Outcome {
    out(
        json!({"ok": true, "pose": "frost", "score": 8312, "confirmed": 3,
        "changed": {"bearing_deg": [322.5, 330.0], "radius_m": [3.75, 4.5]}, "diff_path": LONG_DIFF, "probes": 41}),
    )
}
fn fail_long() -> Outcome {
    let reason = format!("budget: not started {}", "x".repeat(300));
    out(json!({"ok": false, "pose": "supply", "confirmed": 0, "probes": 300, "reason": reason}))
}
fn keys(v: &Value) -> Vec<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

// [1] 'no row is a usage error'
#[test]
fn no_row_is_a_usage_error() {
    assert_eq!(
        check_arguments("first-session", "", None).as_deref(),
        Some(USAGE)
    );
    assert!(USAGE.starts_with("usage: lab calibrate"));
}
// [2] 'no section is a usage error'
#[test]
fn no_section_is_a_usage_error() {
    assert_eq!(check_arguments("", "", None).as_deref(), Some(USAGE));
}
// [3] 'a budget of 0 is refused'  [4] 'a budget of 31 is refused'
#[test]
fn budgets_outside_1_to_30_are_refused() {
    for b in [0, 31] {
        assert_eq!(
            check_arguments("first-session", "FS-P3", Some(b)).as_deref(),
            Some("--budget-minutes must be 1 to 30")
        );
    }
}
// [5] 'budgets 1 and 30 are allowed'
#[test]
fn budgets_1_and_30_are_allowed() {
    assert_eq!(check_arguments("first-session", "FS-P3", Some(1)), None);
    assert_eq!(check_arguments("first-session", "FS-P3", Some(30)), None);
}
// [6] 'no budget is the tool default'
#[test]
fn no_budget_is_the_tool_default() {
    assert_eq!(check_arguments("first-session", "FS-P3", None), None);
}
// [7] "a changed pose's line"
#[test]
fn a_changed_poses_line() {
    assert_eq!(
        format_line(&frost()),
        format!("frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff {FROST_DIFF}")
    );
}
// [8] "an unchanged pose's line"
#[test]
fn an_unchanged_poses_line() {
    assert_eq!(
        format_line(&same()),
        "supply: score 7400, confirmed 3/3, unchanged"
    );
}
// [9] 'an unset field reads none, every changed field is listed'. The
// daemon's JSON object is sorted by key (a `serde_json::Map` without
// `preserve_order`), so yaw comes before zoom; the PowerShell test built its
// object by hand with zoom first, which the daemon never sends.
#[test]
fn an_unset_field_reads_none_and_every_changed_field_is_listed() {
    let guard = out(
        json!({"ok": true, "pose": "guard", "score": 6000, "confirmed": 3, "probes": 9,
        "changed": {"zoom": ["none", 250.0], "yaw_offset_deg": [0, -12.5]}}),
    );
    assert_eq!(
        format_line(&guard),
        "guard: score 6000, confirmed 3/3, yaw 0.0 -> -12.5, zoom none -> 250.0"
    );
}
// New: the fields print in the order of the daemon's JSON (its keys sorted),
// not in a fixed radius-first order, in the line and in the `--json` object.
#[test]
fn changed_fields_print_in_the_daemons_key_order() {
    let all = out(
        json!({"ok": true, "pose": "frost", "score": 1, "confirmed": 3,
        "changed": {"zoom": ["none", 250.0], "yaw_offset_deg": [0, -12.5], "radius_m": [4.16, 3.0],
        "pitch_deg": [-28.8, -30.0], "bearing_deg": [322.5, 330.0]}}),
    );
    assert_eq!(
        format_line(&all),
        "frost: score 1, confirmed 3/3, bearing 322.5 -> 330.0, pitch -28.8 -> -30.0, radius 4.16 -> 3.0, yaw 0.0 -> -12.5, zoom none -> 250.0"
    );
    let j = compact_json("s", "r", &[all], false);
    let pos = |k: &str| j.find(&format!("\"{k}\":")).unwrap();
    assert!(
        pos("bearing") < pos("pitch")
            && pos("pitch") < pos("radius")
            && pos("radius") < pos("yaw")
            && pos("yaw") < pos("zoom"),
        "{j}"
    );
}
// [10] 'a pose with no candidate'
#[test]
fn a_pose_with_no_candidate() {
    assert_eq!(
        format_line(&miss()),
        "frost: no proposal: no candidate hovered the target"
    );
}
// [11] 'a failed pose with a best score; lease ids hidden'
#[test]
fn a_failed_pose_with_a_best_score_hides_lease_ids() {
    let lease = out(
        json!({"ok": false, "pose": "frost", "score": 5100, "confirmed": 1, "probes": 60,
        "reason": "lease revoked: lease-0123456789abcdef by x"}),
    );
    assert_eq!(
        format_line(&lease),
        "frost: no proposal: lease revoked: lease-<redacted> by x (best score 5100, confirmed 1/3)"
    );
}
// [12] 'an early stop says why'
#[test]
fn an_early_stop_says_why() {
    let budget = out(
        json!({"ok": true, "pose": "frost", "score": 8000, "confirmed": 3, "probes": 99,
        "reason": "budget: swept 40 of 108 candidates"}),
    );
    assert_eq!(
        format_line(&budget),
        "frost: score 8000, confirmed 3/3, unchanged (budget: swept 40 of 108 candidates)"
    );
}
// [13] 'every pose proposed or unchanged exits 0'
#[test]
fn every_pose_proposed_or_unchanged_exits_0() {
    assert_eq!(exit_code(&[frost(), same()]), 0);
}
// [14] 'a pose with no candidate exits 1'
#[test]
fn a_pose_with_no_candidate_exits_1() {
    assert_eq!(exit_code(&[frost(), miss()]), 1);
}
// [15] 'no outcomes exits 1'
#[test]
fn no_outcomes_exits_1() {
    assert_eq!(exit_code(&[]), 1);
}

fn two_outcome_json() -> (String, Value) {
    let j = compact_json("first-session", "FS-P3", &[long_ok(), fail_long()], false);
    let v: Value = serde_json::from_str(&j).unwrap();
    (j, v)
}
// [16] 'two outcomes stay under 400 characters'
#[test]
fn two_outcomes_stay_under_400_characters() {
    let (j, _) = two_outcome_json();
    assert!(j.chars().count() < 400, "{} chars: {j}", j.chars().count());
}
// [17] 'no nulls and no empty arrays'
#[test]
fn no_nulls_and_no_empty_arrays() {
    let (j, _) = two_outcome_json();
    assert!(!j.contains("null") && !j.contains("[]"), "{j}");
}
// [18] 'the object names the section and row, ok false when a pose failed'
#[test]
fn the_object_names_the_section_and_row() {
    let (_, v) = two_outcome_json();
    assert_eq!(v["ok"], json!(false));
    assert_eq!(v["section"], "first-session");
    assert_eq!(v["row"], "FS-P3");
    assert_eq!(v["outcomes"].as_array().unwrap().len(), 2);
}
// [19] 'changed fields and the diff path'
#[test]
fn changed_fields_and_the_diff_path() {
    let (_, v) = two_outcome_json();
    let o = &v["outcomes"][0];
    assert_eq!(o["changed"]["bearing"], "322.5 -> 330.0");
    assert_eq!(o["changed"]["radius"], "3.75 -> 4.5");
    assert!(o["diff"].as_str().unwrap().ends_with("first-session.diff"));
}
// [20] 'a failed pose has no score, diff or changed'
#[test]
fn a_failed_pose_has_no_score_diff_or_changed() {
    let (_, v) = two_outcome_json();
    let k = keys(&v["outcomes"][1]);
    for gone in ["score", "diff", "changed"] {
        assert!(!k.contains(&gone.to_string()), "{k:?}");
    }
}
// [21] 'a reason is cut to 40 characters'
#[test]
fn a_reason_is_cut_to_40_characters() {
    let (_, v) = two_outcome_json();
    let r = v["outcomes"][1]["reason"].as_str().unwrap();
    assert_eq!(r.chars().count(), 40);
    assert!(r.ends_with("..."));
}
// [22] 'probes and candidates stay out'
#[test]
fn probes_and_candidates_stay_out() {
    let (_, v) = two_outcome_json();
    assert!(!keys(&v["outcomes"][0]).contains(&"probes".to_string()));
}
// [23] 'an unchanged pose has no changed key; applied is shown'
#[test]
fn an_unchanged_pose_has_no_changed_key_and_applied_is_shown() {
    let v: Value =
        serde_json::from_str(&compact_json("first-session", "FS-P3", &[same()], true)).unwrap();
    assert_eq!(v["ok"], json!(true));
    assert_eq!(v["applied"], json!(true));
    assert!(!keys(&v["outcomes"][0]).contains(&"changed".to_string()));
}
// [24] 'no outcomes: no empty array'
#[test]
fn no_outcomes_no_empty_array() {
    assert_eq!(
        compact_json("first-session", "FS-P3", &[], false),
        r#"{"ok":false,"section":"first-session","row":"FS-P3"}"#
    );
}
// New: the key order of the object and of a pose is the PowerShell's.
#[test]
fn key_order_is_ok_section_row_outcomes_applied() {
    let j = compact_json("first-session", "FS-P3", &[long_ok()], true);
    let pos = |k: &str| j.find(&format!("\"{k}\":")).unwrap();
    assert!(
        pos("ok") < pos("section") && pos("section") < pos("row") && pos("row") < pos("outcomes")
    );
    assert!(pos("outcomes") < pos("applied"));
    assert!(
        pos("pose") < pos("score")
            && pos("score") < pos("confirmed")
            && pos("confirmed") < pos("changed")
    );
    assert!(pos("changed") < pos("diff"));
}

fn apply_msg() -> String {
    format!("{LONG_DIFF}: git apply failed: error: patch failed: docs/guides/uat-specs/first-session.toml:12 (--apply)")
}
// [28] 'a failed -Apply keeps run_dir and diff; the error starts with the diff path'
#[test]
fn a_failed_apply_keeps_run_dir_and_diff() {
    let j = apply_error_json(
        &apply_msg(),
        RUN_DIR_LONG,
        LONG_DIFF,
        &[long_ok(), fail_long()],
        APPLY_ERROR_MAX,
    );
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(
        (v["ok"].clone(), v["exit"].clone()),
        (json!(false), json!(2))
    );
    assert!(v["error"].as_str().unwrap().starts_with(LONG_DIFF));
    assert_eq!(v["run_dir"], RUN_DIR_LONG);
    assert_eq!(v["diff"], LONG_DIFF);
}
// [29] 'the outcomes are kept first to last, without a second copy of the diff path'
#[test]
fn the_outcomes_are_kept_without_a_second_diff_path() {
    let j = apply_error_json(
        &apply_msg(),
        RUN_DIR_LONG,
        LONG_DIFF,
        &[long_ok(), fail_long()],
        APPLY_ERROR_MAX,
    );
    let v: Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["outcomes"].as_array().unwrap().len(), 2);
    assert!(v.get("outcomes_cut").is_none());
    assert_eq!(v["outcomes"][0]["changed"]["bearing"], "322.5 -> 330.0");
    assert!(!keys(&v["outcomes"][0]).contains(&"diff".to_string()));
}
// [30] 'a failed -Apply with two poses and long paths stays within 700 characters'
#[test]
fn a_failed_apply_with_long_paths_stays_within_700_characters() {
    let j = apply_error_json(
        &apply_msg(),
        RUN_DIR_LONG,
        LONG_DIFF,
        &[long_ok(), fail_long()],
        APPLY_ERROR_MAX,
    );
    assert!(
        j.chars().count() <= 700 && !j.contains("null"),
        "{} chars",
        j.chars().count()
    );
}
// [31] 'too many outcomes give way to outcomes_cut'
#[test]
fn too_many_outcomes_give_way_to_outcomes_cut() {
    let many: Vec<Outcome> = ["a1", "a2", "a3", "a4", "a5", "a6"]
        .iter()
        .map(|p| Outcome {
            pose: p.to_string(),
            ..long_ok()
        })
        .collect();
    let j = apply_error_json(
        &apply_msg(),
        RUN_DIR_LONG,
        LONG_DIFF,
        &many,
        APPLY_ERROR_MAX,
    );
    let v: Value = serde_json::from_str(&j).unwrap();
    let kept = v.get("outcomes").map_or(0, |o| o.as_array().unwrap().len());
    let cut = v["outcomes_cut"].as_u64().unwrap() as usize;
    assert!(
        j.chars().count() <= 700 && cut >= 5 && kept + cut == 6,
        "{kept} kept, {cut} cut"
    );
    assert_eq!(v["diff"], LONG_DIFF);
}
// [32] 'the call timeout is the budget plus 5 minutes'
#[test]
fn the_call_timeout_is_the_budget_plus_5_minutes() {
    assert_eq!(timeout_secs(None), 1200);
    assert_eq!(timeout_secs(Some(30)), 2100);
}

// New: values print with one to three decimals; a long reason's newlines collapse.
#[test]
fn values_print_with_one_to_three_decimals() {
    assert_eq!(format_value(&json!(330)), "330.0");
    assert_eq!(format_value(&json!(3.75)), "3.75");
    assert_eq!(format_value(&json!(-12.5)), "-12.5");
    assert_eq!(format_value(&json!(1.23456)), "1.235");
    assert_eq!(format_value(&json!("none")), "none");
}
// New: a failed call is named once, the first line only, lease ids hidden.
#[test]
fn a_failed_call_is_named_once() {
    assert_eq!(
        format_call_error("lab_uat_calibrate failed: lease-0123456789abcdef held\nmore"),
        "lab_uat_calibrate failed: lease-<redacted> held"
    );
    assert_eq!(
        format_call_error("timed out"),
        "lab_uat_calibrate failed: timed out"
    );
    let tool = McpError::Tool("boom".into());
    assert_eq!(
        format_call_error(&call_error_text("lab_uat_calibrate", &tool)),
        "lab_uat_calibrate failed: boom"
    );
}
// New: the stop object of a usage or pre-flight failure.
#[test]
fn the_stop_object_is_ok_exit_error() {
    assert_eq!(
        stop_json(3, "the lab daemon is not answering"),
        r#"{"ok":false,"exit":3,"error":"the lab daemon is not answering"}"#
    );
    let long = stop_json(2, &"y".repeat(400));
    let v: Value = serde_json::from_str(&long).unwrap();
    assert_eq!(v["error"].as_str().unwrap().chars().count(), 300);
}
// New: fancy mode (decision in the packet): glyphs, bold pose, arrow, red and green diff.
#[test]
fn fancy_lines_and_diff() {
    let ui = Ui {
        color: true,
        fancy: true,
        width: None,
    };
    let plain = |o: &Outcome| strip_ansi(&render_line(&ui, o));
    assert_eq!(
        plain(&frost()),
        "✓ frost  score 8312 · confirmed 3/3 · bearing 322.5 → 330.0"
    );
    assert_eq!(
        plain(&same()),
        "· supply  unchanged · score 7400 · confirmed 3/3"
    );
    assert_eq!(
        plain(&miss()),
        "✗ frost  no proposal: no candidate hovered the target"
    );
    let lease =
        out(json!({"ok": false, "pose": "frost", "score": 5100, "confirmed": 1, "reason": "x"}));
    assert_eq!(
        plain(&lease),
        "✗ frost  no proposal: x (best score 5100, confirmed 1/3)"
    );
    assert!(
        render_line(&ui, &frost()).contains("\x1b[32m✓"),
        "green tick"
    );
    let d = render_diff(&ui, "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n");
    assert!(
        d.contains("\x1b[1;31m-old") && d.contains("\x1b[32m+new"),
        "{d:?}"
    );
    let raw = "--- a/x\n+++ b/x\n-old\n+new\n";
    assert_eq!(render_diff(&Ui::plain(), raw), raw);
    assert_eq!(render_line(&Ui::plain(), &frost()), format_line(&frost()));
}
