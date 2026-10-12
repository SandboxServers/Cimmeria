//! `lab uat` before anything is driven. Collaborators come in through [`Deps`]
//! so the pre-flight is tested without a daemon or a game client.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::ascii_json::to_ascii_json;
use crate::batch::PlanLane;
use crate::daemon::env_file::{self, EnvLine};
use crate::daemon::mcp::{hide_lease_ids, McpError};
use crate::daemon::paths;
use crate::daemon::status::{self, Endpoint};
use crate::ui::{Mark, Tone, Ui};

use super::args::{
    chaos_header, chaos_specs, check_arguments, check_chaos_leases, progress_mode, row_ids,
    Progress, USAGE,
};
use super::plan::{first_line, judge_plan, pick_lanes, RunCall};
use super::Args;

/// The plan call is allowed two minutes (`Invoke-McpTool ... 120`).
pub const PLAN_TIMEOUT: Duration = Duration::from_secs(120);

/// What the pre-flight reaches outside itself for.
pub struct Deps<'a> {
    /// The daemon's endpoint (`GET /status` goes here).
    pub ep: &'a Endpoint,
    /// The default spec folder (`--specs-dir` overrides it); `None` outside a checkout.
    pub specs: Option<String>,
    /// The daemon's `CIMMERIA_LAB_MAX_CLIENTS`, 0 for none.
    pub max_clients: u32,
    /// Pids of every running `SGW.exe`.
    pub sgw_pids: &'a dyn Fn() -> Vec<u32>,
    /// One MCP tool call: name, arguments, timeout.
    pub call: &'a mut dyn FnMut(&str, Value, Duration) -> Result<Value, McpError>,
}

/// Everything LX-16 needs to drive the batch after a ready pre-flight.
#[derive(Debug, Clone)]
pub struct Ready {
    /// The section to run.
    pub section: String,
    /// The lanes: one instance each, numbered from 1.
    pub lanes: Vec<PlanLane>,
    /// The rows the runner will run.
    pub rows: Vec<String>,
    /// The runner's chaos header, else the typed one.
    pub chaos: Option<String>,
    /// The arguments shared by every run (plan_only off, no instance).
    pub call: RunCall,
    /// `--diff-golden`, or implied by `--chaos`.
    pub diff_golden: bool,
    /// The spec folder in use.
    pub specs: Option<String>,
}

/// How the pre-flight ended. Returned once per run, so the size of `Ready` costs
/// nothing and stays unboxed for the callers that match on it.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Outcome {
    /// Finished: the exit code (a refusal, or `--plan-only` done).
    Done(u8),
    /// Ready to drive.
    Drive(Ready),
}

#[derive(Serialize)]
/// The --json failure object {ok, exit, error}.
struct StopJson<'a> {
    ok: bool,
    exit: u8,
    error: &'a str,
}

#[derive(Serialize)]
/// The --plan-only --json object.
struct PlanJson<'a> {
    ok: bool,
    plan_only: bool,
    rows: &'a [String],
    lanes: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chaos: Option<&'a str>,
}

/// A usage or pre-flight failure: under `--json` one object on stdout
/// (`{"ok":false,"exit":N,"error":...}`), else the message on stderr (plain, as
/// the PowerShell; fancy: `error:` in bold red first).
pub fn stop(
    ui: &Ui,
    json: bool,
    code: u8,
    message: &str,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Outcome {
    if json {
        let body = StopJson {
            ok: false,
            exit: code,
            error: message,
        };
        let _ = writeln!(out, "{}", to_ascii_json(&body));
    } else if ui.fancy {
        let _ = writeln!(err, "{} {message}", ui.paint(Tone::Fail, "error:"));
    } else {
        let _ = writeln!(err, "{message}");
    }
    Outcome::Done(code)
}

/// One progress line to where `progress` says.
fn say(progress: Progress, text: &str, out: &mut dyn Write, err: &mut dyn Write) {
    match progress {
        Progress::Host => {
            let _ = writeln!(out, "{text}");
        }
        Progress::Stderr => {
            let _ = writeln!(err, "{text}");
        }
        Progress::None => {}
    }
}

/// The committed golden of a section under a spec folder, named as the daemon
/// names it: every character outside `[A-Za-z0-9_-]` becomes `_`, so `v1.2` is
/// `golden/v1_2.json` (the same rule as LX-11's `golden_path`, decision 7).
pub fn golden_file(specs: &Path, section: &str) -> PathBuf {
    let safe: String = section
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    specs.join("golden").join(format!("{safe}.json"))
}

/// Why a committed golden cannot be used, or `None` when it reads as a JSON
/// object with `schema`, `section` and `fingerprint` (a `null` counts as
/// missing). A broken golden never costs a batch.
pub fn golden_unreadable(path: &Path) -> Option<String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return Some(first_line(&e.to_string())),
    };
    let text = String::from_utf8_lossy(&bytes);
    let value: Value = match serde_json::from_str(text.trim_start_matches('\u{feff}')) {
        Ok(v) => v,
        Err(e) => return Some(first_line(&e.to_string())),
    };
    let Some(object) = value.as_object() else {
        return Some("it is not a JSON object".to_string());
    };
    let missing: Vec<&str> = ["schema", "section", "fingerprint"]
        .into_iter()
        .filter(|k| object.get(*k).is_none_or(Value::is_null))
        .collect();
    if missing.is_empty() {
        None
    } else {
        Some(format!("it has no {}", missing.join(", ")))
    }
}

/// `CIMMERIA_LAB_MAX_CLIENTS` from the labd.env lines: all digits (trimmed),
/// else 0.
pub fn max_clients_from(lines: &[EnvLine]) -> u32 {
    env_file::get(lines, "CIMMERIA_LAB_MAX_CLIENTS")
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

/// Runs the pre-flight. Exit 2 for usage (before the daemon is asked), 3 when
/// the daemon, an instance, a foreign client or the plan stops it; with
/// `--plan-only` it prints the plan and finishes with 0; otherwise it returns
/// [`Outcome::Drive`].
pub fn execute(
    args: &Args,
    ui: &Ui,
    deps: &mut Deps<'_>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Outcome {
    let json = args.json;
    let Some(section) = args.section.as_deref().filter(|s| !s.is_empty()) else {
        return stop(ui, json, 2, USAGE, out, err);
    };
    // Argument checks come before the daemon is asked, so they exit 2 whatever its state.
    if let Some(shape) = check_arguments(args) {
        return stop(ui, json, 2, &shape, out, err);
    }
    let chaos = chaos_specs(&args.chaos);
    if let Some(shape) = check_chaos_leases(&chaos, args.leases) {
        return stop(ui, json, 2, &shape, out, err);
    }
    let specs: Option<String> = args
        .specs_dir
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(paths::absolute_specs_dir)
        .or_else(|| deps.specs.clone());
    // lab-chaos D-CH7: a chaos run is reported against the golden, so --chaos
    // implies --diff-golden (and with it fingerprint capture on every row).
    let chaos_on = !chaos.is_empty();
    let diff_golden = args.diff_golden || chaos_on;
    // A diff needs a golden: without one, stop before anything is driven. A
    // chaos plan drives nothing, so it is not checked. With no local spec folder
    // a chaos run lets the daemon resolve its own (a missing golden is then each
    // run's diff error, exit 5); --diff-golden alone still needs a spec folder.
    let check_golden =
        diff_golden && !(chaos_on && args.plan_only) && (specs.is_some() || !chaos_on);
    if check_golden {
        let Some(dir) = &specs else {
            let m = "--diff-golden needs --specs-dir (no spec folder found next to this checkout)";
            return stop(ui, json, 2, m, out, err);
        };
        let path = golden_file(Path::new(dir), section);
        if !path.exists() {
            let m = if chaos_on {
                format!("no golden for {section}; --chaos reports against it, so record one first with lab golden record (lab-golden GD-10)")
            } else {
                format!("no golden for {section}; record one with lab golden record")
            };
            return stop(ui, json, 2, &m, out, err);
        }
        if let Some(why) = golden_unreadable(&path) {
            let m = format!("golden for {section} cannot be read: {why}");
            return stop(ui, json, 2, &m, out, err);
        }
    }
    let requested = row_ids(&args.rows);
    let progress = progress_mode(args.json, args.quiet, args.progress_to_stderr);

    // Pre-flight: daemon, instances, leases, foreign clients.
    let Some(status) = status::fetch(deps.ep) else {
        let m = "the lab daemon is not answering (lab status; lab start)";
        return stop(ui, json, 3, m, out, err);
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
    let instance = args.instance.as_deref().filter(|s| !s.is_empty());
    let lanes = match pick_lanes(
        args.leases,
        args.runs_per_lease,
        &hosted,
        &leased,
        deps.max_clients,
        instance,
    ) {
        Ok(lanes) => lanes,
        Err(s) => return stop(ui, json, s.code, &s.message, out, err),
    };
    if !args.plan_only {
        let lab_pids: Vec<u32> = status
            .instances
            .iter()
            .filter_map(|i| i.client_pid)
            .collect();
        let foreign: Vec<String> = (deps.sgw_pids)()
            .into_iter()
            .filter(|pid| !lab_pids.contains(pid))
            .map(|pid| pid.to_string())
            .collect();
        if !foreign.is_empty() {
            let m = format!(
                "SGW.exe runs outside the lab (pid {}); close it first, or the lab cannot start its clients",
                foreign.join(", ")
            );
            return stop(ui, json, 3, &m, out, err);
        }
    }

    // Plan once: a row that cannot run, or a requested row the spec lacks,
    // stops the batch before anything is driven.
    let base = RunCall::new(
        section,
        &requested,
        specs.as_deref(),
        args.server_version.as_deref(),
        args.fixtures.as_deref(),
        &chaos,
        args.fingerprint || diff_golden,
    );
    let first = lanes
        .first()
        .map(|l| l.instance.clone())
        .unwrap_or_default();
    let reply = match (deps.call)(
        "lab_uat_run",
        base.for_plan(&first).to_value(),
        PLAN_TIMEOUT,
    ) {
        Ok(reply) => reply,
        Err(e) => {
            let m = format!(
                "the plan call failed: {}",
                hide_lease_ids(&first_line(&e.to_string()))
            );
            return stop(ui, json, 3, &m, out, err);
        }
    };
    let planned = match judge_plan(&reply, &requested, section) {
        Ok(p) => p,
        Err(s) => return stop(ui, json, s.code, &s.message, out, err),
    };
    // The runner's resolved header replaces the typed one (lab-chaos CH-08).
    let header = planned.chaos.clone().or_else(|| chaos_header(&chaos));
    let lane_names: Vec<&str> = lanes.iter().map(|l| l.instance.as_str()).collect();

    if args.plan_only {
        if json {
            let body = PlanJson {
                ok: true,
                plan_only: true,
                rows: &planned.rows,
                lanes: lane_names.clone(),
                chaos: header.as_deref(),
            };
            let _ = writeln!(out, "{}", to_ascii_json(&body));
        } else {
            if let Some(h) = &header {
                let line = if ui.fancy {
                    ui.paint(Tone::Dim, h)
                } else {
                    h.clone()
                };
                say(progress, &line, out, err);
            }
            let text = format!(
                "plan: {} row(s) ready on {}: {}",
                planned.rows.len(),
                lane_names.join(", "),
                planned.rows.join(", ")
            );
            let line = if ui.fancy {
                format!("{} {text}", ui.paint(Tone::Ok, ui.mark(Mark::Pass)))
            } else {
                text
            };
            say(progress, &line, out, err);
        }
        return Outcome::Done(0);
    }

    Outcome::Drive(Ready {
        section: section.to_string(),
        lanes,
        rows: planned.rows,
        chaos: header,
        call: base,
        diff_golden,
        specs,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn lines(text: &str) -> Vec<EnvLine> {
        env_file::parse(text)
    }

    #[test]
    fn max_clients_reads_digits_only() {
        assert_eq!(max_clients_from(&lines("CIMMERIA_LAB_MAX_CLIENTS=2\n")), 2);
        assert_eq!(
            max_clients_from(&lines("CIMMERIA_LAB_MAX_CLIENTS= 3 \n")),
            3
        );
        assert_eq!(
            max_clients_from(&lines("CIMMERIA_LAB_MAX_CLIENTS=two\n")),
            0
        );
        assert_eq!(max_clients_from(&lines("CIMMERIA_LAB_MAX_CLIENTS=-1\n")), 0);
        assert_eq!(max_clients_from(&lines("OTHER=2\n")), 0);
        assert_eq!(max_clients_from(&[]), 0);
    }

    // the removed PowerShell golden library Get-GoldenPath: 'v1.2' is golden/v1_2.json
    #[test]
    fn the_golden_file_is_named_as_the_daemon_names_it() {
        let specs = Path::new("S");
        assert_eq!(
            golden_file(specs, "first-session"),
            specs.join("golden").join("first-session.json")
        );
        assert_eq!(
            golden_file(specs, "v1.2"),
            specs.join("golden").join("v1_2.json")
        );
        assert_eq!(
            golden_file(specs, "gm_Parity-2"),
            specs.join("golden").join("gm_Parity-2.json")
        );
    }

    // the removed PowerShell uat library Test-UatGoldenFile
    #[test]
    fn a_golden_must_be_json_with_schema_section_and_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let at = |name: &str, text: &str| {
            let p = dir.path().join(name);
            fs::write(&p, text).unwrap();
            p
        };
        let good = at(
            "good.json",
            r#"{"schema":1,"section":"first-session","fingerprint":{}}"#,
        );
        assert_eq!(golden_unreadable(&good), None);
        let bom = at(
            "bom.json",
            "\u{feff}{\"schema\":1,\"section\":\"s\",\"fingerprint\":[]}",
        );
        assert_eq!(golden_unreadable(&bom), None);
        let partial = at("partial.json", r#"{"schema":1,"section":null}"#);
        assert_eq!(
            golden_unreadable(&partial),
            Some("it has no section, fingerprint".to_string())
        );
        let array = at("array.json", "[1]");
        assert_eq!(
            golden_unreadable(&array),
            Some("it is not a JSON object".to_string())
        );
        let broken = at("broken.json", "{");
        assert!(golden_unreadable(&broken).is_some());
        assert!(golden_unreadable(&dir.path().join("none.json")).is_some());
    }
}
