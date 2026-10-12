//! The text frame and the JSON line. Ports the render checks of
//! tools/lab/cli/test-watch.ps1 (each check is named in a comment).

mod watch_support;

use std::fs;
use std::time::SystemTime;

use cimmeria_lab_cli::batch::{append_record, write_plan, Plan, PlanLane, RunRecord, Verdict};
use cimmeria_lab_cli::commands::watch::json::{error_line, render_json};
use cimmeria_lab_cli::commands::watch::snapshot::{
    BatchState, BatchView, InstanceView, LastFailure, LogCounts, Snapshot,
};
use cimmeria_lab_cli::commands::watch::text::render_text;
use serde_json::{json, Value};
use tempfile::TempDir;
use watch_support::{
    alive, batch_with, instance, log_lines, now, old_daemon, snap, temp_fixtures, two_lanes,
    NEW_BATCH, OLD_BATCH,
};

const SGW: [u32; 2] = [49448, 63904];

/// The two-lane snapshot every frame test reads (the PowerShell `$snap`).
fn two_lane() -> (TempDir, Snapshot) {
    let tmp = temp_fixtures();
    let status = two_lanes(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &log_lines(tmp.path()),
        &SGW,
        &alive,
    );
    (tmp, s)
}

fn lease_id() -> &'static str {
    "0123456789abcdef"
}

// 'no lease id in the snapshot, text or JSON'
#[test]
fn no_lease_id_reaches_the_snapshot_text_or_json() {
    let (_tmp, s) = two_lane();
    let everything = format!(
        "{s:?}{}{}",
        render_text(&s, 120).join("\n"),
        render_json(&s)
    );
    assert!(!everything.contains(lease_id()));
    assert!(
        everything.contains("lease-<redacted>"),
        "the redaction is visible in the text"
    );
}

// New: a lease id in a record's error, its reason or the row under way is hidden
// where the snapshot is built (the fixtures carry one only in a lease purpose).
#[test]
fn a_lease_id_in_a_record_or_a_row_is_hidden_in_the_snapshot() {
    let tmp = temp_fixtures();
    let dir = batch_with(
        tmp.path(),
        "batch-lease",
        &RunRecord {
            lane: 2,
            instance: "p2".into(),
            index: 2,
            error: Some("lab_uat_run failed: lease-0123456789abcdef is gone".into()),
            seconds: 3,
            ..Default::default()
        },
    );
    let mut status = two_lanes(tmp.path());
    status["instances"][1]["uat"]["current_row"] = json!("FS-lease-0123456789abcdef");
    let s = snap(Some(&status), Some(&dir), &[], &[], &alive);
    let failure = s.batch.as_ref().unwrap().last_failure.as_ref().unwrap();
    assert_eq!(
        failure.why,
        "error: lab_uat_run failed: lease-<redacted> is gone"
    );
    let p2 = instance(&s, "p2");
    assert_eq!(p2.row, "FS-lease-<redacted>");
    assert!(
        p2.last.as_ref().unwrap().contains("lease-<redacted>"),
        "{:?}",
        p2.last
    );
    assert!(!format!("{s:?}").contains(lease_id()));
}

// New: the renderers hide a lease id on their own, whatever the snapshot holds.
#[test]
fn the_renderers_hide_a_lease_id_in_every_string() {
    let id = "lease-0123456789abcdef";
    let leaky = format!("x {id} y");
    let s = Snapshot {
        daemon_up: true,
        old_daemon: false,
        batch: Some(BatchView {
            name: format!("batch-{id}"),
            state: BatchState::Live,
            section: Some(leaky.clone()),
            runs: 1,
            total: Some(2),
            passed: 0,
            failed: 1,
            known: 0,
            last_failure: Some(LastFailure {
                run: format!("{id}#1"),
                why: leaky.clone(),
                issue: Some(leaky.clone()),
            }),
        }),
        instances: vec![InstanceView {
            instance: format!("i-{id}"),
            client_pid: Some(7),
            lease: Some(leaky.clone()),
            run: "1/2".into(),
            row: leaky.clone(),
            row_s: Some(4),
            shot_s: Some(5),
            last: Some(leaky),
        }],
        orphans: vec![],
        logs: Some(LogCounts {
            relaunches: vec![(format!("r-{id}"), 1)],
            login_retries: 1,
        }),
    };
    for text in [render_text(&s, 400).join("\n"), render_json(&s)] {
        assert!(!text.contains("0123456789abcdef"), "{text}");
        assert!(text.contains("lease-<redacted>"), "{text}");
    }
}

// 'the old-daemon footer'
#[test]
fn the_old_daemon_footer() {
    let tmp = temp_fixtures();
    let status = old_daemon(tmp.path());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    assert!(render_text(&s, 120)
        .contains(&"daemon has no run progress: lab install, then lab restart".to_string()));
}

// 'the JSON lane string carries the shot age'
#[test]
fn the_json_lane_string_carries_the_shot_age() {
    let tmp = temp_fixtures();
    let run = TempDir::new().unwrap();
    let png = run
        .path()
        .join("rows")
        .join("s")
        .join("A")
        .join("final.png");
    fs::create_dir_all(png.parent().unwrap()).unwrap();
    fs::write(&png, [1u8]).unwrap();
    let file = fs::File::options().write(true).open(&png).unwrap();
    file.set_modified(SystemTime::from(now() - chrono::Duration::seconds(5)))
        .unwrap();
    let mut status = two_lanes(tmp.path());
    status["instances"][0]["uat"]["run_dir"] = json!(run.path().to_str().unwrap());
    let s = snap(
        Some(&status),
        Some(&tmp.path().join(NEW_BATCH)),
        &[],
        &[],
        &alive,
    );
    let v: Value = serde_json::from_str(&render_json(&s)).unwrap();
    assert_eq!(v["lanes"]["default"], "3/3 FS-P3 42s shot 5s");
}

// 'two-lane JSON under 500 characters with no null'
#[test]
fn two_lane_json_is_under_500_characters_with_no_null() {
    let (_tmp, s) = two_lane();
    let line = render_json(&s);
    assert!(
        line.len() < 500 && !line.contains("null"),
        "{} {line}",
        line.len()
    );
}

// 'two-lane JSON fields'
#[test]
fn two_lane_json_fields() {
    let (_tmp, s) = two_lane();
    let v: Value = serde_json::from_str(&render_json(&s)).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["state"], "live");
    assert_eq!(v["runs"], "3/6");
    assert_eq!(v["known"], 2);
    assert_eq!(v["lanes"]["default"], "3/3 FS-P3 42s");
    assert_eq!(v["lanes"]["p2"], "2/3 FS-02 3s");
    assert_eq!(v["orphans"], json!([63904]));
    assert_eq!(
        (v["relaunches"].as_u64(), v["login_retries"].as_u64()),
        (Some(1), Some(1))
    );
}

// 'JSON keys are in contract order'
#[test]
fn json_keys_are_in_contract_order() {
    let (_tmp, s) = two_lane();
    let line = render_json(&s);
    assert!(
        line.starts_with("{\"ok\":true,\"daemon\":\"up\",\"batch\":\"batch-20261011-013906\",\"state\":\"live\",\"section\":\"first-session\",\"runs\":\"3/6\",\"passed\":1,\"failed\":2,\"known\":2,\"last_failure\":"),
        "{line}"
    );
    // The whole line, so a reordered or renamed key fails here.
    assert_eq!(
        line,
        "{\"ok\":true,\"daemon\":\"up\",\"batch\":\"batch-20261011-013906\",\"state\":\"live\",\"section\":\"first-session\",\"runs\":\"3/6\",\"passed\":1,\"failed\":2,\"known\":2,\"last_failure\":{\"run\":\"default#2\",\"why\":\"FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch...\",\"issue\":\"#1343\"},\"lanes\":{\"default\":\"3/3 FS-P3 42s\",\"p2\":\"2/3 FS-02 3s\"},\"orphans\":[63904],\"relaunches\":1,\"login_retries\":1}"
    );
}

// 'daemon down with no batch'
#[test]
fn daemon_down_with_no_batch() {
    let s = snap(None, None, &[], &[], &alive);
    assert_eq!(
        render_json(&s),
        "{\"ok\":true,\"daemon\":\"down\",\"state\":\"none\"}"
    );
}

// New: the usage-error object (lab uat's shape).
#[test]
fn the_error_line_has_lab_uats_shape() {
    assert_eq!(
        error_line(2, "no batch folder: C:\\x"),
        "{\"ok\":false,\"exit\":2,\"error\":\"no batch folder: C:\\\\x\"}"
    );
}

/// The worst five-lane case: 20/20 runs each, a 600-character reason, 5 orphans,
/// a relaunch per instance (the PowerShell 'worst' fixture, built in code).
fn worst_case() -> Snapshot {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("batch-20261011-090000");
    fs::create_dir(&dir).unwrap();
    let names = ["default", "p2", "p3", "p4", "p5"];
    write_plan(
        &dir.join("plan.json"),
        &Plan {
            schema: 1,
            section: "first-session".into(),
            rows: vec!["FS-01".into()],
            lanes: names
                .iter()
                .enumerate()
                .map(|(k, n)| PlanLane {
                    lane: k as u32 + 1,
                    instance: (*n).into(),
                })
                .collect(),
            runs_per_lease: 20,
            started: "2026-10-11T01:00:00.0000000Z".into(),
            pid: Some(1),
        },
    )
    .unwrap();
    let long = "x".repeat(600);
    for n in names {
        for k in 1..=20 {
            append_record(
                &dir.join("batch.jsonl"),
                &RunRecord {
                    lane: 1,
                    instance: n.into(),
                    index: k,
                    ok: false,
                    seconds: 99,
                    verdict: Some(Verdict {
                        ok: false,
                        row: Some("FS-P2".into()),
                        result: Some("FAIL".into()),
                        reason: Some(format!("movie-over {long}")),
                        issue: None,
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        }
    }
    let run = TempDir::new().unwrap();
    let png = run
        .path()
        .join("rows")
        .join("s")
        .join("A")
        .join("final.png");
    fs::create_dir_all(png.parent().unwrap()).unwrap();
    fs::write(&png, [1u8]).unwrap();
    let file = fs::File::options().write(true).open(&png).unwrap();
    file.set_modified(SystemTime::from(now() - chrono::Duration::seconds(5)))
        .unwrap();
    let instances: Vec<Value> = names
        .iter()
        .enumerate()
        .map(|(k, n)| {
            json!({
                "instance": n, "client_pid": 50000 + k,
                "lease": {"held": true, "lease": {"owner": "lab_uat_run", "purpose": long}},
                "uat": {"run_dir": run.path().to_str().unwrap(), "current_row": "FS-P5", "row_started_at": "2026-10-11T01:30:00.000Z"}
            })
        })
        .collect();
    let status = json!({"daemon": {"pid": 1, "version": "0"}, "instances": instances});
    let log: Vec<String> = names
        .iter()
        .map(|n| format!("2026-10-11T01:40:00.000000Z  INFO lab_watchdog{{instance=\"{n}\" pid=1}}: relaunched after crash; logging back in new_pid=2"))
        .collect();
    snap(Some(&status), Some(&dir), &log, &[1, 2, 3, 4, 5], &alive)
}

// 'the worst five-lane JSON is under 700 characters'
#[test]
fn the_worst_five_lane_json_is_under_700_characters() {
    let line = render_json(&worst_case());
    assert!(line.len() < 700, "{} {line}", line.len());
}

// 'worst case: 3 orphans shown, 5 relaunches, 100/100 runs'
#[test]
fn worst_case_shows_3_orphans_5_relaunches_and_100_of_100_runs() {
    let v: Value = serde_json::from_str(&render_json(&worst_case())).unwrap();
    assert_eq!(v["orphans"].as_array().unwrap().len(), 3);
    assert_eq!(v["relaunches"], 5);
    assert_eq!(v["runs"], "100/100");
}

// 'the worst-case text fits 120 columns'
#[test]
fn the_worst_case_text_fits_120_columns() {
    let text = render_text(&worst_case(), 120);
    assert!(text.iter().all(|l| l.chars().count() <= 120));
    assert!(text
        .iter()
        .any(|l| l.contains("orphan SGW.exe: 1, 2, 3 +2 (#1342?)")));
}

// 'every text line fits 120 columns'
#[test]
fn every_text_line_fits_120_columns() {
    let (_tmp, s) = two_lane();
    assert!(render_text(&s, 120)
        .iter()
        .all(|l| l.chars().count() <= 120));
}

// 'the INSTANCE header and a free p3 row'
#[test]
fn the_instance_header_and_a_free_p3_row() {
    let (_tmp, s) = two_lane();
    let text = render_text(&s, 120);
    assert_eq!(
        text.iter().filter(|l| l.starts_with("INSTANCE ")).count(),
        1
    );
    assert_eq!(
        text.iter()
            .filter(|l| l.starts_with("p3") && l.contains(" free "))
            .count(),
        1
    );
}

// 'the orphan footer'
#[test]
fn the_orphan_footer() {
    let (_tmp, s) = two_lane();
    assert_eq!(
        render_text(&s, 120)
            .iter()
            .filter(|l| l.contains("orphan SGW.exe: 63904 (#1342?)"))
            .count(),
        1
    );
}

// 'the header line'
#[test]
fn the_header_line() {
    let (_tmp, s) = two_lane();
    assert_eq!(
        render_text(&s, 120)[0],
        "batch-20261011-013906  first-session  live  runs 3/6  passed 1  failed 2 (2 known)"
    );
}

// 'the default row shows FS-P3 42s'
#[test]
fn the_default_row_shows_fs_p3_42s() {
    let (_tmp, s) = two_lane();
    assert_eq!(
        render_text(&s, 120)
            .iter()
            .filter(|l| l.contains("FS-P3 42s"))
            .count(),
        1
    );
}

// 'LAST is whole on its own line under its row'
#[test]
fn last_is_whole_on_its_own_line_under_its_row() {
    let (_tmp, s) = two_lane();
    let text = render_text(&s, 120);
    let i = text
        .iter()
        .position(|l| l.starts_with("p2 "))
        .expect("the p2 row");
    assert!(
        text[i + 1].starts_with("    [p2 1/3] FAIL FS-P2: clause movie-over failed"),
        "{}",
        text[i + 1]
    );
}

// 'no instance row also carries LAST'
#[test]
fn no_instance_row_also_carries_last() {
    let (_tmp, s) = two_lane();
    let rows_with_last = render_text(&s, 120)
        .into_iter()
        .filter(|l| {
            (l.starts_with("default ") || l.starts_with("p2 "))
                && (l.contains("[default ") || l.contains("[p2 "))
        })
        .count();
    assert_eq!(rows_with_last, 0);
}

// New: the whole frame, column by column (the widths are the longest cell plus 2).
#[test]
fn the_two_lane_frame_is_exact() {
    let (_tmp, s) = two_lane();
    let text = render_text(&s, 120);
    let row = |a: &str, b: &str, c: &str, d: &str, e: &str, f: &str| {
        format!("{a:<10}{b:<8}{c:<59}{d:<5}{e:<11}{f}")
    };
    let lease_default = "lab_uat_run (UAT run, sections first-session, rows FS...)";
    let lease_p2 = "lab_uat_run (UAT run lease-<redacted>, sections first...)";
    assert_eq!(
        text[0],
        "batch-20261011-013906  first-session  live  runs 3/6  passed 1  failed 2 (2 known)"
    );
    assert_eq!(
        text[1],
        "last failure  default#2  FS-01 BLOCKED: setup failed: lab_login: Login_PasswordEdit holds 0 ch...  [#1343]"
    );
    assert_eq!(text[2], "");
    assert_eq!(
        text[3],
        row("INSTANCE", "CLIENT", "LEASE", "RUN", "ROW", "SHOT")
    );
    assert_eq!(
        text[4],
        row("default", "49448", lease_default, "3/3", "FS-P3 42s", "-")
    );
    assert!(text[5].starts_with("    [default 2/3] FAIL FS-01: setup failed: lab_login: Login_PasswordEdit holds 0 characters"));
    assert!(
        text[5].ends_with("...") && text[5].chars().count() == 120,
        "{}",
        text[5]
    );
    assert_eq!(
        text[6],
        row("p2", "51200", lease_p2, "2/3", "FS-02 3s", "-")
    );
    assert_eq!(
        text[7],
        "    [p2 1/3] FAIL FS-P2: clause movie-over failed: client_entity_find/count gte 1 [known #1341] (130s)"
    );
    assert_eq!(text[8], row("p3", "-", "free", "-", "-", "-"));
    assert_eq!(text[9], "");
    assert_eq!(
        text[10],
        "orphan SGW.exe: 63904 (#1342?)  relaunches: p2 1  login retries: 1 (#1343)"
    );
    assert_eq!(text.len(), 11);
}

// New: a frame is cut to the width it is given, with three dots.
#[test]
fn a_narrow_width_cuts_lines_with_three_dots() {
    let (_tmp, s) = two_lane();
    let text = render_text(&s, 50);
    assert!(text.iter().all(|l| l.chars().count() <= 50));
    assert!(text[1].ends_with("..."));
}

// New: the no-batch frames and the daemon-down footer.
#[test]
fn the_no_batch_frames_and_the_daemon_down_footer() {
    assert_eq!(
        render_text(&snap(None, None, &[], &[], &alive), 120)[0],
        "no batch  daemon down"
    );
    let tmp = temp_fixtures();
    let up = two_lanes(tmp.path());
    assert_eq!(
        render_text(&snap(Some(&up), None, &[], &[], &alive), 120)[0],
        "no batch"
    );
    let down = snap(None, Some(&tmp.path().join(NEW_BATCH)), &[], &[], &alive);
    assert_eq!(
        render_text(&down, 120).last().unwrap(),
        "daemon down: showing the batch files only"
    );
}

/// The pre-LW-02 batch (no plan.json) read against the two-lane status: its
/// batch.json removed, so its state is unknown.
fn pre_plan_snapshot() -> Snapshot {
    let tmp = temp_fixtures();
    fs::remove_file(tmp.path().join(OLD_BATCH).join("batch.json")).unwrap();
    let status = two_lanes(tmp.path());
    snap(
        Some(&status),
        Some(&tmp.path().join(OLD_BATCH)),
        &[],
        &[],
        &alive,
    )
}

// 'the pre-LW-02 records aggregate under strict mode'
#[test]
fn the_pre_plan_records_aggregate() {
    let s = pre_plan_snapshot();
    let batch = s.batch.as_ref().unwrap();
    assert_eq!(batch.passed, 2);
    assert_eq!(batch.state.as_str(), "unknown");
    let v: Value = serde_json::from_str(&render_json(&s)).unwrap();
    assert_eq!(v["runs"], "2");
    let _ = render_text(&s, 120);
}

// 'without a plan LAST reads <i>/?'
#[test]
fn without_a_plan_last_reads_i_over_a_question_mark() {
    let s = pre_plan_snapshot();
    let lasts: Vec<&String> = s.instances.iter().filter_map(|i| i.last.as_ref()).collect();
    assert!(!lasts.is_empty());
    assert!(
        lasts
            .iter()
            .all(|l| l.starts_with("[default 1/?] ") || l.starts_with("[p2 1/?] ")),
        "{lasts:?}"
    );
}
