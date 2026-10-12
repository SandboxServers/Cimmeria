mod uat_support;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use cimmeria_lab_cli::commands::uat::lanes::sleep_unless_stopped;
use cimmeria_lab_cli::daemon::mcp::McpError;
use uat_support::rig::{fail_reply, fails, pass_reply, passes, LaneRig, Reply};
use uat_support::{ids, lane, lanes2};

#[test]
fn a_lane_runs_every_run_and_appends_one_record_each() {
    let rig = LaneRig::new(vec![("default", passes(3, &["FS-01"]))]);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 3, 20, 3);
    let calls = rig.calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(calls
        .iter()
        .all(|(i, t)| i == "default" && *t == Duration::from_secs(900)));
    drop(calls);
    let records = rig.records();
    assert_eq!(
        records.iter().map(|r| r.index).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    for r in &records {
        assert!(r.ok && !r.braked);
        assert_eq!(r.run_dir.as_deref(), Some("C:\\runs\\a"));
        assert_eq!(r.rows.len(), 1);
        let (s, e) = (r.started.as_deref().unwrap(), r.ended.as_deref().unwrap());
        assert!(
            s.ends_with('Z') && e.ends_with('Z') && s.len() == 28 && s <= e,
            "{s} {e}"
        );
    }
    assert_eq!(rig.seen.lock().unwrap().len(), 3);
    assert!(
        rig.sleeps.lock().unwrap().is_empty(),
        "lane 1 does not wait"
    );
    assert!(rig.notices.lock().unwrap().is_empty());
}

#[test]
fn a_lane_stops_on_the_brake_and_says_so() {
    let rig = LaneRig::new(vec![("default", fails(5))]);
    rig.run(&[lane(1, "default")], &ids(&["FS-P3"]), 5, 0, 2);
    let records = rig.records();
    assert_eq!(records.len(), 2);
    assert!(!records[0].braked && records[1].braked);
    assert_eq!(rig.calls.lock().unwrap().len(), 2);
    assert_eq!(
        *rig.notices.lock().unwrap(),
        [(
            "default".to_string(),
            "stopped after 2 failed runs in a row".to_string()
        )]
    );
    assert!(rig.waits.lock().unwrap().is_empty());
}

#[test]
fn the_last_run_never_brakes() {
    let rig = LaneRig::new(vec![("default", fails(2))]);
    rig.run(&[lane(1, "default")], &ids(&["FS-P3"]), 2, 0, 2);
    let records = rig.records();
    assert_eq!(records.len(), 2);
    assert!(!records[1].braked);
    assert!(rig.notices.lock().unwrap().is_empty());
}

#[test]
fn a_pass_between_failures_resets_the_brake() {
    // fail, pass, fail, fail: with max 2 the lane stops after run 4, not run 3.
    let mut script: Vec<Reply> = vec![fail_reply("FS-P3", "x"), pass_reply("d", &["FS-P3"])];
    script.push(fail_reply("FS-P3", "x"));
    script.push(fail_reply("FS-P3", "x"));
    let rig = LaneRig::new(vec![("default", script)]);
    rig.run(&[lane(1, "default")], &ids(&["FS-P3"]), 5, 0, 2);
    let records = rig.records();
    assert_eq!(records.len(), 4);
    assert!(!records[2].braked && records[3].braked);
}

#[test]
fn a_timeout_waits_ten_minutes_for_the_lease_then_the_lane_goes_on() {
    let script: Vec<Reply> = vec![
        Err(McpError::Unreachable("operation timed out".to_string())),
        pass_reply("d", &["FS-01"]),
    ];
    let rig = LaneRig::new(vec![("default", script)]);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 2, 0, 3);
    assert_eq!(
        *rig.waits.lock().unwrap(),
        [("default".to_string(), Duration::from_secs(600))]
    );
    let records = rig.records();
    assert_eq!(records.len(), 2);
    assert!(!records[0].ok && !records[0].braked);
    assert_eq!(
        records[0].error.as_deref(),
        Some("cannot reach the lab daemon: operation timed out")
    );
    assert_eq!(
        records[0].verdict.as_ref().unwrap().reason.as_deref(),
        records[0].error.as_deref()
    );
    assert!(records[1].ok);
}

#[test]
fn a_lease_that_never_clears_ends_the_lane() {
    let script: Vec<Reply> = vec![Err(McpError::Unreachable(
        "operation timed out".to_string(),
    ))];
    let mut rig = LaneRig::new(vec![("default", script)]);
    rig.wait_result = false;
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 3, 0, 3);
    let records = rig.records();
    assert_eq!(records.len(), 1);
    assert!(records[0].braked);
    assert_eq!(rig.calls.lock().unwrap().len(), 1);
    assert_eq!(
        *rig.notices.lock().unwrap(),
        [(
            "default".to_string(),
            "its lease did not clear after the timeout; lane stopped".to_string()
        )]
    );
}

#[test]
fn an_error_that_is_not_a_timeout_does_not_wait() {
    let script: Vec<Reply> = vec![
        Err(McpError::Tool("bridge down".to_string())),
        pass_reply("d", &["FS-01"]),
    ];
    let rig = LaneRig::new(vec![("default", script)]);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 2, 0, 3);
    assert!(rig.waits.lock().unwrap().is_empty());
    let records = rig.records();
    assert_eq!(
        records[0].error.as_deref(),
        Some("lab_uat_run failed: bridge down")
    );
    assert!(!records[0].braked);
    assert_eq!(records[0].verdict.as_ref().unwrap().row, None);
}

#[test]
fn lease_ids_never_reach_the_record_and_only_the_first_line_is_kept() {
    let script: Vec<Reply> = vec![Err(McpError::Tool(
        "lease-0123456789abcdef is not current\nstack trace".to_string(),
    ))];
    let rig = LaneRig::new(vec![("default", script)]);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 1, 0, 3);
    let text = std::fs::read_to_string(rig.jsonl()).unwrap();
    assert!(!text.contains("0123456789abcdef"), "{text}");
    assert_eq!(
        rig.records()[0].error.as_deref(),
        Some("lab_uat_run failed: lease-<redacted> is not current")
    );
}

#[test]
fn a_reply_missing_a_planned_row_fails_the_run() {
    let rig = LaneRig::new(vec![("default", vec![pass_reply("d", &["FS-01"])])]);
    rig.run(&[lane(1, "default")], &ids(&["FS-01", "FS-02"]), 1, 0, 3);
    let r = &rig.records()[0];
    assert!(!r.ok);
    assert_eq!(
        r.verdict.as_ref().unwrap().result.as_deref(),
        Some("MISSING")
    );
}

#[test]
fn lanes_are_staggered_by_their_lane_number() {
    let rig = LaneRig::new(vec![
        ("default", passes(1, &["FS-01"])),
        ("p2", passes(1, &["FS-01"])),
        ("p3", passes(1, &["FS-01"])),
    ]);
    let lanes = [lane(1, "default"), lane(2, "p2"), lane(3, "p3")];
    rig.run(&lanes, &ids(&["FS-01"]), 1, 20, 3);
    let mut sleeps = rig.sleeps.lock().unwrap().clone();
    sleeps.sort();
    assert_eq!(sleeps, [Duration::from_secs(20), Duration::from_secs(40)]);
    assert_eq!(rig.records().len(), 3);
}

#[test]
fn lanes_run_at_the_same_time() {
    let mut rig = LaneRig::new(vec![
        ("default", passes(2, &["FS-01"])),
        ("p2", passes(2, &["FS-01"])),
    ]);
    rig.delay_ms = 150;
    rig.run(&lanes2(), &ids(&["FS-01"]), 2, 0, 3);
    assert_eq!(rig.max_in_flight.load(Ordering::SeqCst), 2);
}

#[test]
fn ctrl_c_between_runs_starts_no_new_run_and_keeps_the_run_under_way() {
    let mut rig = LaneRig::new(vec![("default", passes(5, &["FS-01"]))]);
    rig.stop_on_call = Some(2);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 5, 0, 3);
    assert_eq!(rig.calls.lock().unwrap().len(), 2);
    let records = rig.records();
    assert_eq!(records.len(), 2);
    assert!(records.iter().all(|r| r.ok));
}

#[test]
fn a_stop_before_the_lane_starts_runs_nothing() {
    let rig = LaneRig::new(vec![("default", passes(3, &["FS-01"]))]);
    rig.stop.store(true, Ordering::SeqCst);
    rig.run(&[lane(1, "default")], &ids(&["FS-01"]), 3, 0, 3);
    assert!(rig.calls.lock().unwrap().is_empty());
    assert!(rig.records().is_empty());
}

#[test]
fn a_panicking_lane_leaves_a_death_record_and_the_others_finish() {
    let mut rig = LaneRig::new(vec![("default", passes(2, &["FS-01"]))]);
    rig.panic_instance = Some("p2".to_string());
    rig.run(&lanes2(), &ids(&["FS-01"]), 2, 0, 3);
    let records = rig.records();
    let death: Vec<_> = records.iter().filter(|r| r.instance == "p2").collect();
    assert_eq!(death.len(), 1);
    let d = death[0];
    assert_eq!((d.index, d.ok, d.braked, d.seconds), (0, false, true, 0));
    assert_eq!(d.error.as_deref(), Some("lane: boom"));
    assert_eq!(d.verdict.as_ref().unwrap().reason.as_deref(), Some("boom"));
    assert_eq!(
        records
            .iter()
            .filter(|r| r.instance == "default" && r.ok)
            .count(),
        2
    );
}

#[test]
fn two_lanes_append_whole_lines_to_one_file() {
    // Weak on its own: one appended write is rarely torn. It still fails if a
    // lane writes a partial line or the records go to different files.
    let rig = LaneRig::new(vec![
        ("default", passes(10, &["FS-01"])),
        ("p2", passes(10, &["FS-01"])),
    ]);
    rig.run(&lanes2(), &ids(&["FS-01"]), 10, 0, 3);
    let text = std::fs::read_to_string(rig.jsonl()).unwrap();
    assert_eq!(text.lines().count(), 20);
    assert_eq!(rig.records().len(), 20);
    let default: Vec<u32> = rig
        .records()
        .iter()
        .filter(|r| r.instance == "default")
        .map(|r| r.index)
        .collect();
    assert_eq!(default, (1..=10).collect::<Vec<u32>>());
}

#[test]
fn a_stop_ends_the_stagger_sleep_at_once() {
    let stop = std::sync::atomic::AtomicBool::new(true);
    let t = Instant::now();
    sleep_unless_stopped(Duration::from_secs(60), &stop);
    assert!(t.elapsed() < Duration::from_secs(2));
}

#[test]
fn the_stagger_sleep_lasts_its_duration_when_not_stopped() {
    let stop = std::sync::atomic::AtomicBool::new(false);
    let t = Instant::now();
    sleep_unless_stopped(Duration::from_millis(60), &stop);
    assert!(t.elapsed() >= Duration::from_millis(60));
}
