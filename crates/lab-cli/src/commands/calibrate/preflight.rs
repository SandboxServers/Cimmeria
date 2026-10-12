//! The checks `lab calibrate` makes before it drives anything, and the pieces
//! of them that need no daemon: row ids, instance choice, foreign clients,
//! the prepare verdict, the checkout.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::daemon::mcp::hide_lease_ids;

/// `--prepare` as clean row ids: split on commas, trimmed, blanks and repeats
/// dropped (clap already splits on commas; this also trims).
pub fn row_ids(rows: &[String]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for r in rows {
        for p in r.split(',') {
            let t = p.trim();
            if !t.is_empty() && !ids.iter().any(|i| i == t) {
                ids.push(t.to_string());
            }
        }
    }
    ids
}

/// The one instance to calibrate on (`Get-UatPlan` with one lease, one run,
/// no client cap). `Err((exit, message))`: exit 2 for an unknown instance,
/// else 3.
pub fn pick_instance(
    hosted: &[String],
    leased: &[String],
    requested: Option<&str>,
) -> Result<String, (u8, String)> {
    let hosted: Vec<&String> = hosted.iter().filter(|h| !h.is_empty()).collect();
    let leased: Vec<&String> = leased.iter().filter(|l| !l.is_empty()).collect();
    let names = |v: &[&String]| v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    if let Some(i) = requested {
        if !hosted.iter().any(|h| h.as_str() == i) {
            return Err((
                2,
                format!(
                    "unknown instance '{i}'; the daemon hosts: {}",
                    names(&hosted)
                ),
            ));
        }
        if leased.iter().any(|l| l.as_str() == i) {
            return Err((
                3,
                format!("instance {i} is leased by another session; wait for it or pick another"),
            ));
        }
        return Ok(i.to_string());
    }
    if hosted.is_empty() {
        return Err((
            3,
            "no lab instance is free (0 hosted, max clients 0)".to_string(),
        ));
    }
    let free: Vec<&String> = hosted
        .iter()
        .filter(|h| !leased.iter().any(|l| l == *h))
        .copied()
        .collect();
    match free.first() {
        Some(first) => Ok((*first).clone()),
        None => {
            let busy: Vec<&String> = hosted
                .iter()
                .filter(|h| leased.iter().any(|l| l == *h))
                .copied()
                .collect();
            Err((
                3,
                format!(
                    "only 0 free instance(s) for 1 lease(s); leased by other sessions: {}",
                    names(&busy)
                ),
            ))
        }
    }
}

/// The pids of `all_sgw` that are not a lab instance's client.
pub fn foreign_pids(lab_pids: &[u32], all_sgw: &[u32]) -> Vec<u32> {
    all_sgw
        .iter()
        .copied()
        .filter(|p| !lab_pids.contains(p))
        .collect()
}

/// The prepare run's verdict from `lab_uat_run`'s `rows`: `None` when every
/// planned row is there and is `PASS` or `SKIPPED`, else the exit-3 message
/// `prepare row <row> <result>: <reason>` (a missing row is result `MISSING`,
/// reason `the run reported no result for <rows>`; the reason is the first of
/// the row's `reasons`, lease ids hidden).
pub fn prepare_failure(rows: &[Value], expected: &[String]) -> Option<String> {
    let text = |r: &Value, k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let got: Vec<String> = rows.iter().map(|r| text(r, "row")).collect();
    let absent: Vec<&str> = expected
        .iter()
        .filter(|e| !e.is_empty() && !got.contains(e))
        .map(String::as_str)
        .collect();
    if let Some(first) = absent.first() {
        return Some(format!(
            "prepare row {first} MISSING: the run reported no result for {}",
            absent.join(", ")
        ));
    }
    for r in rows {
        let result = text(r, "result");
        if result != "PASS" && result != "SKIPPED" {
            let reason = r
                .get("reasons")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .unwrap_or("");
            return Some(format!(
                "prepare row {} {result}: {}",
                text(r, "row"),
                hide_lease_ids(reason)
            ));
        }
    }
    None
}

/// The checkout root from its `docs/guides/uat-specs` folder (three levels up).
pub fn repo_root_from_specs(specs: &Path) -> Option<PathBuf> {
    specs.ancestors().nth(3).map(Path::to_path_buf)
}
