//! `lab logs`: show the daemon log, filtered by instance or level.
//! LX-02 gives it its clap arguments and help page; the body is still the
//! PowerShell fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Shows the last lines of the lab daemon's log, filtered by instance or level, with tokens masked. Reads labd.log under the lab home.",
    "\n\n",
    "--instance keeps lines that name the instance (instance=\"x\" or instance=Some(\"x\"), ignoring case). --level keeps lines whose level token, the second whitespace field (INFO, WARN, ...), is at or above the level. A line that does not start with a timestamp (a panic body, a backtrace) is kept or dropped with the entry above it.",
    "\n\n",
    "A bearer token or a 64-hex token prints as <redacted>.",
    "\n\n",
    "--follow streams the file through the same filter, so it can show fewer than --lines lines at first.",
);

const EXAMPLES: &str =
    "Examples:\n  lab logs --lines 5\n  lab logs --instance p2 --level warn --follow";

// Arguments of `lab logs`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show the daemon log, filtered by instance or level",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// How many log lines to show
    #[arg(long, default_value_t = 50)]
    pub lines: u32,
    /// Keep streaming new lines (through the same filter)
    #[arg(long)]
    pub follow: bool,
    /// Keep only lines that name this instance
    #[arg(long)]
    pub instance: Option<String>,
    /// Keep lines at or above this level
    #[arg(long, ignore_case = true, value_parser = ["trace", "debug", "info", "warn", "error"])]
    pub level: Option<String>,
}

/// Runs `lab logs` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// Both `.EXAMPLE` lines of logs.ps1 parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Logs(a) = parse_ok("logs", &["-Lines", "5"]) else {
            panic!("not logs")
        };
        assert_eq!((a.lines, a.follow), (5, false));
        let Command::Logs(a) = parse_ok("logs", &["-Instance", "p2", "-Level", "warn", "-Follow"])
        else {
            panic!("not logs")
        };
        assert_eq!(a.instance.as_deref(), Some("p2"));
        assert_eq!(a.level.as_deref(), Some("warn"));
        assert!(a.follow);
    }

    #[test]
    fn defaults_and_bad_level() {
        let Command::Logs(a) = parse_ok("logs", &[]) else {
            panic!("not logs")
        };
        assert_eq!(a.lines, 50);
        assert!(try_parse("logs", &["-Level", "loud"]).is_err());
    }
}
