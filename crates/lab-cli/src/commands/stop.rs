//! `lab stop`: stops the lab daemon and closes its lab clients. LX-02 gives it
//! its clap arguments and help page; the body is still the PowerShell
//! fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Stops the lab daemon and closes its lab clients, then passes the stop script's exit code on.",
    "\n\n",
    "A later daemon does not adopt the clients this one launched, so the stop closes them once the daemon is down. While a session holds a lease it refuses, naming the holder (exit 3). --force closes the leased clients too.",
);

const EXAMPLES: &str = "Examples:\n  lab stop\n  lab stop --force";

// Arguments of `lab stop`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Stop the lab daemon and close its lab clients",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// Close the clients of leased instances too
    #[arg(long)]
    pub force: bool,
}

/// Runs `lab stop` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    /// Both `.EXAMPLE` lines of the retired stop.ps1 parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Stop(a) = parse_ok("stop", &[]) else {
            panic!("not stop")
        };
        assert!(!a.force);
        let Command::Stop(a) = parse_ok("stop", &["-Force"]) else {
            panic!("not stop")
        };
        assert!(a.force);
    }
}
