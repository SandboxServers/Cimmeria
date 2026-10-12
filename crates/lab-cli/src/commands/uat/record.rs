//! One run's record: the verdict of its rows, the brake, the error text and the
//! UTC stamps. Pure: no daemon, no clock reads of its own (callers pass times).

use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::Value;

use crate::batch::{RowResult, RunRecord, Verdict};
use crate::daemon::mcp::{hide_lease_ids, McpError};
use crate::known_issues;

fn eqi(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// One run's verdict from `lab_uat_run`'s rows (`Get-UatRunVerdict`). `expected`
/// are the planned row ids: a row the run did not report fails it, result
/// `MISSING`, naming every absent row. Otherwise the first row that is neither
/// `PASS` nor `SKIPPED` decides: its row, result, reason (first reason, lease ids
/// hidden) and the known issue `known_issues::tag` finds for it. All rows passing
/// is `ok` with every other field `None`.
pub fn run_verdict(rows: &[RowResult], expected: &[String]) -> Verdict {
    let absent: Vec<&str> = expected
        .iter()
        .map(String::as_str)
        .filter(|id| !id.is_empty() && !rows.iter().any(|r| eqi(&r.row, id)))
        .collect();
    if let Some(first) = absent.first() {
        return Verdict {
            ok: false,
            row: Some((*first).to_string()),
            result: Some("MISSING".to_string()),
            reason: Some(format!(
                "the run reported no result for {}",
                absent.join(", ")
            )),
            issue: None,
        };
    }
    let Some(first) = rows
        .iter()
        .find(|r| !eqi(&r.result, "PASS") && !eqi(&r.result, "SKIPPED"))
    else {
        return Verdict {
            ok: true,
            ..Verdict::default()
        };
    };
    let reason = hide_lease_ids(first.reasons.first().map(String::as_str).unwrap_or(""));
    let issue = known_issues::tag(&first.row, &reason).map(str::to_string);
    Verdict {
        ok: false,
        row: Some(first.row.clone()),
        result: Some(first.result.clone()),
        reason: Some(reason),
        issue,
    }
}

/// `Test-UatBrake`: true when the failed runs at the end of `outcomes` number at
/// least `max_consecutive` (0 never brakes). `outcomes` is `true` for a pass.
pub fn brake(outcomes: &[bool], max_consecutive: u32) -> bool {
    if max_consecutive == 0 {
        return false;
    }
    let streak = outcomes.iter().rev().take_while(|ok| !**ok).count();
    streak >= max_consecutive as usize
}

/// `Test-UatStopLane`: the brake, but never after the lane's last run (a lane
/// that ran every run did not stop early).
pub fn stop_lane(index: u32, total: u32, outcomes: &[bool], max_consecutive: u32) -> bool {
    index < total && brake(outcomes, max_consecutive)
}

/// The text stored for a run whose call failed: the error's first line with lease
/// ids hidden. A tool error reads `lab_uat_run failed: <message>` (as
/// `Invoke-McpTool` threw it); any other error keeps its own text.
pub fn error_text(error: &McpError) -> String {
    let full = match error {
        McpError::Tool(message) => format!("lab_uat_run failed: {message}"),
        other => other.to_string(),
    };
    hide_lease_ids(full.lines().next().unwrap_or(""))
}

/// Whether an error text is a timeout (`Timeout|timed out|canceled`, any case).
pub fn is_timeout(error: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new("(?i)timeout|timed out|canceled").expect("timeout regex"))
        .is_match(error)
}

/// `2026-10-11T01:39:06.0000000Z`: PowerShell's round-trip (`'o'`) form of a UTC
/// time, seven fractional digits.
pub fn utc_stamp(at: DateTime<Utc>) -> String {
    // One implementation: LX-15's `plan.json` stamp.
    super::plan::started_text(at)
}

/// The record a run starts as: lane, instance, index, `Started`, nothing else set.
pub fn new_record(lane: u32, instance: &str, index: u32, started: DateTime<Utc>) -> RunRecord {
    RunRecord {
        lane,
        instance: instance.to_string(),
        index,
        started: Some(utc_stamp(started)),
        ..RunRecord::default()
    }
}

/// The rows of a `lab_uat_run` reply: its `rows` array, `null` elements and
/// elements that are not rows dropped. A reply that is not an object has none.
pub fn rows_of(reply: &Value) -> Vec<RowResult> {
    reply
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| !row.is_null())
                .filter_map(|row| serde_json::from_value::<RowResult>(row.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Fills `rec` from a reply: `Rows`, `RunDir` (the reply's `run_dir`, `""` when
/// it has none), `Verdict` (against `expected`) and `Ok`.
pub fn apply_reply(rec: &mut RunRecord, reply: &Value, expected: &[String]) {
    rec.rows = rows_of(reply);
    rec.run_dir = Some(
        reply
            .get("run_dir")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    );
    let verdict = run_verdict(&rec.rows, expected);
    rec.ok = verdict.ok;
    rec.verdict = Some(verdict);
}

/// Fills `rec` from a failed call: `Error`, and a verdict with the error as its
/// reason and nothing else. Returns whether it was a timeout.
pub fn apply_error(rec: &mut RunRecord, error: &McpError) -> bool {
    let text = error_text(error);
    let timed_out = is_timeout(&text);
    rec.ok = false;
    rec.verdict = Some(Verdict {
        ok: false,
        row: None,
        result: None,
        reason: Some(text.clone()),
        issue: None,
    });
    rec.error = Some(text);
    timed_out
}

/// The record of a lane that died outside a run: `Index` 0, braked, `Error`
/// `lane: <message>` (first line, lease ids hidden), both stamps `at`.
pub fn lane_death(lane: u32, instance: &str, message: &str, at: DateTime<Utc>) -> RunRecord {
    let err = hide_lease_ids(message.lines().next().unwrap_or(""));
    let stamp = utc_stamp(at);
    RunRecord {
        lane,
        instance: instance.to_string(),
        index: 0,
        ok: false,
        rows: Vec::new(),
        verdict: Some(Verdict {
            ok: false,
            row: None,
            result: None,
            reason: Some(err.clone()),
            issue: None,
        }),
        run_dir: None,
        error: Some(format!("lane: {err}")),
        seconds: 0,
        braked: true,
        started: Some(stamp.clone()),
        ended: Some(stamp),
        chaos: None,
        golden: None,
    }
}
