//! `lab doctor`: checks the install, the daemon and each instance. LX-02 gives
//! it its clap arguments and help page; the body is still the PowerShell
//! fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = "Read-only. Prints `PASS|WARN|FAIL  <check>  <detail>` for each check and exits 1 when any check fails (WARN does not fail). The token is only ever reported as set or not set.";

const EXAMPLES: &str = "Examples:\n  lab doctor";

// Arguments of `lab doctor`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Check the install, the daemon and each instance",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {}

/// Runs `lab doctor` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    /// The `.EXAMPLE` line of doctor.ps1 parses.
    #[test]
    fn examples_parse() {
        assert!(matches!(parse_ok("doctor", &[]), Command::Doctor(_)));
    }
}
