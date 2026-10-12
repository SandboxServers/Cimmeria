//! `lab watch`: watch a running lab uat batch, read-only.
//! LX-02 gave it its clap arguments and help page; LX-13 reads the batch
//! folder, `/status` and `labd.log` and prints one snapshot as text or JSON.

pub mod json;
pub mod process;
pub mod snapshot;
pub mod source;
pub mod text;

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::batch::{find_latest_batch, BATCH_JSONL, PLAN_JSON};
use crate::commands::Ctx;
use crate::daemon::mcp::hide_lease_ids;
use crate::daemon::paths;
use crate::daemon::status::{self, Endpoint};
use crate::ui::Ui;

use snapshot::{build, Inputs, Snapshot};

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

/// Runs `lab watch`: checks the arguments, takes one snapshot and prints it as
/// text or one JSON line. Exit 0 a snapshot was printed; 1 the daemon is down
/// and there is no batch; 2 usage; 3 the lab state could not be read. Under
/// `--json`, 2 and 3 print `{"ok":false,"exit":N,"error":...}` on stdout.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let as_json = args.json;
    // Usage checks come first, so they exit 2 whatever the daemon's state.
    if !(1..=60).contains(&args.interval_seconds) {
        return fail(
            as_json,
            2,
            &format!(
                "--interval-seconds must be 1 to 60 (got {})",
                args.interval_seconds
            ),
        );
    }
    // An empty value reads as not given, as `if ($Batch)` did in PowerShell.
    let given_or_none = |value: &Option<String>| value.clone().filter(|v| !v.is_empty());
    let args = &Args {
        batch: given_or_none(&args.batch),
        status_file: given_or_none(&args.status_file),
        log_file: given_or_none(&args.log_file),
        ..*args
    };
    let batch_dir = match args.batch.as_deref() {
        None => None,
        Some(given) => {
            let path = Path::new(given);
            if !path.is_dir() {
                return fail(as_json, 2, &format!("no batch folder: {given}"));
            }
            if !path.join(PLAN_JSON).exists() && !path.join(BATCH_JSONL).exists() {
                return fail(
                    as_json,
                    2,
                    &format!("not a lab uat batch (no plan.json or batch.jsonl): {given}"),
                );
            }
            Some(std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
        }
    };
    if let Some(file) = args.status_file.as_deref() {
        if !Path::new(file).is_file() {
            return fail(as_json, 2, &format!("no status file: {file}"));
        }
    }

    // LX-14: the live view goes here, when neither --once nor --json is given
    // and stdout is a terminal. Until then every call prints one snapshot.
    let snapshot = match take_snapshot(args, batch_dir) {
        Ok(snapshot) => snapshot,
        Err(why) => {
            let first = why.lines().next().unwrap_or("");
            return fail(
                as_json,
                3,
                &format!("could not read the lab state: {}", hide_lease_ids(first)),
            );
        }
    };
    let mut out = std::io::stdout().lock();
    if as_json {
        let _ = writeln!(out, "{}", json::render_json(&snapshot));
    } else {
        for line in text::render_text(&snapshot, frame_width(&ctx.ui)) {
            let _ = writeln!(out, "{line}");
        }
    }
    if !snapshot.daemon_up && snapshot.batch.is_none() {
        1
    } else {
        0
    }
}

/// A usage or read failure: plain text on stderr, or under `--json` one object
/// on stdout. Returns the exit code.
fn fail(as_json: bool, code: u8, message: &str) -> u8 {
    // A path the user typed may carry a lease id: nothing printed keeps one.
    let message = hide_lease_ids(message);
    let message = message.as_str();
    if as_json {
        let _ = writeln!(std::io::stdout(), "{}", json::error_line(code, message));
    } else {
        eprintln!("{message}");
    }
    code
}

/// The terminal's width less one (a full-width line wraps on some hosts), or
/// 120 when there is no terminal to measure (stdout redirected) or it is
/// narrower than 41.
fn frame_width(ui: &Ui) -> usize {
    match ui.width {
        Some(w) if w >= 41 => w - 1,
        _ => 120,
    }
}

/// Everything one snapshot reads: /status (the file, or the daemon's `GET
/// /status`), the batch folder (`--batch`, else the newest under the UAT
/// root), the labd.log tail, the SGW pids and the clock. The only daemon call
/// is `GET /status`. An error is a batch folder or labd.log that cannot be read.
fn take_snapshot(args: &Args, batch_dir: Option<PathBuf>) -> Result<Snapshot, String> {
    let status_body = match args.status_file.as_deref() {
        Some(file) => source::read_status_file(Path::new(file)),
        None => status::fetch_value(&Endpoint::discover()),
    };
    // The newest batch is looked up on every snapshot: a batch started after
    // the watch began replaces the one on screen.
    let dir = match batch_dir {
        Some(dir) => Some(dir),
        None => {
            let root = paths::uat_root();
            find_latest_batch(&root).map_err(|e| format!("{}: {e}", root.display()))?
        }
    };
    let log_path = args
        .log_file
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(paths::labd_log_path);
    let log_lines = if log_path.is_file() {
        source::read_tail_lines(&log_path, source::LOG_TAIL_LINES)
            .map_err(|e| format!("{}: {e}", log_path.display()))?
    } else {
        Vec::new()
    };
    let sgw = process::sgw_pids();
    build(&Inputs {
        status: status_body.as_ref(),
        batch_dir: dir.as_deref(),
        log_lines: &log_lines,
        sgw_pids: &sgw,
        now: Utc::now(),
        is_alive: &process::batch_owner_alive,
    })
    .map_err(|e| e.to_string())
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
