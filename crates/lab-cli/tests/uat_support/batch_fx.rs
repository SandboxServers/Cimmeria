#![allow(dead_code)]
//! Fixtures shared by the lab uat golden, json, exit-code, chaos and heal tests.

use cimmeria_lab_cli::batch::{PlanLane, RowResult, RunRecord, Verdict};
use cimmeria_lab_cli::commands::uat::summary::{LaneStat, RowStat};
use serde_json::json;

pub const DIV_A: &str = "row FS-P3: event 4 expected dialogDisplay(dialog=3995), got dialogDisplay(dialog=2982); after mission(688).step(2)=active";
pub const DIV_B: &str = "row FS-P5: expected windows opened {MissionLog}, got {}";
pub const DIV_LINE: &str = "row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995` after `c2s interact target=tag:ArmYourself_FrostBody`, got `s2c onDialogDisplay@self dialog_id=3996`";
pub const CANNOT: &str = "error: cannot_diff: run d: no fingerprint.json";
pub const PRAXIS: [&str; 7] = [
    "FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4", "FS-P5",
];
pub const BATCH_LONG: &str =
    r"C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832";

/// One row of a `lab_uat_run` reply.
pub fn row(id: &str, result: &str, reason: &str) -> RowResult {
    RowResult {
        row: id.into(),
        result: result.into(),
        reasons: if reason.is_empty() {
            vec![]
        } else {
            vec![reason.into()]
        },
        ..Default::default()
    }
}

/// A row with a heal hint (`pose` None: no hint).
pub fn heal_row(id: &str, result: &str, pose: Option<&str>) -> RowResult {
    let mut r = row(id, result, "step action 2 failed");
    r.heal = pose.map(|p| json!({"pose": p, "step": "hover"}));
    r
}

pub fn good_rows() -> Vec<RowResult> {
    vec![row("FS-P1", "PASS", ""), row("FS-P2", "PASS", "")]
}

pub fn frost_rows() -> Vec<RowResult> {
    vec![
        row("FS-P1", "PASS", ""),
        row("FS-P2", "FAIL", "clause movie-over failed"),
    ]
}

/// One run record (the PowerShell test's `Rec`).
pub fn run(instance: &str, index: u32, ok: bool, rows: Vec<RowResult>) -> RunRecord {
    let bad = rows.iter().find(|r| r.result != "PASS");
    let verdict = Verdict {
        ok,
        row: bad.map(|r| r.row.clone()),
        result: bad.map(|r| r.result.clone()),
        reason: bad.and_then(|r| r.reasons.first().cloned()),
        issue: None,
    };
    RunRecord {
        lane: 1,
        instance: instance.into(),
        index,
        ok,
        rows,
        verdict: Some(verdict),
        run_dir: Some(format!(r"d\{instance}-{index}")),
        seconds: 90,
        ..Default::default()
    }
}

pub fn with_golden(mut r: RunRecord, golden: &str) -> RunRecord {
    r.golden = Some(golden.into());
    r
}

pub fn rs(row: &str, pass: u32, total: u32) -> (String, RowStat) {
    (row.to_string(), RowStat { pass, total })
}

pub fn ls(lane: u32, instance: &str, runs: u32, passed: u32, braked: bool) -> LaneStat {
    LaneStat {
        lane,
        instance: instance.into(),
        runs,
        passed,
        braked,
    }
}

pub fn plan_lanes() -> Vec<PlanLane> {
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
