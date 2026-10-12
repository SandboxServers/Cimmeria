//! Unit tests of `plan.rs` (lane planning, the request body, the plan reply, `plan.json`).

use chrono::{NaiveDate, TimeZone};
use serde_json::json;

use super::*;
use crate::batch::read_plan;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn hosted() -> Vec<String> {
    strings(&["default", "p2", "p3", "p4", "p5"])
}

fn names(lanes: &[PlanLane]) -> Vec<&str> {
    lanes.iter().map(|l| l.instance.as_str()).collect()
}

// ---- lane planning (test-uat.ps1, "planning") ----

// 'one lease runs on the first instance'
#[test]
fn one_lease_runs_on_the_first_instance() {
    let lanes = pick_lanes(1, 1, &hosted(), &[], 0, None).unwrap();
    assert_eq!(
        lanes,
        [PlanLane {
            lane: 1,
            instance: "default".into()
        }]
    );
}

// 'lanes skip an instance another session leased'
#[test]
fn lanes_skip_a_leased_instance() {
    let lanes = pick_lanes(3, 20, &hosted(), &strings(&["p2"]), 0, None).unwrap();
    assert_eq!(names(&lanes), ["default", "p3", "p4"]);
    assert_eq!(lanes.iter().map(|l| l.lane).collect::<Vec<_>>(), [1, 2, 3]);
}

// '0 leases is refused', '6 leases is refused'
#[test]
fn leases_outside_1_to_5_are_refused() {
    for n in [0, 6] {
        let err = pick_lanes(n, 1, &hosted(), &[], 0, None).unwrap_err();
        assert_eq!(err, Stop::new(3, "--leases must be 1 to 5"), "{n}");
    }
}

// '21 runs per lease is refused'
#[test]
fn runs_per_lease_above_20_are_refused() {
    let err = pick_lanes(1, 21, &hosted(), &[], 0, None).unwrap_err();
    assert_eq!(err, Stop::new(3, "--runs-per-lease must be 1 to 20"));
}

// 'more leases than hosted instances is refused'
#[test]
fn more_leases_than_hosted_instances_is_refused() {
    let err = pick_lanes(3, 1, &strings(&["default", "p2"]), &[], 0, None).unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            3,
            "--leases 3 is more than the lab allows (2: 2 hosted, max clients 0)"
        )
    );
}

// 'the client cap limits leases'
#[test]
fn the_client_cap_limits_leases() {
    let err = pick_lanes(3, 1, &hosted(), &[], 2, None).unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            3,
            "--leases 3 is more than the lab allows (2: 5 hosted, max clients 2)"
        )
    );
}

// 'too few free instances is refused, naming the busy ones'
#[test]
fn too_few_free_instances_names_the_busy_ones() {
    let err = pick_lanes(
        2,
        1,
        &strings(&["default", "p2"]),
        &strings(&["p2"]),
        0,
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            3,
            "only 1 free instance(s) for 2 lease(s); leased by other sessions: p2"
        )
    );
}

// '-Instance picks that lane'
#[test]
fn instance_picks_that_lane() {
    let lanes = pick_lanes(1, 5, &hosted(), &[], 0, Some("p3")).unwrap();
    assert_eq!(
        lanes,
        [PlanLane {
            lane: 1,
            instance: "p3".into()
        }]
    );
}

// '-Instance with 2 leases is refused'
#[test]
fn instance_with_two_leases_is_refused() {
    let err = pick_lanes(2, 1, &hosted(), &[], 0, Some("p3")).unwrap_err();
    assert_eq!(
        err,
        Stop::new(3, "--instance runs one lane; drop --leases or --instance")
    );
}

// 'an unknown instance is refused' (exit 2, the one plan error that is a usage error)
#[test]
fn an_unknown_instance_is_exit_2() {
    let err = pick_lanes(1, 1, &hosted(), &[], 0, Some("p9")).unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            2,
            "unknown instance 'p9'; the daemon hosts: default, p2, p3, p4, p5"
        )
    );
}

// 'a leased -Instance is refused'
#[test]
fn a_leased_instance_is_refused() {
    let err = pick_lanes(1, 1, &hosted(), &strings(&["p3"]), 0, Some("p3")).unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            3,
            "instance p3 is leased by another session; wait for it or pick another"
        )
    );
}

// ---- the request body (test-uat-chaos.ps1: 'two -Chaos values both reach
// the request body', '-Chaos implies -Fingerprint', 'a quoted multi-knob
// spec reaches the body as one element', 'without -Chaos the body has no
// chaos or fingerprint key') ----

#[test]
fn a_bare_call_has_only_the_section() {
    let call = RunCall::new("first-session", &[], None, None, None, &[], false);
    assert_eq!(call.to_value(), json!({"sections": ["first-session"]}));
}

#[test]
fn the_plan_call_adds_plan_only_and_the_first_instance() {
    let call = RunCall::new("first-session", &[], None, None, None, &[], false);
    assert_eq!(
        call.for_plan("default").to_value(),
        json!({"sections": ["first-session"], "plan_only": true, "instance": "default"})
    );
    // a lane's call has the instance and no plan_only
    assert_eq!(
        call.for_lane("p2").to_value(),
        json!({"sections": ["first-session"], "instance": "p2"})
    );
}

#[test]
fn every_option_reaches_the_body() {
    let call = RunCall::new(
        "first-session",
        &strings(&["FS-01", "FS-02"]),
        Some("C:\\specs"),
        Some("abc1234"),
        Some("always"),
        &strings(&["cpu", "flush:pack", "net:latency=120,loss=1"]),
        true,
    );
    assert_eq!(
        call.for_plan("p2").to_value(),
        json!({
            "sections": ["first-session"],
            "rows": ["FS-01", "FS-02"],
            "specs_dir": "C:\\specs",
            "server_version": "abc1234",
            "fixtures": "always",
            "chaos": ["cpu", "flush:pack", "net:latency=120,loss=1"],
            "fingerprint": true,
            "plan_only": true,
            "instance": "p2"
        })
    );
}

// ---- the plan reply ----

fn ready_row(id: &str) -> Value {
    json!({"row": id, "result": "SKIPPED", "reasons": []})
}

// test-uat-chaos.ps1: "the runner's headers join with '; '"
#[test]
fn the_runners_headers_join_with_a_semicolon() {
    let reply = json!({"chaos": ["chaos=cpu seed=5", "chaos=flush:pack seed=5"]});
    assert_eq!(
        run_chaos(&reply),
        Some("chaos=cpu seed=5; chaos=flush:pack seed=5".to_string())
    );
}

// test-uat-chaos.ps1: 'a result without chaos gives no header'
#[test]
fn a_result_without_chaos_gives_no_header() {
    assert_eq!(run_chaos(&json!({"rows": []})), None);
    assert_eq!(run_chaos(&json!({"chaos": []})), None);
}

// uat.ps1: the plan is ready when every row is SKIPPED
#[test]
fn a_ready_plan_lists_its_rows_and_the_runners_header() {
    let reply = json!({"rows": [ready_row("FS-01"), ready_row("FS-02")], "chaos": ["chaos=cpu:safe seed=<per run>"]});
    let planned = judge_plan(&reply, &strings(&["FS-02"]), "first-session").unwrap();
    assert_eq!(planned.rows, ["FS-01", "FS-02"]);
    assert_eq!(
        planned.chaos.as_deref(),
        Some("chaos=cpu:safe seed=<per run>")
    );
}

// uat.ps1: "no rows matched in section $Section"
#[test]
fn no_rows_is_exit_2() {
    for reply in [json!({"rows": []}), json!({})] {
        let err = judge_plan(&reply, &[], "first-session").unwrap_err();
        assert_eq!(
            err,
            Stop::new(2, "no rows matched in section first-session")
        );
    }
}

// uat.ps1: 'section X has no row(s) ... (row ids are exact and case sensitive)'
#[test]
fn a_requested_row_the_section_lacks_is_exit_2_and_exact() {
    let reply = json!({"rows": [ready_row("FS-01")]});
    let err = judge_plan(
        &reply,
        &strings(&["FS-01", "FS-99", "fs-01"]),
        "first-session",
    )
    .unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            2,
            "section first-session has no row(s) FS-99, fs-01 (row ids are exact and case sensitive)"
        )
    );
}

// uat.ps1: 'not ready: <row> <result>: <first reason>', joined with '; '
#[test]
fn a_row_that_cannot_run_is_exit_3_with_its_first_reason() {
    let reply = json!({"rows": [
        {"row": "FS-01", "result": "BLOCKED", "reasons": ["missing tool x lease-0123456789abcdef", "second"]},
        ready_row("FS-02"),
        {"row": "FS-03", "result": "BLOCKED", "reasons": []}
    ]});
    let err = judge_plan(&reply, &[], "first-session").unwrap_err();
    assert_eq!(
        err,
        Stop::new(
            3,
            "not ready: FS-01 BLOCKED: missing tool x lease-<redacted>; FS-03 BLOCKED: "
        )
    );
}

// ---- plan.json (test-uat.ps1, Write-UatPlan / Read-UatPlan) ----

fn lanes2() -> Vec<PlanLane> {
    vec![
        PlanLane {
            lane: 1,
            instance: "default".into(),
        },
        PlanLane {
            lane: 2,
            instance: "p2".into(),
        },
    ]
}

fn started() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 11, 1, 39, 6).unwrap()
}

fn sample_plan() -> Plan {
    build_plan(
        "first-session",
        &strings(&["FS-01", "FS-P1"]),
        &lanes2(),
        3,
        started(),
        41236,
    )
}

// 'plan.json round-trips its section, two lanes and pid', 'plan.json keeps
// the rows and each lane'
#[test]
fn plan_json_round_trips() {
    let root = tempfile::tempdir().unwrap();
    let dir = create_batch(root.path(), "batch-20261011-013906", &sample_plan()).unwrap();
    let back = read_plan(&dir.join("plan.json")).unwrap();
    assert_eq!(back, sample_plan());
    assert_eq!(back.section, "first-session");
    assert_eq!(back.lanes.len(), 2);
    assert_eq!(back.pid, Some(41236));
    assert_eq!(back.rows, ["FS-01", "FS-P1"]);
    assert_eq!(back.lanes[1].instance, "p2");
    assert_eq!(back.runs_per_lease, 3);
}

// 'plan.json has no byte order mark'; and the keys are in the PowerShell's order
#[test]
fn plan_json_has_no_bom_and_keeps_the_key_order() {
    let root = tempfile::tempdir().unwrap();
    let dir = create_batch(root.path(), "batch-x", &sample_plan()).unwrap();
    let bytes = fs::read(dir.join("plan.json")).unwrap();
    assert_ne!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    let text = String::from_utf8(bytes).unwrap();
    let at = |key: &str| text.find(&format!("\"{key}\"")).unwrap();
    assert!(at("schema") < at("section"));
    assert!(at("section") < at("rows"));
    assert!(at("rows") < at("lanes"));
    assert!(at("lanes") < at("runs_per_lease"));
    assert!(at("runs_per_lease") < at("started"));
    assert!(at("started") < at("pid"));
    assert!(text.contains("\"started\": \"2026-10-11T01:39:06.0000000Z\""));
}

// 'a missing plan.json reads as nothing'
#[test]
fn a_missing_plan_reads_as_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(read_plan(&root.path().join("no-such-plan.json")), None);
}

// 'a plan.json that is not JSON reads as nothing'
#[test]
fn a_plan_that_is_not_json_reads_as_nothing() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("plan.json");
    fs::write(&path, "{").unwrap();
    assert_eq!(read_plan(&path), None);
}

// uat.ps1: "cannot write the batch folder ..." is exit 3
#[test]
fn an_unwritable_batch_folder_is_exit_3() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("a-file");
    fs::write(&file, "x").unwrap();
    let err = create_batch(&file, "batch-x", &sample_plan()).unwrap_err();
    assert_eq!(err.code, 3);
    assert!(
        err.message.starts_with("cannot write the batch folder "),
        "{}",
        err.message
    );
}

#[test]
fn started_is_utc_with_seven_fractional_digits() {
    assert_eq!(started_text(started()), "2026-10-11T01:39:06.0000000Z");
    let later = started() + chrono::Duration::nanoseconds(123_456_700);
    assert_eq!(started_text(later), "2026-10-11T01:39:06.1234567Z");
}

#[test]
fn the_batch_folder_is_named_by_local_start_time() {
    let at = NaiveDate::from_ymd_opt(2026, 10, 11)
        .unwrap()
        .and_hms_opt(1, 39, 6)
        .unwrap();
    assert_eq!(batch_folder_name(&at), "batch-20261011-013906");
}
