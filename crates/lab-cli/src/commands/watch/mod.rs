//! `lab watch`: watch a running lab uat batch, read-only.
//! LX-02 gives it its clap arguments and help page; the body is still the
//! PowerShell fallback.

pub mod snapshot;

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Shows the newest lab uat batch (the batch-* folder under the UAT root with the greatest name), or the --batch folder, redrawn every --interval-seconds: the batch line (its name, section, state: live, done, interrupted or unknown; runs done of planned, passed, failed and how many failures match a known issue; then the last failure and its issue tag); one row per lab instance from the daemon's /status (per plan lane when the daemon is down): CLIENT pid, LEASE owner and purpose, RUN, ROW (the row under way and its seconds), SHOT (the age of the run's newest screenshot), and under it the lane's last progress line; and a footer: SGW.exe processes no instance owns, client relaunches and login retries in labd.log since the batch started.",
    "\n\n",
    "Read-only: it takes no lease, asks the daemon only for GET /status, and every string it prints has its lease ids hidden.",
    "\n\n",
    "--once prints one snapshot and exits; --json prints it as one JSON line and implies --once. With output redirected, the live view prints one --once snapshot instead of looping. The live view uses the terminal's alternate screen; Ctrl+C ends it and restores the screen.",
    "\n\n",
    "Exit codes: 0 a snapshot was printed, or the live view ended with Ctrl+C; 1 the daemon is down and there is no batch to show; 2 usage (an --interval-seconds outside 1 to 60, a --batch folder with neither plan.json nor batch.jsonl); 3 a one-snapshot read failed (a locked or denied batch folder or labd.log). Under --json exits 2 and 3 print {\"ok\":false,\"exit\":N,\"error\":...}.",
    "\n\n",
    "Limits: with stdin redirected, Ctrl+C is the shell's own and the exit is not 0 (the screen is still restored); Ctrl+Break or closing the window skips the restore.",
);

const EXAMPLES: &str = "Examples:\n  lab watch\n  lab watch --once\n  lab watch --once --json\n  lab watch --batch C:\\uat-runs\\batch-20261011-013906 --once";

// Arguments of `lab watch`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Watch a running lab uat batch, read-only",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// A batch folder to show (default: the newest batch-* folder under the UAT root)
    #[arg(long)]
    pub batch: Option<String>,
    /// Print one snapshot and exit
    #[arg(long)]
    pub once: bool,
    /// Print one snapshot as one JSON line (implies --once)
    #[arg(long)]
    pub json: bool,
    /// Seconds between redraws, 1 to 60
    #[arg(long, default_value_t = 2)]
    pub interval_seconds: u32,
    /// Test seam: read the /status JSON from this file instead of the daemon
    #[arg(long, hide = true)]
    pub status_file: Option<String>,
    /// Test seam: read this file instead of labd.log
    #[arg(long, hide = true)]
    pub log_file: Option<String>,
}

/// Runs `lab watch` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use crate::commands::{parse_ok, Cli, Command};

    fn watch(tokens: &[&str]) -> super::Args {
        let Command::Watch(args) = parse_ok("watch", tokens) else {
            panic!("not watch")
        };
        args
    }

    /// The four EXAMPLES forms parse, with the right values.
    #[test]
    fn examples_parse() {
        watch(&[]);
        assert!(watch(&["-Once"]).once);
        let a = watch(&["-Once", "-Json"]);
        assert!(a.once && a.json);
        let a = watch(&[
            "-Batch",
            "C:\\lab\\uat-runs\\batch-20261011-013906",
            "-Once",
        ]);
        assert_eq!(
            a.batch.as_deref(),
            Some("C:\\lab\\uat-runs\\batch-20261011-013906")
        );
        assert!(a.once);
    }

    #[test]
    fn defaults_and_seams() {
        assert_eq!(watch(&[]).interval_seconds, 2);
        let a = watch(&[
            "-StatusFile",
            "s.json",
            "-LogFile",
            "l.log",
            "-IntervalSeconds",
            "5",
        ]);
        assert_eq!(a.status_file.as_deref(), Some("s.json"));
        assert_eq!(a.log_file.as_deref(), Some("l.log"));
        assert_eq!(a.interval_seconds, 5);
    }

    #[test]
    fn seams_are_hidden() {
        let cmd = Cli::command();
        let sub = cmd.find_subcommand("watch").expect("a watch subcommand");
        for id in ["status_file", "log_file"] {
            let arg = sub
                .get_arguments()
                .find(|a| a.get_id() == id)
                .expect("the seam argument");
            assert!(arg.is_hide_set(), "{id} should be hidden");
        }
    }
}
