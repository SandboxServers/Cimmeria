//! `lab calibrate`, end to end. Collaborators come in through [`Deps`] so the
//! flow is tested without a daemon or a game client.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::daemon::mcp::{hide_lease_ids, McpError};
use crate::daemon::paths;
use crate::daemon::status::{self, Endpoint};
use crate::ui::{Mark, Tone, Ui};

use super::apply::{apply_diff, spec_dirty, ApplyResult, SPEC_PREFIX};
use super::outcome::{
    apply_error_json, call_error_text, check_arguments, compact_json, exit_code, format_call_error,
    outcomes_from, stop_json, timeout, APPLY_ERROR_MAX,
};
use super::preflight::{foreign_pids, pick_instance, prepare_failure, row_ids};
use super::render::{render_diff, render_line};
use super::Args;

/// What the flow reaches outside itself for.
pub struct Deps<'a> {
    /// The daemon's endpoint (`GET /status` goes here).
    pub ep: &'a Endpoint,
    /// The checkout that `--apply` applies in; `None` when none was found.
    pub repo: Option<PathBuf>,
    /// The default spec folder (`--specs-dir` overrides it).
    pub specs: Option<String>,
    /// Pids of every running `SGW.exe`.
    pub sgw_pids: &'a dyn Fn() -> Vec<u32>,
    /// One MCP tool call: name, arguments, timeout.
    pub call: &'a mut dyn FnMut(&str, Value, Duration) -> Result<Value, McpError>,
}

/// A usage or pre-flight failure: one JSON object on stdout under `--json`,
/// else the message on stderr (plain, as the PowerShell; fancy: `error:` in
/// bold red). Returns the exit code.
fn stop(
    ui: &Ui,
    json: bool,
    code: u8,
    message: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    if json {
        let _ = writeln!(out, "{}", stop_json(code, message));
    } else if ui.fancy {
        let _ = writeln!(err, "{} {message}", ui.paint(Tone::Fail, "error:"));
    } else {
        let _ = writeln!(err, "{message}");
    }
    code
}

/// A progress line on stdout (not under `--json`); fancy mode leads with a
/// cyan `▶`.
fn say(ui: &Ui, json: bool, text: &str, out: &mut dyn Write) {
    if json {
        return;
    }
    if ui.fancy {
        let _ = writeln!(out, "{} {text}", ui.paint(Tone::Cmd, ui.mark(Mark::Active)));
    } else {
        let _ = writeln!(out, "{text}");
    }
}

/// Runs the command and returns its exit code (0, 1, 2 or 3).
pub fn execute(
    args: &Args,
    ui: &Ui,
    deps: &mut Deps<'_>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let json = args.json;
    let section = args.section.as_deref().unwrap_or("");
    let row = args.row.as_deref().unwrap_or("");
    // Argument checks come before the daemon is asked, so they exit 2 whatever its state.
    if let Some(shape) = check_arguments(section, row, args.budget_minutes) {
        return stop(ui, json, 2, &shape, out, err);
    }
    let specs: Option<String> = args
        .specs_dir
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(paths::absolute_specs_dir)
        .or_else(|| deps.specs.clone());
    let repo = deps.repo.clone();
    // A spec with local edits is refused before any lab time is spent; the
    // file the diff names is checked again before git apply.
    if args.apply {
        let Some(repo) = &repo else {
            // No checkout, nowhere to apply: refuse before lab time is spent.
            let m = "no checkout was found (run lab calibrate from inside the repository); --apply has nowhere to apply the diff (drop --apply)";
            return stop(ui, json, 2, m, out, err);
        };
        let guess = format!("{SPEC_PREFIX}{section}.toml");
        if repo.join(&guess).exists() && spec_dirty(repo, &guess) {
            let m = format!("{guess} has uncommitted changes or is not tracked; --apply would clobber them (commit or revert first, or drop --apply)");
            return stop(ui, json, 2, &m, out, err);
        }
    }
    let prepare_rows = row_ids(&args.prepare);

    // Pre-flight: daemon, instance, foreign clients.
    let Some(status) = status::fetch(deps.ep) else {
        return stop(
            ui,
            json,
            3,
            "the lab daemon is not answering (lab status; lab start)",
            out,
            err,
        );
    };
    let hosted: Vec<String> = status
        .instances
        .iter()
        .map(|i| i.instance.clone())
        .collect();
    let leased: Vec<String> = status
        .instances
        .iter()
        .filter(|i| {
            i.lease
                .as_ref()
                .and_then(|l| l.get("held"))
                .and_then(Value::as_bool)
                == Some(true)
        })
        .map(|i| i.instance.clone())
        .collect();
    let requested = args.instance.as_deref().filter(|s| !s.is_empty());
    let lane = match pick_instance(&hosted, &leased, requested) {
        Ok(l) => l,
        Err((code, m)) => return stop(ui, json, code, &m, out, err),
    };
    let lab_pids: Vec<u32> = status
        .instances
        .iter()
        .filter_map(|i| i.client_pid)
        .collect();
    let foreign = foreign_pids(&lab_pids, &(deps.sgw_pids)());
    if !foreign.is_empty() {
        let list: Vec<String> = foreign.iter().map(u32::to_string).collect();
        let m = format!(
            "SGW.exe runs outside the lab (pid {}); close it first, or the lab cannot start its clients",
            list.join(", ")
        );
        return stop(ui, json, 3, &m, out, err);
    }

    // --prepare: walk the client to where the row starts.
    if !prepare_rows.is_empty() {
        say(
            ui,
            json,
            &format!("prepare: {} on {lane}", prepare_rows.join(", ")),
            out,
        );
        // end_session = false: the calibration starts where these rows leave the
        // client, so the run must not log out and disconnect (TG-PIPE-08).
        let mut pa = json!({ "sections": [section], "rows": prepare_rows, "instance": lane, "end_session": false });
        if let Some(s) = &specs {
            pa["specs_dir"] = json!(s);
        }
        match (deps.call)("lab_uat_run", pa, Duration::from_secs(15 * 60)) {
            Err(e) => {
                let text = call_error_text("lab_uat_run", &e);
                let first = text.split('\n').next().unwrap_or("").trim_end_matches('\r');
                let m = format!("prepare failed: {}", hide_lease_ids(first));
                return stop(ui, json, 3, &m, out, err);
            }
            Ok(prep) => {
                let rows: Vec<Value> = prep
                    .get("rows")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if let Some(m) = prepare_failure(&rows, &prepare_rows) {
                    return stop(ui, json, 3, &m, out, err);
                }
            }
        }
    }

    // The sweep.
    let pose = args.pose.as_deref().filter(|p| !p.is_empty());
    let mut ca = json!({ "section": section, "row": row, "instance": lane });
    if let Some(p) = pose {
        ca["pose"] = json!(p);
    }
    if let Some(s) = &specs {
        ca["specs_dir"] = json!(s);
    }
    if let Some(b) = args.budget_minutes {
        ca["budget_minutes"] = json!(b);
    }
    let pose_part = pose.map(|p| format!(" pose {p}")).unwrap_or_default();
    let budget = args.budget_minutes.unwrap_or(15);
    say(
        ui,
        json,
        &format!("calibrate {section} {row}{pose_part} on {lane} (budget {budget} min)"),
        out,
    );
    let reply = match (deps.call)("lab_uat_calibrate", ca, timeout(args.budget_minutes)) {
        Ok(r) => r,
        Err(e) => {
            let m = format_call_error(&call_error_text("lab_uat_calibrate", &e));
            return stop(ui, json, 3, &m, out, err);
        }
    };
    let outcomes = outcomes_from(&reply);
    let code = exit_code(&outcomes);
    let diff_path: Option<String> = outcomes
        .iter()
        .map(|o| o.diff_path.clone())
        .find(|p| !p.is_empty());
    let run_dir = reply
        .get("run_dir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // --apply.
    let mut applied = false;
    if args.apply {
        if let Some(diff) = &diff_path {
            let result = match &repo {
                Some(r) => apply_diff(r, diff),
                None => ApplyResult::refused(
                    diff,
                    "no checkout was found (run lab calibrate from inside the repository); apply it by hand",
                    None,
                ),
            };
            if !result.ok {
                // The error starts with the diff path, and --json keeps the run
                // folder and the poses: a refused apply must not lose the proposal.
                let why = result.error.unwrap_or_default();
                if json {
                    let m = format!("{why} (--apply)");
                    let _ = writeln!(
                        out,
                        "{}",
                        apply_error_json(&m, &run_dir, diff, &outcomes, APPLY_ERROR_MAX)
                    );
                    return 2;
                }
                for o in &outcomes {
                    let _ = writeln!(out, "{}", render_line(ui, o));
                }
                let m = format!("{why} (--apply; the diff stays in the run folder)");
                return stop(ui, false, 2, &m, out, err);
            }
            applied = true;
        }
    }

    if json {
        let _ = writeln!(out, "{}", compact_json(section, row, &outcomes, applied));
        return code;
    }
    if outcomes.is_empty() {
        let _ = writeln!(
            out,
            "lab_uat_calibrate returned no outcomes; run folder {run_dir}"
        );
    }
    for o in &outcomes {
        let _ = writeln!(out, "{}", render_line(ui, o));
    }
    if let Some(diff) = &diff_path {
        if !args.quiet && Path::new(diff).exists() {
            if let Ok(text) = std::fs::read_to_string(diff) {
                let _ = writeln!(out);
                let _ = writeln!(out, "{}", render_diff(ui, &text));
            }
        }
    }
    if applied {
        let repo_text = repo
            .as_ref()
            .map(|r| r.display().to_string())
            .unwrap_or_default();
        let line = format!("applied to the checkout ({repo_text}); review it with git diff, nothing is staged or committed");
        if ui.fancy {
            let _ = writeln!(out, "{} {line}", ui.paint(Tone::Ok, ui.mark(Mark::Pass)));
        } else {
            let _ = writeln!(out, "{line}");
        }
    } else if let Some(diff) = &diff_path {
        let line = match &repo {
            Some(r) => format!(
                "to apply: git -C \"{}\" apply \"{diff}\" (or rerun with --apply)",
                r.display()
            ),
            None => {
                format!("to apply: git apply \"{diff}\" in the checkout (or rerun with --apply)")
            }
        };
        let _ = writeln!(out, "{}", ui.paint(Tone::Dim, &line));
    }
    code
}
