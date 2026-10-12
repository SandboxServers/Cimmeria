//! plan.json, batch.jsonl and batch folders. Ports the 'finding and reading a
//! batch' checks of the removed PowerShell test file (each check is named in a
//! comment) and pins the file formats LX-15 to LX-17 write.

mod watch_support;

use std::fs;
use std::time::Duration;

use cimmeria_lab_cli::batch::{
    append_record, find_latest_batch, read_plan, read_records, write_plan, Plan, PlanLane,
    RowResult, RunRecord, Verdict,
};
use tempfile::TempDir;
use watch_support::{copy_dir, fixture_dir, NEW_BATCH, OLD_BATCH};

// 'the newest batch is picked by name, not by file time'
#[test]
fn the_newest_batch_is_picked_by_name_not_by_file_time() {
    let root = TempDir::new().unwrap();
    // The old batch is created last, so a sort by file time would pick it.
    copy_dir(&fixture_dir().join(NEW_BATCH), &root.path().join(NEW_BATCH));
    std::thread::sleep(Duration::from_millis(30));
    copy_dir(&fixture_dir().join(OLD_BATCH), &root.path().join(OLD_BATCH));
    fs::create_dir(root.path().join("notes")).unwrap();
    fs::write(
        root.path().join("batch-29990101-000000"),
        "a file, not a batch",
    )
    .unwrap();
    let found = find_latest_batch(root.path()).unwrap().unwrap();
    assert_eq!(found.file_name().unwrap().to_string_lossy(), NEW_BATCH);
}

// 'no UAT root, no batch'
#[test]
fn no_uat_root_means_no_batch() {
    let root = TempDir::new().unwrap();
    assert_eq!(
        find_latest_batch(&root.path().join("nowhere")).unwrap(),
        None
    );
    assert_eq!(find_latest_batch(root.path()).unwrap(), None);
}

// 'the half-written last line is skipped'
#[test]
fn a_half_written_last_line_is_skipped() {
    let records = read_records(&fixture_dir().join(NEW_BATCH).join("batch.jsonl")).unwrap();
    assert_eq!(records.len(), 3);
}

// New: the fields of a real record, as the removed PowerShell uat command wrote them.
#[test]
fn record_fields_are_read_as_written() {
    let records = read_records(&fixture_dir().join(NEW_BATCH).join("batch.jsonl")).unwrap();
    let r = &records[1];
    assert_eq!(
        (r.lane, r.instance.as_str(), r.index, r.ok),
        (2, "p2", 1, false)
    );
    assert_eq!((r.seconds, r.braked), (130, false));
    assert_eq!(r.rows.len(), 7);
    assert_eq!(r.rows[3].row, "FS-P2");
    assert_eq!(r.rows[3].result, "FAIL");
    assert_eq!(r.rows[3].section.as_deref(), Some("first-session"));
    assert_eq!(
        r.rows[3].reasons,
        vec!["clause movie-over failed: client_entity_find/count gte 1"]
    );
    let v = r.verdict.as_ref().unwrap();
    assert_eq!(v.row.as_deref(), Some("FS-P2"));
    assert_eq!(v.result.as_deref(), Some("FAIL"));
    assert_eq!(v.issue.as_deref(), Some("#1341"));
    assert_eq!(
        r.run_dir.as_deref(),
        Some("C:\\lab\\uat-runs\\2026-10-11-unknown-cfzzhpbb")
    );
    assert_eq!(r.error, None);
    assert_eq!(r.started.as_deref(), Some("2026-10-11T01:39:27.0381952Z"));
    assert_eq!(r.ended.as_deref(), Some("2026-10-11T01:41:37.2209135Z"));
}

// New: the pre-plan batch's records have no Started or Ended.
#[test]
fn an_old_record_without_started_and_ended_reads() {
    let records = read_records(&fixture_dir().join(OLD_BATCH).join("batch.jsonl")).unwrap();
    assert_eq!(records.len(), 2);
    assert!(records[0].ok && records[0].started.is_none() && records[0].ended.is_none());
    assert_eq!(records[1].instance, "p2");
}

// New: plan.json as the removed PowerShell uat command wrote it.
#[test]
fn plan_json_fields_are_read_as_written() {
    let plan = read_plan(&fixture_dir().join(NEW_BATCH).join("plan.json")).unwrap();
    assert_eq!(plan.schema, 1);
    assert_eq!(plan.section, "first-session");
    assert_eq!(
        plan.rows,
        ["FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4", "FS-P5"]
    );
    assert_eq!(
        plan.lanes,
        vec![
            PlanLane {
                lane: 1,
                instance: "default".into()
            },
            PlanLane {
                lane: 2,
                instance: "p2".into()
            }
        ]
    );
    assert_eq!(plan.runs_per_lease, 3);
    assert_eq!(plan.started, "2026-10-11T01:39:06.0000000Z");
    assert_eq!(plan.pid, Some(41236));
}

// New: a missing or broken plan.json is None, never an error.
#[test]
fn a_missing_or_broken_plan_is_none() {
    let tmp = TempDir::new().unwrap();
    assert_eq!(read_plan(&tmp.path().join("plan.json")), None);
    fs::write(tmp.path().join("plan.json"), "not json {").unwrap();
    assert_eq!(read_plan(&tmp.path().join("plan.json")), None);
}

// New: no batch.jsonl is no records; an unreadable one is an error (exit 3 in the command).
#[test]
fn a_missing_batch_jsonl_is_no_records_and_an_unreadable_one_is_an_error() {
    let tmp = TempDir::new().unwrap();
    assert!(read_records(&tmp.path().join("batch.jsonl"))
        .unwrap()
        .is_empty());
    fs::create_dir(tmp.path().join("batch.jsonl")).unwrap();
    assert!(read_records(&tmp.path().join("batch.jsonl")).is_err());
}

// New: the key order of a record line is the order the removed PowerShell uat command's hashtable wrote.
#[test]
fn a_run_record_serializes_in_the_order_powershell_wrote_it() {
    let mut r = RunRecord {
        lane: 1,
        instance: "default".into(),
        index: 1,
        ok: true,
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        "{\"Lane\":1,\"Instance\":\"default\",\"Index\":1,\"Ok\":true,\"Rows\":[],\"Verdict\":null,\"RunDir\":null,\"Error\":null,\"Seconds\":0,\"Braked\":false,\"Started\":null,\"Ended\":null}"
    );
    r.chaos = Some("chaos=cpu seed=1".into());
    r.golden = Some("match".into());
    r.verdict = Some(Verdict {
        ok: true,
        ..Default::default()
    });
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        "{\"Lane\":1,\"Instance\":\"default\",\"Index\":1,\"Ok\":true,\"Rows\":[],\"Verdict\":{\"Ok\":true,\"Row\":null,\"Result\":null,\"Reason\":null,\"Issue\":null},\"RunDir\":null,\"Error\":null,\"Seconds\":0,\"Braked\":false,\"Started\":null,\"Ended\":null,\"Chaos\":\"chaos=cpu seed=1\",\"Golden\":\"match\"}"
    );
}

// New: the key order of plan.json.
#[test]
fn a_plan_serializes_in_the_order_powershell_wrote_it() {
    let plan = Plan {
        schema: 1,
        section: "first-session".into(),
        rows: vec!["FS-01".into()],
        lanes: vec![PlanLane {
            lane: 1,
            instance: "default".into(),
        }],
        runs_per_lease: 3,
        started: "2026-10-11T01:39:06.0000000Z".into(),
        pid: Some(41236),
    };
    assert_eq!(
        serde_json::to_string(&plan).unwrap(),
        "{\"schema\":1,\"section\":\"first-session\",\"rows\":[\"FS-01\"],\"lanes\":[{\"lane\":1,\"instance\":\"default\"}],\"runs_per_lease\":3,\"started\":\"2026-10-11T01:39:06.0000000Z\",\"pid\":41236}"
    );
}

// New: what the writers write, the readers read.
#[test]
fn write_plan_and_append_record_round_trip() {
    let tmp = TempDir::new().unwrap();
    let plan = read_plan(&fixture_dir().join(NEW_BATCH).join("plan.json")).unwrap();
    write_plan(&tmp.path().join("plan.json"), &plan).unwrap();
    assert_eq!(read_plan(&tmp.path().join("plan.json")), Some(plan));
    let jsonl = tmp.path().join("batch.jsonl");
    let records = read_records(&fixture_dir().join(NEW_BATCH).join("batch.jsonl")).unwrap();
    for r in &records {
        append_record(&jsonl, r).unwrap();
    }
    assert_eq!(read_records(&jsonl).unwrap(), records);
    assert_eq!(fs::read_to_string(&jsonl).unwrap().lines().count(), 3);
}

// New: a key the daemon adds to a row survives a round trip.
#[test]
fn an_unknown_row_field_survives_a_round_trip() {
    let row: RowResult = serde_json::from_str(
        "{\"row\":\"FS-01\",\"result\":\"PASS\",\"section\":\"s\",\"focus\":{\"a\":1},\"reasons\":null}",
    )
    .unwrap();
    assert!(row.reasons.is_empty());
    let text = serde_json::to_string(&row).unwrap();
    assert!(text.contains("\"focus\":{\"a\":1}"), "{text}");
    assert_eq!(serde_json::from_str::<RowResult>(&text).unwrap(), row);
}
