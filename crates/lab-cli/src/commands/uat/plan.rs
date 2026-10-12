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
    /// The process exit code: 2 for usage, 3 for a pre-flight failure.
    pub code: u8,
    /// What to print.
    pub message: String,
}

impl Stop {
    /// A stop with exit code and message.
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
    /// The sections to run (one).
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
/// starts, so a watcher never sees runs without a plan. A folder of that name
/// that already exists (two batches in the same second) is never reused: the
/// next free `<folder>-2`, `<folder>-3`, ... is made instead. `Err` is exit 3.
pub fn create_batch(root: &Path, folder: &str, plan: &Plan) -> Result<PathBuf, Stop> {
    let fail = |dir: &Path, e: std::io::Error| {
        Stop::new(
            3,
            format!(
                "cannot write the batch folder {}: {}",
                dir.display(),
                first_line(&e.to_string())
            ),
        )
    };
    fs::create_dir_all(root).map_err(|e| fail(root, e))?;
    let mut n = 1u32;
    let dir = loop {
        let dir = if n == 1 {
            root.join(folder)
        } else {
            root.join(format!("{folder}-{n}"))
        };
        match fs::create_dir(&dir) {
            Ok(()) => break dir,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < MAX_BATCH_FOLDERS => {
                n += 1;
            }
            Err(e) => return Err(fail(&dir, e)),
        }
    };
    write_plan(&dir.join(PLAN_JSON), plan).map_err(|e| fail(&dir, e))?;
    Ok(dir)
}

/// How many same-named batch folders `create_batch` tries before it gives up.
const MAX_BATCH_FOLDERS: u32 = 99;

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
