//! `lab uat --heal` (lab-spec-vocab D-SV8 (a)): after the batch, recalibrate
//! each pose a failed row missed on, save the proposed diffs in the batch
//! folder, and list them in `batch.json`, `summary.md` and `--json`. The spec
//! file is never written.

use std::collections::HashSet;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::time::Duration;

use chrono::Local;
use serde_json::Value;

use crate::batch::RunRecord;
use crate::commands::calibrate;
use crate::commands::calibrate::heal::{invoke_heal, HealHint, HealJob, HealRecord};
use crate::commands::Ctx;
use crate::daemon::mcp::{self, McpClient, McpError};
use crate::daemon::status::Endpoint;

use super::batch_run::{BatchInput, BatchOptions, BatchOutcome};
use super::ctrlc;
use super::progress::{stdio_emit, ProgressMode};
use super::summary::{exit_code, write_batch_summary, BatchMeta, HealEntry, Summary};

/// The distinct `(row, pose)` pairs of the runs' row summaries that carry a
/// heal hint (`{pose, step}`, set only on a failed row whose first failed step
/// missed on a pose), in the order first seen. A `PASS` row (any case), a row
/// with no hint, and a hint with an empty pose or row are ignored.
pub fn heal_hints(runs: &[RunRecord]) -> Vec<HealHint> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out = Vec::new();
    for run in runs {
        for r in &run.rows {
            if r.result.eq_ignore_ascii_case("PASS") {
                continue;
            }
            let pose = r
                .heal
                .as_ref()
                .and_then(|h| h.get("pose"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if pose.is_empty() || r.row.is_empty() {
                continue;
            }
            if seen.insert((r.row.clone(), pose.to_string())) {
                out.push(HealHint {
                    row: r.row.clone(),
                    pose: pose.to_string(),
                });
            }
        }
    }
    out
}

/// The three things a heal pass talks to. `call` is the `lab_uat_calibrate`
/// call, `say` prints one progress line, `wait` (when given) waits for the
/// instance's lease to clear after a failed call, `print_diff` (when given)
/// prints a proposal's text (the caller passes it only in host mode).
pub struct HealIo<'a> {
    pub call: &'a mut dyn FnMut(Value) -> Result<Value, McpError>,
    pub say: &'a mut dyn FnMut(&str),
    pub wait: Option<&'a mut dyn FnMut(&str) -> bool>,
    pub print_diff: Option<&'a mut dyn FnMut(&str)>,
}

/// `lab uat --heal` after the batch: recalibrates each hint of `runs` on
/// `instance`, says a line per hint, and records the proposals as
/// `summary.heal`, rewriting `summary.md` and `batch.json`. Returns the new
/// Markdown, or `None` when no proposal was written (the files stay as the
/// batch left them). The summary is rewritten before a panic from a closure is
/// re-raised, so an interrupted heal still lists the diffs already written.
pub fn heal_pass(
    runs: &[RunRecord],
    summary: &mut Summary,
    batch: &BatchMeta<'_>,
    specs_dir: &str,
    instance: &str,
    io: HealIo<'_>,
) -> Option<String> {
    let HealIo {
        call,
        say,
        wait,
        mut print_diff,
    } = io;
    let hints = heal_hints(runs);
    if hints.is_empty() {
        say("heal: no failed row carries a heal hint; nothing to recalibrate");
        return None;
    }
    let names: Vec<String> = hints
        .iter()
        .map(|h| format!("{} {}", h.row, h.pose))
        .collect();
    say(&format!(
        "heal: recalibrating {} pose(s) on {}: {}",
        hints.len(),
        instance,
        names.join(", ")
    ));
    let job = HealJob {
        batch_dir: batch.dir,
        section: batch.section,
        specs_dir,
        instance,
    };
    let mut records: Vec<HealRecord> = Vec::new();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut on_record = |r: &HealRecord| {
            records.push(r.clone());
            say(&r.line);
            if let (Some(text), Some(print)) = (r.text.as_deref(), print_diff.as_mut()) {
                print(text);
            }
        };
        invoke_heal(&hints, &job, call, Some(&mut on_record), wait)
    }));
    // The PowerShell `finally`: whatever happened, list the proposals.
    let proposed: Vec<HealEntry> = records
        .iter()
        .filter_map(|r| {
            let diff = r.diff.as_ref()?;
            Some(HealEntry {
                row: r.row.clone().unwrap_or_default(),
                pose: r.pose.clone().unwrap_or_default(),
                diff: diff.display().to_string(),
            })
        })
        .collect();
    let mut markdown = None;
    if !proposed.is_empty() {
        summary.heal = proposed;
        match write_batch_summary(summary, batch, Local::now()) {
            Ok(text) => markdown = Some(text),
            Err(e) => say(&format!("heal: cannot write the summary: {e}")),
        }
    }
    if let Err(panic) = outcome {
        resume_unwind(panic);
    }
    markdown
}

/// `--heal` after a native batch (the `-Heal` block of `uat.ps1`): recalibrates
/// each hint on the batch's one instance (`--heal` needs `--leases 1`) and, when
/// a proposal was written, replaces `outcome.md` with the new Markdown and
/// `outcome.exit` with the recomputed code (8). The caller skips it after Ctrl+C.
pub fn heal_batch(
    ctx: &Ctx,
    ep: &Endpoint,
    input: &BatchInput,
    opts: &BatchOptions,
    outcome: &mut BatchOutcome,
) {
    let Some(instance) = input.lanes.first().map(|l| l.instance.clone()) else {
        return;
    };
    let title = outcome.title.clone();
    let meta = BatchMeta {
        dir: &outcome.dir,
        title: &title,
        section: &input.section,
        row_ids: &input.row_ids,
        lane_count: input.lanes.len() as u32,
        runs_per_lease: opts.runs_per_lease,
        started: outcome.started,
    };
    let specs = input.specs_dir.clone().unwrap_or_default();
    // Each call is its own MCP session, as the PowerShell's was. After Ctrl+C a
    // hint that has not started fails at once and the wait gives up.
    let mut call = |a: Value| -> Result<Value, McpError> {
        if ctrlc::stop_requested() {
            return Err(McpError::Tool("interrupted by Ctrl+C".to_string()));
        }
        let mut client = McpClient::connect(ep)?;
        let reply = client.call_tool("lab_uat_calibrate", a, calibrate::outcome::timeout(None));
        client.close();
        reply
    };
    let emit = stdio_emit(opts.mode);
    let mut say = |m: &str| emit(m);
    let mut wait =
        |i: &str| !ctrlc::stop_requested() && mcp::wait_lease_free(ep, i, Duration::from_secs(600));
    let ui = &ctx.ui;
    let mut print = |t: &str| println!("{}", calibrate::render::render_diff(ui, t));
    let io = HealIo {
        call: &mut call,
        say: &mut say,
        wait: Some(&mut wait),
        print_diff: if opts.mode == ProgressMode::Host {
            Some(&mut print)
        } else {
            None
        },
    };
    let healed = heal_pass(
        &outcome.records,
        &mut outcome.summary,
        &meta,
        &specs,
        &instance,
        io,
    );
    if let Some(markdown) = healed {
        outcome.md = markdown;
        outcome.exit = exit_code(&outcome.summary);
    }
}
