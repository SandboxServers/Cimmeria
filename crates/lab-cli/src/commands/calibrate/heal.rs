//! `lab uat --heal` (D-SV8 (a)): for each (row, pose) hint, one
//! `lab_uat_calibrate` call for that pose on the batch's instance; a proposed
//! diff is copied into the batch folder as `heal-<row>-<pose>.diff`. The spec
//! file is never written. The MCP call and the lease wait are closures, so
//! tests pass stubs and `uat` passes the real ones.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::daemon::mcp::McpError;

use super::outcome::{call_error_text, format_call_error, format_line, outcomes_from, Outcome};

/// One pose a failed row missed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealHint {
    /// The failed row.
    pub row: String,
    /// The pose of that row to recalibrate.
    pub pose: String,
}

/// Where and what to heal. `specs_dir` and `instance` are empty for "none".
#[derive(Debug, Clone, Copy)]
pub struct HealJob<'a> {
    /// The batch folder the proposals are copied into.
    pub batch_dir: &'a Path,
    /// The UAT section of every hint.
    pub section: &'a str,
    /// The spec folder to pass on, or empty.
    pub specs_dir: &'a str,
    /// The instance the batch ran on, or empty.
    pub instance: &'a str,
}

/// One hint's result. The last record of a stopped heal has `row` and `pose`
/// `None` and only `line` set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealRecord {
    /// The hint's row (`None` on the stop record).
    pub row: Option<String>,
    /// The hint's pose (`None` on the stop record).
    pub pose: Option<String>,
    /// The batch copy of the proposal, when there is one.
    pub diff: Option<PathBuf>,
    /// The one line the summary prints for this hint.
    pub line: String,
    /// The proposal's text, kept so the caller can print it.
    pub text: Option<String>,
}

/// `heal-<row>-<pose>.diff`, anything but letters, digits, `.`, `_` and `-`
/// made `_`.
pub fn heal_file_name(row: &str, pose: &str) -> String {
    let safe = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    };
    format!("heal-{}-{}.diff", safe(row), safe(pose))
}

/// Calls `lab_uat_calibrate` once per hint (arguments `{section, row, pose,
/// instance?, specs_dir?}`) and returns one record per hint.
///
/// `on_record` gets each record as soon as its call is done, so a caller
/// interrupted mid-heal still has the earlier ones. `wait_lease_free` gets the
/// instance after a call that failed and before the next hint (a call our
/// timeout ended may still hold its lease): when it returns false, healing
/// stops with a last record naming the poses left.
pub fn invoke_heal(
    hints: &[HealHint],
    job: &HealJob<'_>,
    call: &mut dyn FnMut(Value) -> Result<Value, McpError>,
    mut on_record: Option<&mut dyn FnMut(&HealRecord)>,
    mut wait_lease_free: Option<&mut dyn FnMut(&str) -> bool>,
) -> Vec<HealRecord> {
    let mut results = Vec::new();
    for (i, hint) in hints.iter().enumerate() {
        let (row, pose) = (hint.row.as_str(), hint.pose.as_str());
        let mut args = json!({ "section": job.section, "row": row, "pose": pose });
        if !job.instance.is_empty() {
            args["instance"] = json!(job.instance);
        }
        if !job.specs_dir.is_empty() {
            args["specs_dir"] = json!(job.specs_dir);
        }
        let mut rec = HealRecord {
            row: Some(row.to_string()),
            pose: Some(pose.to_string()),
            diff: None,
            line: String::new(),
            text: None,
        };
        let mut threw = false;
        match call(args) {
            Ok(reply) => {
                let outcome = outcomes_from(&reply).into_iter().find(|o| o.pose == pose);
                match outcome {
                    None => {
                        rec.line = format!(
                            "heal {row} {pose}: lab_uat_calibrate returned no outcome for the pose"
                        );
                    }
                    Some(o) => {
                        let src = Path::new(&o.diff_path);
                        if o.ok && !o.diff_path.is_empty() && src.exists() {
                            let dest = job.batch_dir.join(heal_file_name(row, pose));
                            match std::fs::copy(src, &dest) {
                                Ok(_) => {
                                    rec.text = std::fs::read_to_string(&dest).ok();
                                    // The line names the batch copy, the one the summary lists.
                                    let shown = Outcome {
                                        diff_path: dest.display().to_string(),
                                        ..o.clone()
                                    };
                                    rec.line = format!("heal {row} {}", format_line(&shown));
                                    rec.diff = Some(dest);
                                }
                                Err(e) => {
                                    threw = true;
                                    rec.line = format!(
                                        "heal {row} {pose}: {}",
                                        format_call_error(&format!(
                                            "cannot save the proposal: {e}"
                                        ))
                                    );
                                }
                            }
                        } else {
                            rec.line = format!("heal {row} {}", format_line(&o));
                        }
                    }
                }
            }
            Err(e) => {
                threw = true;
                rec.line = format!(
                    "heal {row} {pose}: {}",
                    format_call_error(&call_error_text("lab_uat_calibrate", &e))
                );
            }
        }
        if let Some(f) = on_record.as_deref_mut() {
            f(&rec);
        }
        results.push(rec);
        // Nothing to wait for after the last hint: lab uat ends there.
        if threw && i + 1 < hints.len() {
            if let Some(wait) = wait_lease_free.as_deref_mut() {
                if !wait(job.instance) {
                    let left: Vec<String> = hints[i + 1..]
                        .iter()
                        .map(|h| format!("{} {}", h.row, h.pose))
                        .collect();
                    let stop = HealRecord {
                        row: None,
                        pose: None,
                        diff: None,
                        text: None,
                        line: format!(
                            "heal stopped: the lease on {} did not clear after the failed call; not recalibrated: {}",
                            job.instance,
                            left.join(", ")
                        ),
                    };
                    if let Some(f) = on_record.as_deref_mut() {
                        f(&stop);
                    }
                    results.push(stop);
                    break;
                }
            }
        }
    }
    results
}
