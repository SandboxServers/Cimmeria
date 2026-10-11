//! `lab clients`: closes the game client of one lab instance, or of all of
//! them. LX-02 gives it its clap arguments and help page; the body is still
//! the PowerShell fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Usage: lab clients stop [<instance>|all] [--force].",
    "\n\n",
    "Asks each client window to close (CloseMainWindow) and force-stops it when it is still alive after 8 s. Only a process named SGW is touched, and the pid comes from the daemon's GET /status.",
    "\n\n",
    "An instance a session holds (a lease) is not stopped, because its holder would lose the client. --force overrides that, for one named instance only: 'all' with --force is refused. Exit codes: 0 when nothing was refused, 1 when the daemon is not running, 2 on a usage error, 3 when a leased client was refused, 4 when a client could not be stopped.",
);

const EXAMPLES: &str = "Examples:\n  lab clients stop p2\n  lab clients stop p2 --force";

// Arguments of `lab clients`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Close the game client of one lab instance, or of all of them",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// What to do; only 'stop' exists
    pub verb: Option<String>,
    /// An instance name, or 'all'
    pub which: Option<String>,
    /// Stop the client of a leased instance too (one named instance only; 'all' with --force is refused)
    #[arg(long)]
    pub force: bool,
}

/// Runs `lab clients` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    /// The `.EXAMPLE` lines of clients.ps1 parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Clients(a) = parse_ok("clients", &["stop", "p2"]) else {
            panic!("not clients")
        };
        assert_eq!(a.verb.as_deref(), Some("stop"));
        assert_eq!(a.which.as_deref(), Some("p2"));
        assert!(!a.force);
        let Command::Clients(a) = parse_ok("clients", &["stop", "p2", "-Force"]) else {
            panic!("not clients")
        };
        assert!(a.force);
    }
}
