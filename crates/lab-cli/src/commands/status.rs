//! `lab status`: the daemon and each lab instance, from the daemon's status
//! endpoint. LX-02 gives it its clap arguments and help page; the body is
//! still the PowerShell fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Reads the daemon's GET /status and prints the daemon and each lab instance: account, client, port, lease, seed.",
    "\n\n",
    "When the daemon or its endpoint is down, prints the instances named by labd.env's CIMMERIA_LAB_INSTANCES with the SEED column only, and exits 1.",
    "\n\n",
    "SEED is 'seeded' when the instance's profile folder exists under the profile root, else '-'.",
);

const EXAMPLES: &str = "Examples:\n  lab status";

// Arguments of `lab status`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show the daemon and each instance: account, client, port, lease, seed",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {}

/// Runs `lab status` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, try_parse, Command};

    /// The `.EXAMPLE` line of status.ps1 parses.
    #[test]
    fn examples_parse() {
        assert!(matches!(parse_ok("status", &[]), Command::Status(_)));
        assert!(try_parse("status", &["extra"]).is_err());
    }
}
