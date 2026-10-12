//! `lab start`: starts the lab daemon through its scheduled task. LX-02 gives
//! it its clap arguments and help page; the body is still the PowerShell
//! fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = "Starts the lab daemon through its scheduled task, CimmeriaLabDaemon, and passes the start script's exit code on.";

const EXAMPLES: &str = "Examples:\n  lab start";

// Arguments of `lab start`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Start the lab daemon (the CimmeriaLabDaemon scheduled task)",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {}

/// Runs `lab start` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// The `.EXAMPLE` line of the retired start.ps1 parses.
    #[test]
    fn examples_parse() {
        assert!(matches!(parse_ok("start", &[]), Command::Start(_)));
        assert!(try_parse("start", &["extra"]).is_err());
    }
}
