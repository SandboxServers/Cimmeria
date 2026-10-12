//! `lab uat`: run a UAT spec on the lab with no agent.
//!
//! LX-15 is the front half: [`args`] (the argument checks), [`plan`] (lanes,
//! the plan call and `plan.json`) and [`preflight`] (the order of events before
//! anything is driven, and `--plan-only`). LX-16 is the back half: [`record`],
//! [`summary`], [`progress`], [`lanes`], [`ctrlc`] and [`batch_run`] run a real
//! batch. Only `--json`, `--diff-golden`, `--heal` and `--chaos` still use the
//! PowerShell fallback (until LX-17).

pub mod args;
pub mod batch_run;
pub mod chaos;
pub mod ctrlc;
pub mod golden;
pub mod json;
pub mod lanes;
pub mod plan;
pub mod preflight;
pub mod progress;
pub mod record;
pub mod summary;

use std::time::Duration;

use serde_json::Value;

use crate::commands::doctor::system::sgw_pids;
use crate::commands::Ctx;
use crate::daemon::env_file;
use crate::daemon::mcp::{McpClient, McpError};
use crate::daemon::paths;
use crate::daemon::status::Endpoint;
use crate::fallback;

pub use preflight::{execute, Deps, Outcome, Ready};

const LONG: &str = concat!(
    "Calls lab_uat_run on the shared lab daemon. --leases lanes run at once, one per hosted lab instance in the daemon's order (default, p2, ...); each lane runs the rows --runs-per-lease times, one after another. Every run takes and releases its own lease, so nothing stays held. Lanes start --stagger-seconds apart (run ids are time based, and simultaneous client boots are where logins go wrong).",
    "\n\n",
    "Before anything is driven: the arguments must be valid, the daemon must answer, the chosen instances must be free, no SGW.exe outside the lab may run, and a plan-only pass of the rows must be ready (every requested row found). A lane stops after --max-consecutive-failures failed runs in a row (0 never stops); the other lanes go on. A run that times out is followed by a wait for its instance's lease to clear, so the next run does not collide with it; a lease that never clears ends the lane.",
    "\n\n",
    "Output: one progress line per run, then a summary (pass rate per row and per lane, failures grouped by their first failing row, known issues tagged). --json prints only one compact JSON object (on failure too: {\"ok\":false,\"exit\":N,\"error\":...}), for agents.",
    "\n\n",
    "Every batch writes plan.json first, then batch.jsonl (one line per run, as each finishes), then batch.json and summary.md, under uat-runs\\batch-<time>\\ next to the runs' own evidence folders. Ctrl+C starts no new run and still writes the summary of the runs that finished; a run under way finishes on the daemon and releases its lease.",
    "\n\n",
    "--fixtures auto|always|never sets when a row's [row.fixture] is applied (the daemon's default is auto: a fixture row continues a passing chain and starts from its fixture otherwise).",
    "\n\n",
    "--chaos <profile[:level][:knobs]> runs every row under chaos specs: cpu (the lab machine), net and flush (the row's server session; a lab-chaos server build). burst is not wired yet and is refused. A bad spec fails the plan call (exit 3) before anything is driven. cpu loads the whole machine, so cpu needs --leases 1 (exit 2 otherwise). A multi-knob spec contains commas, so quote it: --chaos 'net:latency=120,loss=1'. --chaos implies a fingerprint and --diff-golden, so it needs the section's golden: exit 2 without one in the local spec folder, before the daemon is asked; --plan-only skips that check. A chaos run's comparison is reported as chaos_divergence, in the same shape as golden; a divergence alone does not exit 5. A chaos run never becomes a golden.",
    "\n\n",
    "--diff-golden compares every passing run with the section's committed golden (<specs>/golden/<section>.json, made by lab golden record): each gets a golden value: match, the first divergence as one line, or error: <why>. A failed run is not compared. Rows' verdicts do not change. Without a golden, or with one that cannot be read, it stops before anything is driven (exit 2).",
    "\n\n",
    "--heal (needs --leases 1) recalibrates the poses failed rows missed on: after the batch, for each distinct (row, pose) whose row summary carries a heal hint, it calls lab_uat_calibrate for that pose (up to its 15-minute budget each), saves a proposed diff as heal-<row>-<pose>.diff in the batch folder and prints it. It never writes the spec file and never commits: review the diff, then git apply it (or run lab calibrate <section> <row> --pose <pose> --apply).",
    "\n\n",
    "Exit codes: 0 every run passed; 1 a run failed; 2 usage; 3 the pre-flight or plan failed (nothing was driven); 4 a lane stopped early on its brake; 5 every run passed but --diff-golden did not match on one (a divergence, or a run it could not compare; under --chaos only the latter); 8 a heal proposal was written, which wins over every other code, 5 included.",
);

const EXAMPLES: &str = "Examples:\n  lab uat first-session --rows FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5\n  lab uat first-session --rows FS-P1,FS-P2 --leases 2 --runs-per-lease 5\n  lab uat first-session --instance p2 --plan-only\n  lab uat first-session --rows FS-P4 --runs-per-lease 20\n  lab uat first-session --rows FS-P5,FS-P3,FS-P4 --fixtures always\n  lab uat first-session --rows FS-01,FS-02,FS-P1 --diff-golden\n  lab uat first-session --leases 3 --runs-per-lease 10 --json\n  lab uat first-session --rows FS-01,FS-02,FS-P1,FS-P2,FS-P3 --heal";

// Arguments of `lab uat`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Run a UAT spec on the lab with no agent",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The UAT section, e.g. first-session
    pub section: Option<String>,
    /// Rows to run, e.g. FS-01,FS-02 (default: every row of the section)
    #[arg(long, action = clap::ArgAction::Append, value_delimiter = ',')]
    pub rows: Vec<String>,
    /// Lanes to run at once, 1 to 5; one per hosted lab instance
    #[arg(long, default_value_t = 1)]
    pub leases: u32,
    /// Runs each lane does one after another, 1 to 20
    #[arg(long, default_value_t = 1)]
    pub runs_per_lease: u32,
    /// Run one lane on this instance (needs --leases 1)
    #[arg(long)]
    pub instance: Option<String>,
    /// Plan the rows and drive nothing
    #[arg(long)]
    pub plan_only: bool,
    /// The UAT spec folder (default: the checkout's, else the daemon's)
    #[arg(long)]
    pub specs_dir: Option<String>,
    /// The server build the runs drive (a git sha)
    #[arg(long)]
    pub server_version: Option<String>,
    /// Seconds between lane starts
    #[arg(long, default_value_t = 20)]
    pub stagger_seconds: u32,
    /// Minutes before one run times out
    #[arg(long, default_value_t = 15)]
    pub run_timeout_minutes: u32,
    /// Failed runs in a row that stop a lane (0 never stops)
    #[arg(long, default_value_t = 3)]
    pub max_consecutive_failures: u32,
    /// When a row's fixture is applied: auto, always or never
    #[arg(long, value_parser = ["auto", "always", "never"])]
    pub fixtures: Option<String>,
    /// Run every row under a chaos spec profile[:level][:knobs]; repeat the flag for several, and quote a spec that has commas
    #[arg(long, action = clap::ArgAction::Append)]
    pub chaos: Vec<String>,
    /// Compare every passing run with the section's committed golden
    #[arg(long)]
    pub diff_golden: bool,
    /// Recalibrate the poses failed rows missed on (needs --leases 1)
    #[arg(long)]
    pub heal: bool,
    /// Capture what a golden fingerprint needs (used by lab golden)
    #[arg(long, hide = true)]
    pub fingerprint: bool,
    /// Write progress lines to stderr, even with --json (used by lab golden)
    #[arg(long, hide = true)]
    pub progress_to_stderr: bool,
    /// Print one compact JSON object only
    #[arg(long)]
    pub json: bool,
    /// Print neither the progress lines nor the summary text
    #[arg(long)]
    pub quiet: bool,
}

/// The native pre-flight on real collaborators. Returns the daemon endpoint (a
/// native batch drives on it) with how the pre-flight ended.
fn native_preflight(args: &Args, ctx: &Ctx) -> (Endpoint, Outcome) {
    let ep = Endpoint::discover();
    let specs = paths::default_specs_dir().map(|p| p.display().to_string());
    let max_clients =
        preflight::max_clients_from(&env_file::read_or_empty(&paths::labd_env_path()));
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
        specs,
        max_clients,
        sgw_pids: &sgw,
        call: &mut call,
    };
    let mut out = anstream::stdout();
    let mut err = anstream::stderr();
    let outcome = execute(args, &ctx.ui, &mut deps, &mut out, &mut err);
    (ep, outcome)
}

/// Runs `lab uat` and returns its exit code. `--plan-only` and every refusal
/// before anything is driven are native (LX-15). A real run drives natively
/// unless it needs a flag the Rust code does not do yet (`--json`,
/// `--diff-golden`, `--heal`, `--chaos`): then the PowerShell script does the
/// whole run, its own pre-flight included, so nothing is planned twice.
/// LX-17b replaces this function and removes the fallback.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    if !args.plan_only && batch_run::needs_powershell(args) {
        return fallback::run(ctx);
    }
    let (ep, outcome) = native_preflight(args, ctx);
    match outcome {
        Outcome::Done(code) => code,
        Outcome::Drive(ready) => {
            batch_run::execute(args, ctx, &ep, batch_run::BatchInput::from_ready(ready))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    use super::Args;

    const ROWS7: &str = "FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5";

    fn uat(tokens: &[&str]) -> Args {
        let Command::Uat(args) = parse_ok("uat", tokens) else {
            panic!("not uat")
        };
        args
    }

    /// The eight EXAMPLES lines parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = uat(&["first-session", "-Rows", ROWS7]);
        assert_eq!(a.rows.len(), 7);
        assert_eq!(a.rows[0], "FS-01");
        assert_eq!(a.section.as_deref(), Some("first-session"));

        let a = uat(&[
            "first-session",
            "-Rows",
            ROWS7,
            "-Leases",
            "2",
            "-RunsPerLease",
            "5",
        ]);
        assert_eq!((a.leases, a.runs_per_lease), (2, 5));

        let a = uat(&["first-session", "-Instance", "p2", "-PlanOnly"]);
        assert_eq!(a.instance.as_deref(), Some("p2"));
        assert!(a.plan_only);

        let a = uat(&["first-session", "-Rows", "FS-P4", "-RunsPerLease", "20"]);
        assert_eq!(a.runs_per_lease, 20);

        let a = uat(&[
            "first-session",
            "-Rows",
            "FS-P5,FS-P3,FS-P4",
            "-Fixtures",
            "always",
        ]);
        assert_eq!(a.fixtures.as_deref(), Some("always"));

        let a = uat(&["first-session", "-Rows", ROWS7, "-DiffGolden"]);
        assert!(a.diff_golden);

        let a = uat(&[
            "first-session",
            "-Leases",
            "3",
            "-RunsPerLease",
            "10",
            "-Json",
        ]);
        assert!(a.json);
        assert_eq!(a.leases, 3);

        let a = uat(&[
            "first-session",
            "-Rows",
            "FS-01,FS-02,FS-P1,FS-P2,FS-P3",
            "-Heal",
        ]);
        assert!(a.heal);
    }

    #[test]
    fn defaults() {
        let a = uat(&["first-session"]);
        assert_eq!(a.leases, 1);
        assert_eq!(a.runs_per_lease, 1);
        assert_eq!(a.stagger_seconds, 20);
        assert_eq!(a.run_timeout_minutes, 15);
        assert_eq!(a.max_consecutive_failures, 3);
        assert!(a.rows.is_empty());
        assert!(a.chaos.is_empty());
        assert_eq!(a.fixtures, None);
    }

    #[test]
    fn rows_take_one_comma_list_per_flag() {
        let a = uat(&["first-session", "-Rows", "FS-01,FS-02"]);
        assert_eq!(a.rows, ["FS-01", "FS-02"]);
        let a = uat(&["first-session", "--rows", "FS-01", "--rows", "FS-02"]);
        assert_eq!(a.rows, ["FS-01", "FS-02"]);
    }

    #[test]
    fn list_flag_does_not_swallow_the_section() {
        let a = uat(&["--chaos", "cpu", "first-session", "--plan-only"]);
        assert_eq!(a.section.as_deref(), Some("first-session"));
        assert_eq!(a.chaos, ["cpu"]);
        assert!(a.plan_only);
    }

    #[test]
    fn chaos_keeps_a_quoted_multi_knob_spec_whole() {
        let a = uat(&[
            "first-session",
            "-Chaos",
            "net:latency=120,loss=1",
            "-Chaos",
            "cpu",
        ]);
        assert_eq!(a.chaos, ["net:latency=120,loss=1", "cpu"]);
    }

    #[test]
    fn kebab_flags() {
        let a = uat(&[
            "first-session",
            "-StaggerSeconds",
            "0",
            "-RunTimeoutMinutes",
            "5",
            "-MaxConsecutiveFailures",
            "0",
            "-SpecsDir",
            "C:\\specs",
            "-ServerVersion",
            "1a2b3c4",
            "-Quiet",
            "-Fingerprint",
            "-ProgressToStderr",
        ]);
        assert_eq!(a.stagger_seconds, 0);
        assert_eq!(a.run_timeout_minutes, 5);
        assert_eq!(a.max_consecutive_failures, 0);
        assert_eq!(a.specs_dir.as_deref(), Some("C:\\specs"));
        assert_eq!(a.server_version.as_deref(), Some("1a2b3c4"));
        assert!(a.quiet);
        assert!(a.fingerprint);
        assert!(a.progress_to_stderr);
    }

    #[test]
    fn bad_fixtures_value_is_rejected() {
        assert!(try_parse("uat", &["first-session", "-Fixtures", "sometimes"]).is_err());
    }
}
