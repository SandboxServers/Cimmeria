//! The plan of a batch: which instances run, what the plan-only call asks and
//! how its reply is judged, and the `plan.json` of the batch folder.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::batch::{write_plan, Plan, PlanLane, PLAN_JSON};
use crate::daemon::mcp::hide_lease_ids;

use super::args::{MAX_LEASES, MAX_RUNS_PER_LEASE};

/// A usage or pre-flight failure: the exit code and the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub code: u8,
    pub message: String,
}

impl Stop {
    pub fn new(code: u8, message: impl Into<String>) -> Self {
        Stop {
            code,
            message: message.into(),
        }
    }
}

/// The first line of a message, trailing whitespace cut.
pub fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or("").trim_end().to_string()
}

/// Picks the instances the lanes run on (`Get-UatPlan`). `hosted` is the
/// daemon's instance labels in order, `leased` the labels a session holds,
/// `max_clients` the daemon's `CIMMERIA_LAB_MAX_CLIENTS` (0 for none). Lanes
/// are numbered from 1. `Err` is exit 2 for an unknown `--instance`, else 3.
pub fn pick_lanes(
    leases: u32,
    runs_per_lease: u32,
    hosted: &[String],
    leased: &[String],
    max_clients: u32,
    instance: Option<&str>,
) -> Result<Vec<PlanLane>, Stop> {
    if !(1..=MAX_LEASES).contains(&leases) {
        return Err(Stop::new(3, format!("--leases must be 1 to {MAX_LEASES}")));
    }
    if !(1..=MAX_RUNS_PER_LEASE).contains(&runs_per_lease) {
        return Err(Stop::new(
            3,
            format!("--runs-per-lease must be 1 to {MAX_RUNS_PER_LEASE}"),
        ));
    }
    let hosted: Vec<&String> = hosted.iter().filter(|h| !h.is_empty()).collect();
    let leased: Vec<&String> = leased.iter().filter(|h| !h.is_empty()).collect();
    if let Some(instance) = instance.filter(|i| !i.is_empty()) {
        if leases != 1 {
            return Err(Stop::new(
                3,
                "--instance runs one lane; drop --leases or --instance",
            ));
        }
        if !hosted.iter().any(|h| h.as_str() == instance) {
            let names: Vec<&str> = hosted.iter().map(|h| h.as_str()).collect();
            return Err(Stop::new(
                2,
                format!(
                    "unknown instance '{instance}'; the daemon hosts: {}",
                    names.join(", ")
                ),
            ));
        }
        if leased.iter().any(|l| l.as_str() == instance) {
            return Err(Stop::new(
                3,
                format!(
                    "instance {instance} is leased by another session; wait for it or pick another"
                ),
            ));
        }
        return Ok(vec![PlanLane {
            lane: 1,
            instance: instance.to_string(),
        }]);
    }
    let cap = if max_clients > 0 {
        hosted.len().min(max_clients as usize)
    } else {
        hosted.len()
    };
    if leases as usize > cap {
        return Err(Stop::new(
            3,
            format!(
                "--leases {leases} is more than the lab allows ({cap}: {} hosted, max clients {max_clients})",
                hosted.len()
            ),
        ));
    }
    let free: Vec<&String> = hosted
        .iter()
        .copied()
        .filter(|h| !leased.contains(h))
        .collect();
    if free.len() < leases as usize {
        let busy: Vec<&str> = hosted
            .iter()
            .filter(|h| leased.contains(*h))
            .map(|h| h.as_str())
            .collect();
        return Err(Stop::new(
            3,
            format!(
                "only {} free instance(s) for {leases} lease(s); leased by other sessions: {}",
                free.len(),
                busy.join(", ")
            ),
        ));
    }
    Ok(free
        .iter()
        .take(leases as usize)
        .enumerate()
        .map(|(i, name)| PlanLane {
            lane: i as u32 + 1,
            instance: (*name).clone(),
        })
        .collect())
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// The arguments of one `lab_uat_run` call. Keys the PowerShell leaves out are
/// left out here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunCall {
    pub sections: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixtures: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chaos: Vec<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub fingerprint: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub plan_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

impl RunCall {
    /// The arguments every call of the batch shares (`$baseArgs`). `chaos` is
    /// already filtered (`args::chaos_specs`); `fingerprint` is
    /// `--fingerprint` or `--diff-golden` (and `--chaos` implies the latter).
    pub fn new(
        section: &str,
        rows: &[String],
        specs_dir: Option<&str>,
        server_version: Option<&str>,
        fixtures: Option<&str>,
        chaos: &[String],
        fingerprint: bool,
    ) -> Self {
        let some = |v: Option<&str>| v.filter(|s| !s.is_empty()).map(str::to_string);
        RunCall {
            sections: vec![section.to_string()],
            rows: rows.to_vec(),
            specs_dir: some(specs_dir),
            server_version: some(server_version),
            fixtures: some(fixtures),
            chaos: chaos.to_vec(),
            fingerprint,
            plan_only: false,
            instance: None,
        }
    }

    /// The plan call: `plan_only` on the first lane's instance.
    pub fn for_plan(&self, instance: &str) -> RunCall {
        RunCall {
            plan_only: true,
            instance: Some(instance.to_string()),
            ..self.clone()
        }
    }

    /// One run on a lane (LX-16 uses it): the instance set, `plan_only` off.
    pub fn for_lane(&self, instance: &str) -> RunCall {
        RunCall {
            plan_only: false,
            instance: Some(instance.to_string()),
            ..self.clone()
        }
    }

    /// The call as the JSON the daemon receives.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// What a ready plan reply says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The rows the runner will run, in its order.
    pub rows: Vec<String>,
    /// The runner's own chaos header (`Get-UatRunChaos`), when the reply has one.
    pub chaos: Option<String>,
}

/// The runner's `chaos` list joined with `"; "`; `None` when the reply has none
/// (or only empty entries).
pub fn run_chaos(reply: &Value) -> Option<String> {
    let headers: Vec<&str> = reply
        .get("chaos")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if headers.is_empty() {
        None
    } else {
        Some(headers.join("; "))
    }
}

/// Judges the plan-only reply: a request for a row the section lacks is exit 2,
/// a row that cannot run is exit 3.
pub fn judge_plan(reply: &Value, requested: &[String], section: &str) -> Result<Planned, Stop> {
    let chaos = run_chaos(reply);
    let rows: Vec<&Value> = reply
        .get("rows")
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let text = |row: &Value, key: &str| -> String {
        row.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let ids: Vec<String> = rows.iter().map(|r| text(r, "row")).collect();
    if ids.is_empty() {
        return Err(Stop::new(
            2,
            format!("no rows matched in section {section}"),
        ));
    }
    let missing: Vec<&str> = requested
        .iter()
        .filter(|r| !ids.contains(*r))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        return Err(Stop::new(
            2,
            format!(
                "section {section} has no row(s) {} (row ids are exact and case sensitive)",
                missing.join(", ")
            ),
        ));
    }
    let not_ready: Vec<String> = rows
        .iter()
        .filter(|r| text(r, "result") != "SKIPPED")
        .map(|r| {
            let reason = r
                .get("reasons")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            format!(
                "{} {}: {}",
                text(r, "row"),
                text(r, "result"),
                hide_lease_ids(reason)
            )
        })
        .collect();
    if !not_ready.is_empty() {
        return Err(Stop::new(3, format!("not ready: {}", not_ready.join("; "))));
    }
    Ok(Planned { rows: ids, chaos })
}

/// `plan.json`'s `started`: UTC, round-trip format with seven fractional digits.
pub fn started_text(at: DateTime<Utc>) -> String {
    format!(
        "{}.{:07}Z",
        at.format("%Y-%m-%dT%H:%M:%S"),
        at.timestamp_subsec_nanos() / 100
    )
}

/// The batch folder's name from the local start time: `batch-yyyyMMdd-HHmmss`.
pub fn batch_folder_name(local: &NaiveDateTime) -> String {
    format!("batch-{}", local.format("%Y%m%d-%H%M%S"))
}

/// The `plan.json` of a batch (schema 1).
pub fn build_plan(
    section: &str,
    rows: &[String],
    lanes: &[PlanLane],
    runs_per_lease: u32,
    started: DateTime<Utc>,
    pid: u32,
) -> Plan {
    Plan {
        schema: 1,
        section: section.to_string(),
        rows: rows.to_vec(),
        lanes: lanes.to_vec(),
        runs_per_lease,
        started: started_text(started),
        pid: Some(pid),
    }
}

/// Creates `<root>\<folder>` and writes `plan.json` into it before any lane
/// starts, so a watcher never sees runs without a plan. `Err` is exit 3.
pub fn create_batch(root: &Path, folder: &str, plan: &Plan) -> Result<PathBuf, Stop> {
    let dir = root.join(folder);
    fs::create_dir_all(&dir)
        .and_then(|_| write_plan(&dir.join(PLAN_JSON), plan))
        .map_err(|e| {
            Stop::new(
                3,
                format!(
                    "cannot write the batch folder {}: {}",
                    dir.display(),
                    first_line(&e.to_string())
                ),
            )
        })?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
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
}
