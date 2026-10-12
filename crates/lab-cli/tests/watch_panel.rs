//! The batch panel: state, counts, the last failure, braked lanes and lane
//! deaths. Ports the 'finding and reading a batch', 'the panel', 'a braked
//! lane' and 'a lane death' checks of tools/lab/cli/test-watch.ps1.

mod watch_support;

use cimmeria_lab_cli::batch::{RunRecord, Verdict};
use cimmeria_lab_cli::commands::watch::snapshot::{issue_of, panel, read_batch, BatchState};
use std::fs;
use watch_support::{
    alive, batch_with, brake_record, dead, death_record, instance, snap, temp_fixtures, two_lanes,
    NEW_BATCH, OLD_BATCH,
};

// 'a plan whose pid is alive is live'
#[test]
fn a_plan_whose_pid_is_alive_is_live() {
    let tmp = temp_fixtures();
    let files = read_batch(&tmp.path().join(NEW_BATCH), &alive).unwrap();
    assert_eq!(files.state, BatchState::Live);
}

// 'a plan whose pid is dead is interrupted'
#[test]
fn a_plan_whose_pid_is_dead_is_interrupted() {
    let tmp = temp_fixtures();
    let files = read_batch(&tmp.path().join(NEW_BATCH), &dead).unwrap();
    assert_eq!(files.state, BatchState::Interrupted);
}

// 'the pre-LW-02 batch with batch.json is done, with no plan'
#[test]
fn the_pre_plan_batch_with_batch_json_is_done_with_no_plan() {
    let tmp = temp_fixtures();
    let files = read_batch(&tmp.path().join(OLD_BATCH), &alive).unwrap();
    assert_eq!(files.state, BatchState::Done);
    assert!(files.plan.is_none());
}

// 'without plan.json or batch.json the state is unknown'
#[test]
fn without_plan_json_or_batch_json_the_state_is_unknown() {
    let tmp = temp_fixtures();
    fs::remove_file(tmp.path().join(OLD_BATCH).join("batch.json")).unwrap();
    let files = read_batch(&tmp.path().join(OLD_BATCH), &alive).unwrap();
    assert_eq!(files.state, BatchState::Unknown);
}

// New: batch.json wins over a live pid (done is done).
#[test]
fn batch_json_wins_over_a_live_pid() {
    let tmp = temp_fixtures();
    fs::write(tmp.path().join(NEW_BATCH).join("batch.json"), "{}").unwrap();
    let files = read_batch(&tmp.path().join(NEW_BATCH), &alive).unwrap();
    assert_eq!(files.state, BatchState::Done);
}

fn new_panel() -> cimmeria_lab_cli::commands::watch::snapshot::BatchView {
    let tmp = temp_fixtures();
    panel(&read_batch(&tmp.path().join(NEW_BATCH), &alive).unwrap()).0
}

// 'runs 3 of 6'
#[test]
fn runs_3_of_6() {
    let view = new_panel();
    assert_eq!((view.runs, view.total), (3, Some(6)));
    assert_eq!(view.runs_text(), "3/6");
    assert_eq!(view.section.as_deref(), Some("first-session"));
}

// 'passed 1, failed 2'
#[test]
fn passed_1_failed_2() {
    let view = new_panel();
    assert_eq!((view.passed, view.failed), (1, 2));
}

// 'the known-issue table re-classifies the untagged login failure'
#[test]
fn the_known_issue_table_reclassifies_the_untagged_login_failure() {
    // default#2's own Verdict.Issue is null; the table tags it #1343, p2#1 is #1341.
    assert_eq!(new_panel().known, 2);
}

// 'the last failure is default#2, #1343'
#[test]
fn the_last_failure_is_default_2_with_1343() {
    let view = new_panel();
    let last = view.last_failure.unwrap();
    assert_eq!(last.run, "default#2");
    assert_eq!(last.issue.as_deref(), Some("#1343"));
}

// 'the failure reason is cut to 72 with ...'
#[test]
fn the_failure_reason_is_cut_to_72_with_three_dots() {
    let why = new_panel().last_failure.unwrap().why;
    assert!(why.chars().count() <= 72 && why.ends_with("..."), "{why}");
    assert_eq!(
        why,
        "FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch..."
    );
}

// 'an Error record has no issue'
#[test]
fn an_error_record_has_no_issue() {
    let error_only = RunRecord {
        error: Some("lane: boom".into()),
        ..Default::default()
    };
    assert_eq!(issue_of(&error_only), None);
    // Even when its verdict would match the table: an error record is never known.
    let with_verdict = RunRecord {
        error: Some("lane: boom".into()),
        verdict: Some(Verdict {
            row: Some("FS-P2".into()),
            reason: Some("movie-over".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(issue_of(&with_verdict), None);
}

// New: the record's own Verdict.Issue is only the fallback when the table has no match.
#[test]
fn the_records_own_issue_is_the_fallback() {
    let r = RunRecord {
        verdict: Some(Verdict {
            row: Some("FS-P3".into()),
            reason: Some("nothing known".into()),
            issue: Some("#9999".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(issue_of(&r).as_deref(), Some("#9999"));
}

// 'a braked lane reads <n>/3 braked'
#[test]
fn a_braked_lane_reads_n_of_3_braked() {
    let tmp = temp_fixtures();
    let dir = batch_with(tmp.path(), "batch-braked", &brake_record());
    let status = two_lanes(tmp.path());
    let s = snap(Some(&status), Some(&dir), &[], &[], &alive);
    assert_eq!(instance(&s, "p2").run, "2/3 braked");
}

// 'a lane death is not a run'
#[test]
fn a_lane_death_is_not_a_run() {
    let tmp = temp_fixtures();
    let dir = batch_with(tmp.path(), "batch-death", &death_record());
    let (view, _) = panel(&read_batch(&dir, &alive).unwrap());
    assert_eq!((view.runs, view.failed), (3, 2));
}

// 'a lane death can be the last failure'
#[test]
fn a_lane_death_can_be_the_last_failure() {
    let tmp = temp_fixtures();
    let dir = batch_with(tmp.path(), "batch-death", &death_record());
    let (view, _) = panel(&read_batch(&dir, &alive).unwrap());
    assert_eq!(view.last_failure.unwrap().run, "p2#0");
}

// 'a lane death brakes its lane'
#[test]
fn a_lane_death_brakes_its_lane() {
    let tmp = temp_fixtures();
    let dir = batch_with(tmp.path(), "batch-death", &death_record());
    let status = two_lanes(tmp.path());
    let s = snap(Some(&status), Some(&dir), &[], &[], &alive);
    assert_eq!(instance(&s, "p2").run, "1/3 braked");
}
