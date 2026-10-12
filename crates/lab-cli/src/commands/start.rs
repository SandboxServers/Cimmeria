//! `lab start`: starts the lab daemon through its scheduled task by running
//! `daemon.ps1 start` (LX-18); the script's exit code is ours.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = "Starts the lab daemon through its scheduled task, CimmeriaLabDaemon, and passes the start script's exit code on.";

const EXAMPLES: &str = "Examples:\n  lab start";

/// Arguments of `lab start`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Start the lab daemon (the CimmeriaLabDaemon scheduled task)",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {}

/// Runs `lab start`: `daemon.ps1 start`.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "daemon.ps1", &["start"])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// The `.EXAMPLE` line of the removed PowerShell start command parses.
    #[test]
    fn examples_parse() {
        assert!(matches!(parse_ok("start", &[]), Command::Start(_)));
        assert!(try_parse("start", &["extra"]).is_err());
    }
}
