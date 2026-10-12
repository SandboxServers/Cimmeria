#![allow(dead_code)]
//! Shared by the `lab uat` tests: rows, records, lanes.

use cimmeria_lab_cli::batch::{PlanLane, RowResult, RunRecord, Verdict};
use cimmeria_lab_cli::commands::uat::record::run_verdict;

pub fn row(id: &str, result: &str, reason: &str) -> RowResult {
    RowResult {
        row: id.to_string(),
        result: result.to_string(),
        reasons: vec![reason.to_string()],
        ..RowResult::default()
    }
}

pub fn good() -> Vec<RowResult> {
    vec![row("FS-P1", "PASS", ""), row("FS-P2", "PASS", "")]
}

pub fn frost() -> Vec<RowResult> {
    vec![
        row("FS-P1", "PASS", ""),
        row("FS-P2", "FAIL", "clause movie-over failed"),
    ]
}

/// A record as the PowerShell test's `Rec`: lane 1, 90 s, run dir `d\<inst>-<i>`,
/// verdict from `run_verdict` (or the error as the reason).
pub fn rec(inst: &str, i: u32, ok: bool, rows: Vec<RowResult>, err: Option<&str>) -> RunRecord {
    let verdict = match err {
        Some(e) => Verdict {
            ok: false,
            row: None,
            result: None,
            reason: Some(e.to_string()),
            issue: None,
        },
        None => run_verdict(&rows, &[]),
    };
    RunRecord {
        lane: 1,
        instance: inst.to_string(),
        index: i,
        ok,
        rows,
        verdict: Some(verdict),
        run_dir: Some(format!("d\\{inst}-{i}")),
        error: err.map(str::to_string),
        seconds: 90,
        ..RunRecord::default()
    }
}

pub fn lane(n: u32, instance: &str) -> PlanLane {
    PlanLane {
        lane: n,
        instance: instance.to_string(),
    }
}

pub fn lanes2() -> Vec<PlanLane> {
    vec![lane(1, "default"), lane(2, "p2")]
}

pub fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}
