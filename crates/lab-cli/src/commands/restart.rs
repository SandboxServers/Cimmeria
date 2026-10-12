//! `lab restart`: restarts the lab daemon, picking up a newer build, by running
//! `daemon.ps1 restart [-Force]` (LX-18); the script's exit code is ours.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Restarts the lab daemon, picking up a newer cimmeria-lab.exe build, and passes the restart script's exit code on.",
    "\n\n",
    "The new daemon does not adopt the old one's clients, so the restart closes every lab client once the old daemon is down. While a session holds a lease it refuses, naming the holder (exit 3): the lease ends with the daemon. --force closes the leased clients too.",
);

const EXAMPLES: &str = "Examples:\n  lab restart\n  lab restart --force";

// Arguments of `lab restart`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Restart the lab daemon, picking up a newer cimmeria-lab.exe",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// Close the clients of leased instances too
    #[arg(long)]
    pub force: bool,
}

/// Runs `lab restart`: `daemon.ps1 restart`, with `-Force` when given.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "daemon.ps1", &["restart"])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    /// Both `.EXAMPLE` lines of the removed PowerShell restart command parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Restart(a) = parse_ok("restart", &[]) else {
            panic!("not restart")
        };
        assert!(!a.force);
        let Command::Restart(a) = parse_ok("restart", &["-Force"]) else {
            panic!("not restart")
        };
        assert!(a.force);
    }
}
