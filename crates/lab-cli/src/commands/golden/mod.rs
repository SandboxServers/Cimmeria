//! `lab golden`: record or show a golden fingerprint.
//! `record` runs this program's own `lab uat` and then asks the daemon to
//! write the golden; `show` reads a committed one.

use crate::commands::Ctx;
use crate::daemon::paths;
use crate::daemon::status::Endpoint;

mod batch;
mod check;
mod exec;
mod reply;
mod summary;

pub use batch::{wait_for_child, ChildOutput};
pub use exec::{execute, Deps};

const LONG: &str = concat!(
    "record runs the rows --runs times in fingerprint mode through lab uat (serial on one instance by default; --leases up to the run count splits them over that many lanes), then records the section's golden from those runs with lab_golden_record on the shared lab daemon. A golden is recorded only from five or more fresh runs that all pass every row and agree exactly, so --runs is 5 to 20, and the lane brake is one failed run. --server-version names the server build the runs drove; the tool refuses runs without one. The golden goes to <specs>/golden/<section>.json (--specs-dir, default the checkout's docs/guides/uat-specs). An existing golden is replaced only with --rebless, from fresh runs and with every row it held.",
    "\n\n",
    "While recording, don't call lab_timeline on the lab character: it reads and drains the same packet tap, and a drained tap cannot be recorded.",
    "\n\n",
    "show prints a committed golden: section, rows, total ordered events, server build, recorded time and evidence run ids. It needs no daemon.",
    "\n\n",
    "record shows lab uat's per-run progress lines live on stderr.",
    "\n\n",
    "--json prints only one compact JSON object on stdout: {\"ok\":true,\"exit\":0,...} or {\"ok\":false,\"exit\":N,\"error\":...}, with \"batch\" (the runs' batch folder) on any failure after the runs. Progress still goes to stderr.",
    "\n\n",
    "Exit codes: 0 recorded (or shown); 1 a run or row did not pass; 2 usage, or a golden exists without --rebless (show: no golden); 3 the lab uat pre-flight or plan failed (nothing driven); 6 the runs disagree; 7 the runs cannot be recorded as given, the daemon failed after the runs, or (show) the golden cannot be read; 130 Ctrl+C stopped the runs (nothing is recorded; the first press lets the run under way finish, the second stops at once).",
);

const EXAMPLES: &str = "Examples:\n  lab golden record first-session --rows FS-P1 --server-version 1a2b3c4\n  lab golden record first-session --runs 10 --server-version 1a2b3c4 --json\n  lab golden record first-session --server-version 1a2b3c4 --rebless\n  lab golden show first-session";

// Arguments of `lab golden`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Record or show a golden fingerprint",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// record or show
    pub action: Option<String>,
    /// The UAT section
    pub section: Option<String>,
    /// Rows to run (record)
    #[arg(long, action = clap::ArgAction::Append, value_delimiter = ',')]
    pub rows: Vec<String>,
    /// Fresh runs to record from, 5 to 20
    #[arg(long, default_value_t = 5)]
    pub runs: u32,
    /// Lanes to split the runs over, up to the run count
    #[arg(long, default_value_t = 1)]
    pub leases: u32,
    /// The server build the runs drive (a git sha); record refuses runs without one
    #[arg(long)]
    pub server_version: Option<String>,
    /// The UAT spec folder (default: the checkout's docs/guides/uat-specs)
    #[arg(long)]
    pub specs_dir: Option<String>,
    /// Replace an existing golden, from fresh runs and with every row it held
    #[arg(long)]
    pub rebless: bool,
    /// Print one compact JSON object on stdout
    #[arg(long)]
    pub json: bool,
}

/// Runs `lab golden`: `record` drives this program's own `lab uat` and then
/// asks the daemon to write the golden; `show` reads a committed golden.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let deps = Deps {
        run_uat: &batch::spawn_uat,
        endpoint: Endpoint::discover(),
        default_specs: paths::default_specs_dir(),
    };
    let mut out = anstream::stdout();
    let mut err = anstream::stderr();
    execute(args, &ctx.ui, &deps, &mut out, &mut err)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn golden(tokens: &[&str]) -> super::Args {
        let Command::Golden(args) = parse_ok("golden", tokens) else {
            panic!("not golden")
        };
        args
    }

    /// The EXAMPLES lines parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = golden(&[
            "record",
            "first-session",
            "-Rows",
            "FS-01,FS-02,FS-P1,FS-P2,FS-P3,FS-P4,FS-P5",
            "-ServerVersion",
            "1a2b3c4",
        ]);
        assert_eq!(a.action.as_deref(), Some("record"));
        assert_eq!(a.section.as_deref(), Some("first-session"));
        assert_eq!(a.rows.len(), 7);
        assert_eq!(a.server_version.as_deref(), Some("1a2b3c4"));
        assert_eq!(a.runs, 5);

        let a = golden(&[
            "record",
            "first-session",
            "-Runs",
            "10",
            "-Leases",
            "2",
            "-ServerVersion",
            "1a2b3c4",
            "-Rebless",
            "-Json",
        ]);
        assert_eq!((a.runs, a.leases), (10, 2));
        assert!(a.rebless && a.json);

        let a = golden(&["show", "first-session"]);
        assert_eq!(a.action.as_deref(), Some("show"));
        assert_eq!(a.section.as_deref(), Some("first-session"));
    }

    #[test]
    fn defaults() {
        let a = golden(&["show", "first-session"]);
        assert_eq!(a.runs, 5);
        assert_eq!(a.leases, 1);
        assert!(a.rows.is_empty());
        assert!(!a.rebless);
    }

    #[test]
    fn specs_dir() {
        let a = golden(&["record", "s", "-SpecsDir", "C:\\s"]);
        assert_eq!(a.specs_dir.as_deref(), Some("C:\\s"));
    }
}
