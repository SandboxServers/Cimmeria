//! `lab calibrate`: recalibrate a row's poses by hover and propose a spec diff.
//!
//! LX-12a holds the pure half: [`outcome`] (what the daemon returns, the human
//! line, the `--json` objects, argument checks), [`render`] (fancy mode),
//! [`apply`] (`--apply`'s git flow) and [`heal`] (what `lab uat --heal` calls).
//! LX-12b adds [`preflight`] (the checks before anything is driven) and
//! [`flow`] (the whole command), which [`run`] drives.

pub mod apply;
pub mod flow;
pub mod heal;
pub mod outcome;
pub mod preflight;
pub mod render;

pub use apply::{apply_diff, spec_dirty, ApplyResult};
pub use flow::{execute, Deps};
pub use heal::{heal_file_name, invoke_heal, HealHint, HealJob, HealRecord};
pub use outcome::{call_error_text, format_call_error, format_line, outcomes_from, Outcome};

use std::time::Duration;

use serde_json::Value;

use crate::commands::Ctx;
use crate::daemon::mcp::{McpClient, McpError};
use crate::daemon::paths;
use crate::daemon::status::Endpoint;
use crate::processes::sgw_pids;

const LONG: &str = concat!(
    "Calls lab_uat_calibrate on the shared lab daemon: for each pose a client_world_click step of the row names (or only --pose), it sweeps stand-off radii, bearings and camera angles around the target, scores each by hover, confirms the winner three times, and writes the proposed pose line as a diff in a new UAT run folder (with calibrate.jsonl, one line per probe). The spec file itself is never written; nothing is committed.",
    "\n\n",
    "The client must already stand in the world where the row starts. --prepare runs those rows first (one lab_uat_run on the same instance); a prepare row that does not PASS stops with exit 3, naming it.",
    "\n\n",
    "Before anything is driven: the arguments must be valid, the daemon must answer, the instance (--instance, else the first free one) must be free, and no SGW.exe outside the lab may run.",
    "\n\n",
    "Output: one line per pose (frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff <path>), then the diff's text unless --quiet. --json prints only one compact JSON object (under about 400 characters, on failure too: {\"ok\":false,\"exit\":N,\"error\":...}): ok, section, row, and per pose pose, ok, score, confirmed, the changed fields and diff.",
    "\n\n",
    "--apply runs git apply with the diff in this checkout, unless the spec file has uncommitted changes or is not tracked (exit 2; the diff stays in the run folder, and the error starts with its path). It never stages or commits. --budget-minutes (1 to 30, default 15) bounds the whole sweep.",
    "\n\n",
    "Exit codes: 0 every pose has a proposal (or is unchanged); 1 a pose found no candidate; 2 usage, or --apply refused or failed; 3 the pre-flight, the prepare rows or the calibrate call failed.",
);

const EXAMPLES: &str = "Examples:\n  lab calibrate first-session FS-P3\n  lab calibrate first-session FS-P3 --pose frost --prepare FS-01,FS-P1\n  lab calibrate first-session FS-P3 --pose frost --budget-minutes 8 --apply\n  lab calibrate first-session FS-P3 --instance p2 --json";

// Arguments of `lab calibrate`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Recalibrate a row's poses by hover and propose a spec diff",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The UAT section
    pub section: Option<String>,
    /// The row to recalibrate, e.g. FS-P3
    pub row: Option<String>,
    /// Calibrate only this pose (default: every client_world_click pose of the row)
    #[arg(long)]
    pub pose: Option<String>,
    /// The lab instance (default: the first free one)
    #[arg(long)]
    pub instance: Option<String>,
    /// Rows to run first on the same instance (a prepare row that does not PASS stops with exit 3)
    #[arg(long, action = clap::ArgAction::Append, value_delimiter = ',')]
    pub prepare: Vec<String>,
    /// Bound the whole sweep, 1 to 30 (default 15)
    #[arg(long)]
    pub budget_minutes: Option<u32>,
    /// The UAT spec folder
    #[arg(long)]
    pub specs_dir: Option<String>,
    /// Run git apply with the proposed diff in this checkout (never stages or commits)
    #[arg(long)]
    pub apply: bool,
    /// Print one compact JSON object only
    #[arg(long)]
    pub json: bool,
    /// Do not print the diff's text
    #[arg(long)]
    pub quiet: bool,
}

/// Runs `lab calibrate` and returns its exit code: 0 every pose has a
/// proposal or is unchanged, 1 a pose found none, 2 usage or `--apply`
/// refused or failed, 3 the pre-flight, a prepare row or the call failed.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let ep = Endpoint::discover();
    let specs = paths::default_specs_dir();
    let repo = specs.as_deref().and_then(preflight::repo_root_from_specs);
    let specs = specs.map(|p| p.display().to_string());
    // One MCP session per call, closed after it (the PowerShell did the same).
    let call_ep = ep.clone();
    let mut call = move |name: &str, call_args: Value, wait: Duration| -> Result<Value, McpError> {
        let mut client = McpClient::connect(&call_ep)?;
        let reply = client.call_tool(name, call_args, wait);
        client.close();
        reply
    };
    let sgw = sgw_pids;
    let mut deps = Deps {
        ep: &ep,
        repo,
        specs,
        sgw_pids: &sgw,
        call: &mut call,
    };
    let mut out = anstream::stdout();
    let mut err = anstream::stderr();
    execute(args, &ctx.ui, &mut deps, &mut out, &mut err)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn calibrate(tokens: &[&str]) -> super::Args {
        let Command::Calibrate(args) = parse_ok("calibrate", tokens) else {
            panic!("not calibrate")
        };
        args
    }

    /// The four EXAMPLES lines parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = calibrate(&["first-session", "FS-P3"]);
        assert_eq!(a.section.as_deref(), Some("first-session"));
        assert_eq!(a.row.as_deref(), Some("FS-P3"));

        let a = calibrate(&[
            "first-session",
            "FS-P3",
            "-Pose",
            "frost",
            "-Prepare",
            "FS-01,FS-02,FS-P1,FS-P2",
        ]);
        assert_eq!(a.pose.as_deref(), Some("frost"));
        assert_eq!(a.prepare, ["FS-01", "FS-02", "FS-P1", "FS-P2"]);

        let a = calibrate(&[
            "first-session",
            "FS-P3",
            "-Pose",
            "frost",
            "-BudgetMinutes",
            "8",
            "-Apply",
        ]);
        assert_eq!(a.budget_minutes, Some(8));
        assert!(a.apply);

        let a = calibrate(&["first-session", "FS-P3", "-Instance", "p2", "-Json"]);
        assert_eq!(a.instance.as_deref(), Some("p2"));
        assert!(a.json);
    }

    #[test]
    fn defaults() {
        let a = calibrate(&["first-session", "FS-P3"]);
        assert_eq!(a.budget_minutes, None);
        assert!(a.prepare.is_empty());
        assert!(!a.apply);
        assert!(!a.quiet);
    }

    #[test]
    fn quiet_and_specs_dir() {
        let a = calibrate(&["first-session", "FS-P3", "-Quiet", "-SpecsDir", "C:\\s"]);
        assert!(a.quiet);
        assert_eq!(a.specs_dir.as_deref(), Some("C:\\s"));
    }
}
