//! `lab instances`: each instance's account, profile and client, or the extra
//! account files, by running `instances.ps1` with the same arguments (LX-18).

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Shows each lab instance's account, profile seed and client, or writes the extra account files. `lab instances status` prints one line per instance, and `lab instances init [--install-dir <dir>] [--count <n>] [--force]` writes the account files for p2..p<Count> (Count is 1 to 5, default 5). The runbook is docs/guides/live-research-lab.md.",
    "\n\n",
    "The arguments go to the instances script unchanged, so the PowerShell spellings (-InstallDir, -Count, -Force) work too.",
);

const EXAMPLES: &str = "Examples:\n  lab instances status\n  lab instances init\n  lab instances init --install-dir 'D:\\Games\\Stargate Worlds' --force";

/// Arguments of `lab instances`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Show each instance's account, profile and client, or write account files",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// status, or init [--install-dir <dir>] [--count <n>] [--force]
    #[arg(
        value_name = "ARGS",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    pub rest: Vec<String>,
}

/// Runs `lab instances`: `instances.ps1` with the same arguments.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "instances.ps1", &[])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn rest_of(command: Command) -> Vec<String> {
        match command {
            Command::Instances(args) => args.rest,
            other => panic!("not instances: {other:?}"),
        }
    }

    #[test]
    fn status_is_passed_through() {
        assert_eq!(rest_of(parse_ok("instances", &["status"])), ["status"]);
    }

    /// The trailing capture takes hyphen words, so the PowerShell spellings
    /// reach the script as the rewritten flags.
    #[test]
    fn init_keeps_its_flags_in_order() {
        assert_eq!(
            rest_of(parse_ok(
                "instances",
                &[
                    "init",
                    "-InstallDir",
                    "D:\\Games\\Stargate Worlds",
                    "-Force"
                ],
            )),
            [
                "init",
                "--install-dir",
                "D:\\Games\\Stargate Worlds",
                "--force"
            ]
        );
        assert_eq!(rest_of(parse_ok("instances", &["init"])), ["init"]);
    }
}
