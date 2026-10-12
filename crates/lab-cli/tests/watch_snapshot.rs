//! Instance rows, orphans, the old daemon, screenshot age and the log counts.
//! Ports the 'snapshot with the two-lane status', 'the old daemon', 'screenshot
//! age' and 'labd.log counts' checks of tools/lab/cli/test-watch.ps1.

mod watch_support;

use std::fs;
use std::time::SystemTime;

use cimmeria_lab_cli::commands::watch::snapshot::{
    lease_cell, log_counts, shot_age, since_stamp, LogCounts,
};
use serde_json::json;
use tempfile::TempDir;
use watch_support::{
    alive, instance, log_lines, names, now, old_daemon, snap, temp_fixtures, two_lanes, NEW_BATCH,
    OLD_BATCH,
};

const SGW: [u32; 2] = [49448, 63904];

// 'the daemon is up and serves run progress'
#[test]
fn the_daemon_is_up_and_serves_run_progress() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &log_lines(tmp.path()),
        &SGW,
        &alive,
    );
    assert!(s.daemon_up && !s.old_daemon);
}

// 'RUN default 3/3, p2 2/3, p3 -'
#[test]
fn run_reads_3_of_3_2_of_3_and_a_dash() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert_eq!(instance(&s, "default").run, "3/3");
    assert_eq!(instance(&s, "p2").run, "2/3");
    assert_eq!(instance(&s, "p3").run, "-");
}

// 'ROW default FS-P3 42s'
#[test]
fn row_default_is_fs_p3_for_42_seconds() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let i = instance(&s, "default");
    assert_eq!((i.row.as_str(), i.row_s), ("FS-P3", Some(42)));
    assert_eq!(i.client_pid, Some(49448));
}

// 'ROW p2 FS-02 3s'
#[test]
fn row_p2_is_fs_02_for_3_seconds() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let i = instance(&s, "p2");
    assert_eq!((i.row.as_str(), i.row_s), ("FS-02", Some(3)));
}

// 'p3 is free with no row'
#[test]
fn p3_is_free_with_no_row() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let i = instance(&s, "p3");
    assert_eq!(
        (i.row.as_str(), i.lease.as_deref(), i.client_pid),
        ("-", Some("free"), None)
    );
}

// 'the lease purpose is redacted and cut'
#[test]
fn the_lease_purpose_is_redacted_and_cut() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let lease = instance(&s, "p2").lease.clone().unwrap();
    assert!(
        lease.starts_with("lab_uat_run (") && lease.ends_with("...)"),
        "{lease}"
    );
    assert!(lease.contains("lease-<redacted>"), "{lease}");
    assert_eq!(
        lease,
        "lab_uat_run (UAT run lease-<redacted>, sections first...)"
    );
}

// New: an owner with no purpose, and a lease that is not held.
#[test]
fn a_lease_without_a_purpose_is_the_owner_alone() {
    assert_eq!(
        lease_cell(Some(
            &json!({"held": true, "lease": {"owner": "claude-a1"}})
        )),
        "claude-a1"
    );
    assert_eq!(lease_cell(Some(&json!({"held": false}))), "free");
    assert_eq!(lease_cell(None), "free");
}

// 'LAST is the lane's last progress line'
#[test]
fn last_is_the_lanes_last_progress_line() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert_eq!(
        instance(&s, "p2").last.as_deref(),
        Some("[p2 1/3] FAIL FS-P2: clause movie-over failed: client_entity_find/count gte 1 [known #1341] (130s)")
    );
}

// 'LAST carries the re-classified issue'
#[test]
fn last_carries_the_reclassified_issue() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let last = instance(&s, "default").last.clone().unwrap();
    assert!(last.starts_with("[default 2/3] FAIL FS-01: "), "{last}");
    assert!(last.contains("[known #1343]"), "{last}");
}

// 'orphans are SGW pids no instance owns'
#[test]
fn orphans_are_sgw_pids_no_instance_owns() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &SGW,
        &alive,
    );
    assert_eq!(s.orphans, vec![63904]);
}

// 'no orphans when the daemon is down'
#[test]
fn no_orphans_when_the_daemon_is_down() {
    let tmp = temp_fixtures();
    let s = snap(None, Some(&tmp.path().join(NEW_BATCH)), &[], &SGW, &alive);
    assert!(s.orphans.is_empty() && !s.daemon_up);
}

// 'daemon down: one row per plan lane'
#[test]
fn daemon_down_gives_one_row_per_plan_lane() {
    let tmp = temp_fixtures();
    let s = snap(None, Some(&tmp.path().join(NEW_BATCH)), &[], &[], &alive);
    assert_eq!(names(&s), "default,p2");
    assert_eq!(instance(&s, "default").lease, None);
    assert_eq!(instance(&s, "default").client_pid, None);
}

// 'a plan lane /status lacks is added after its rows'
#[test]
fn a_plan_lane_status_lacks_is_added_after_its_rows() {
    let tmp = temp_fixtures();
    let full = two_lanes(tmp.path());
    let rows: Vec<_> = full["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["instance"] != "p2")
        .cloned()
        .collect();
    let status = json!({"daemon": full["daemon"].clone(), "instances": rows});
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert_eq!(names(&s), "default,p3,p2");
}

// 'a daemon without uat reads ROW ? and old_daemon'
#[test]
fn a_daemon_without_uat_reads_row_question_mark_and_old_daemon() {
    let tmp = temp_fixtures();
    let status = old_daemon(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert!(s.old_daemon);
    assert!(s.instances.iter().all(|i| i.row == "?"));
}

// New: a `uat` that is null (between rows) is not an old daemon.
#[test]
fn a_null_uat_is_between_rows_not_an_old_daemon() {
    let tmp = temp_fixtures();
    let mut status = two_lanes(tmp.path());
    status["instances"][0]["uat"] = serde_json::Value::Null;
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert!(!s.old_daemon);
    assert_eq!(instance(&s, "default").row, "-");
}

fn run_dir_with_png_aged_5s() -> TempDir {
    let run = TempDir::new().unwrap();
    let png = run
        .path()
        .join("rows")
        .join("s")
        .join("A")
        .join("final.png");
    fs::create_dir_all(png.parent().unwrap()).unwrap();
    fs::write(&png, [1u8, 2, 3]).unwrap();
    let file = fs::File::options().write(true).open(&png).unwrap();
    file.set_modified(SystemTime::from(now() - chrono::Duration::seconds(5)))
        .unwrap();
    run
}

// 'the newest screenshot is 5 s old'
#[test]
fn the_newest_screenshot_is_5_seconds_old() {
    let run = run_dir_with_png_aged_5s();
    assert_eq!(shot_age(run.path().to_str().unwrap(), now()), Some(5));
}

// 'a run dir without rows has no screenshot age'
#[test]
fn a_run_dir_without_rows_has_no_screenshot_age() {
    let tmp = TempDir::new().unwrap();
    assert_eq!(shot_age(tmp.path().to_str().unwrap(), now()), None);
    assert_eq!(shot_age("", now()), None);
}

// 'SHOT ages the screenshot under /status uat's run_dir'
#[test]
fn shot_ages_the_screenshot_under_status_uats_run_dir() {
    let tmp = temp_fixtures();
    let run = run_dir_with_png_aged_5s();
    let mut status = two_lanes(tmp.path());
    status["instances"][0]["uat"]["run_dir"] = json!(run.path().to_str().unwrap());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert_eq!(instance(&s, "default").shot_s, Some(5));
}

const SINCE: &str = "2026-10-11T01:39:06.0000000Z";

// 'one relaunch on p2 since the plan started (the earlier one excluded)'
#[test]
fn one_relaunch_on_p2_since_the_plan_started() {
    let tmp = temp_fixtures();
    let counts = log_counts(&log_lines(tmp.path()), SINCE);
    assert_eq!(counts.relaunches, vec![("p2".to_string(), 1)]);
}

// 'one login retry'
#[test]
fn one_login_retry() {
    let tmp = temp_fixtures();
    assert_eq!(log_counts(&log_lines(tmp.path()), SINCE).login_retries, 1);
}

// 'the snapshot counts the log since the plan start'
#[test]
fn the_snapshot_counts_the_log_since_the_plan_start() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &log_lines(tmp.path()),
        &[],
        &alive,
    );
    assert_eq!(
        s.logs,
        Some(LogCounts {
            relaunches: vec![("p2".to_string(), 1)],
            login_retries: 1
        })
    );
}

// 'a relaunch line without an instance span counts for default'
#[test]
fn a_relaunch_line_without_an_instance_span_counts_for_default() {
    let line = "2026-10-11T01:40:00.000000Z  INFO lab_watchdog{pid=1}: relaunched after crash; logging back in new_pid=2";
    let counts = log_counts(&[line.to_string()], "");
    assert_eq!(counts.relaunches, vec![("default".to_string(), 1)]);
}

// New: the plan's start time as the log stamp compares it (7 fractional digits).
#[test]
fn the_plan_start_is_formatted_as_a_log_stamp() {
    let t = "2026-10-11T01:39:06.5Z".parse().unwrap();
    assert_eq!(since_stamp(t), "2026-10-11T01:39:06.5000000Z");
}

// New: with no plan there is no log count.
#[test]
fn without_a_plan_there_are_no_log_counts() {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(OLD_BATCH)),
        &log_lines(tmp.path()),
        &[],
        &alive,
    );
    assert_eq!(s.logs, None);
}
