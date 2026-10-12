//! `lab golden` itself. Everything outside the process (the child `uat`, the
//! daemon endpoint, the default spec folder, stdout and stderr) is passed in,
//! so tests stub it. Ports `golden.ps1`.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};

use super::batch::{self, ChildOutput};
use super::check;
use super::reply::{self, first_line, text_of};
use super::summary;
use super::Args;
use crate::daemon::mcp::{hide_lease_ids, McpClient, McpError};
use crate::daemon::status::Endpoint;
use crate::ui::{Mark, Tone, Ui};

/// `Invoke-McpTool ... 300`: the tool reads the run bundles, so it can take a while.
const TOOL_TIMEOUT: Duration = Duration::from_secs(300);

/// What `execute` needs from outside.
pub struct Deps<'a> {
    /// Runs `<this exe> uat <args>` and returns its stdout and exit code.
    pub run_uat: &'a dyn Fn(&[String]) -> io::Result<ChildOutput>,
    /// The daemon `lab_golden_record` is sent to.
    pub endpoint: Endpoint,
    /// The checkout's `docs/guides/uat-specs` when there is one.
    pub default_specs: Option<PathBuf>,
}

/// `{"ok":false,"exit":N,"error":...[,"batch":...]}`.
#[derive(Serialize)]
struct StopJson<'a> {
    ok: bool,
    exit: u8,
    error: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<&'a str>,
}

struct Io<'a> {
    ui: &'a Ui,
    json: bool,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
}

impl Io<'_> {
    /// A failure: plain text on stderr (fancy: after a red `error:`), or under
    /// `--json` one object on stdout naming the batch folder once the runs
    /// exist. Lease ids never leave. Returns the exit code.
    fn stop(&mut self, code: u8, message: &str, batch: Option<&str>) -> u8 {
        let message = hide_lease_ids(message);
        if self.json {
            let body = StopJson {
                ok: false,
                exit: code,
                error: &message,
                batch: batch.filter(|b| !b.is_empty()),
            };
            let text = serde_json::to_string(&body).unwrap_or_default();
            let _ = writeln!(self.out, "{text}");
        } else {
            self.fail_line(&message);
        }
        code
    }

    /// One failure line on stderr; plain mode prints the bare message.
    fn fail_line(&mut self, message: &str) {
        let text = if self.ui.fancy {
            format!("{} {message}", self.ui.paint(Tone::Fail, "error:"))
        } else {
            message.to_string()
        };
        let _ = writeln!(self.err, "{text}");
    }
}

/// Runs `lab golden` and returns its exit code: 0 recorded (or shown); 1 a run
/// or row did not pass; 2 usage, or a golden exists without `--rebless`
/// (show: no golden); 3 the `lab uat` pre-flight or plan failed (nothing
/// driven); 6 the runs disagree; 7 the runs cannot be recorded as given, the
/// daemon failed after the runs, or (show) the golden cannot be read.
pub fn execute(args: &Args, ui: &Ui, deps: &Deps, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let action = args.action.as_deref().unwrap_or("");
    let section = args.section.as_deref().unwrap_or("");
    let server_version = args.server_version.as_deref().unwrap_or("");
    let mut io = Io {
        ui,
        json: args.json,
        out,
        err,
    };
    if let Some(bad) =
        check::validate_arguments(action, section, args.runs, args.leases, server_version)
    {
        return io.stop(2, &bad, None);
    }
    let specs: Option<PathBuf> = args
        .specs_dir
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| deps.default_specs.clone());
    if action == "show" {
        return show(&mut io, section, specs.as_deref());
    }
    record(
        &mut io,
        args,
        deps,
        section,
        server_version,
        specs.as_deref(),
    )
}

/// `lab golden show`: no daemon.
fn show(io: &mut Io, section: &str, specs: Option<&Path>) -> u8 {
    let Some(specs) = specs else {
        return io.stop(2, "no spec folder here; pass --specs-dir", None);
    };
    let path = check::golden_path(specs, section);
    if !path.exists() {
        let message = format!(
            "no golden for {section} ({}); record one with lab golden record",
            path.display()
        );
        return io.stop(2, &message, None);
    }
    let parsed: Result<Value, String> = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|text| {
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| e.to_string())
        });
    let golden = match parsed {
        Ok(golden) => golden,
        Err(e) => {
            let message = format!("cannot read {}: {}", path.display(), first_line(&e));
            return io.stop(7, &message, None);
        }
    };
    let summary = summary::summarize(&golden);
    let text = if io.json {
        summary::show_json(&summary)
    } else if io.ui.fancy {
        summary::summary_fancy(io.ui, &summary)
    } else {
        summary::summary_plain(&summary)
    };
    let _ = writeln!(io.out, "{text}");
    0
}

/// `lab golden record`: the batch, then `lab_golden_record`.
fn record(
    io: &mut Io,
    args: &Args,
    deps: &Deps,
    section: &str,
    server_version: &str,
    specs: Option<&Path>,
) -> u8 {
    // Away from a checkout the daemon picks its own spec folder, and its
    // `exists` refusal says the same thing after the runs.
    if let Some(specs) = specs {
        if !args.rebless && check::golden_path(specs, section).exists() {
            return io.stop(
                2,
                "a golden exists; re-blessing needs --rebless and five fresh runs",
                None,
            );
        }
    }
    let requested = check::row_ids(&args.rows);
    let specs_text = specs.map(|p| p.to_string_lossy().into_owned());
    let child_args = match batch::batch_args(
        section,
        &requested,
        args.runs,
        args.leases,
        server_version,
        specs_text.as_deref(),
    ) {
        Ok(child_args) => child_args,
        Err(message) => return io.stop(2, &message, None),
    };
    if !io.json {
        let line = format!(
            "lab golden record {section}: {} fingerprint run(s) on {} lane(s), then lab_golden_record (a run takes a few minutes)",
            args.runs, args.leases
        );
        let text = if io.ui.fancy {
            format!(
                "{} {line}",
                io.ui.paint(Tone::Cmd, io.ui.mark(Mark::Active))
            )
        } else {
            line
        };
        let _ = writeln!(io.out, "{text}");
    }
    let child = match (deps.run_uat)(&child_args) {
        Ok(child) => child,
        Err(e) => return io.stop(3, &format!("lab uat could not start: {e}"), None),
    };
    let lines: Vec<String> = child.stdout.lines().map(str::to_owned).collect();
    let outcome = batch::parse_batch_output(&lines, child.code);
    if outcome.exit != 0 {
        return io.stop(
            outcome.exit,
            outcome.error.as_deref().unwrap_or(""),
            outcome.batch.as_deref(),
        );
    }
    let batch_dir = outcome.batch.unwrap_or_default();

    // Run folders come from batch.jsonl: --json names only the batch (F16).
    let records = match batch::read_records(&Path::new(&batch_dir).join("batch.jsonl")) {
        Ok(records) => records,
        Err(e) => {
            let message = format!(
                "cannot read the batch's runs: {}; {batch_dir}",
                first_line(&e)
            );
            return io.stop(7, &message, Some(&batch_dir));
        }
    };
    let Some(dirs) = batch::run_dirs(&records) else {
        let message = format!("the batch passed but a run has no run folder; {batch_dir}");
        return io.stop(7, &message, Some(&batch_dir));
    };
    let mut tool_args = json!({"section": section, "run_dirs": dirs, "rebless": args.rebless});
    if !requested.is_empty() {
        tool_args["rows"] = json!(requested);
    }
    if let Some(specs) = &specs_text {
        tool_args["specs_dir"] = json!(specs);
    }
    if deps.endpoint.token.as_deref().is_none_or(str::is_empty) {
        let message =
            format!("the lab daemon is not answering (lab status); the runs are in {batch_dir}");
        return io.stop(7, &message, Some(&batch_dir));
    }
    let reply = match call_record(&deps.endpoint, tool_args) {
        Ok(reply) => reply,
        Err(e) => {
            let message = format!(
                "lab_golden_record failed: {}; the runs are in {batch_dir}",
                first_line(&e.to_string())
            );
            return io.stop(7, &message, Some(&batch_dir));
        }
    };
    let code = reply::reply_exit(&reply);
    if io.json {
        let text = hide_lease_ids(&reply::compact_json(&reply, code, Some(&batch_dir)));
        let _ = writeln!(io.out, "{text}");
    } else if code == 0 {
        let text = if io.ui.fancy {
            reply::record_line_fancy(io.ui, section, &reply)
        } else {
            reply::record_line(section, &reply)
        };
        let _ = writeln!(io.out, "{text}");
    } else {
        let message = format!(
            "{}: {}; the runs are in {batch_dir}",
            text_of(reply.get("code")),
            text_of(reply.get("error"))
        );
        io.fail_line(&hide_lease_ids(&message));
    }
    code
}

/// One `lab_golden_record` call on its own MCP session.
fn call_record(endpoint: &Endpoint, tool_args: Value) -> Result<Value, McpError> {
    let mut client = McpClient::connect(endpoint)?;
    let reply = client.call_tool("lab_golden_record", tool_args, TOOL_TIMEOUT);
    client.close();
    reply
}
