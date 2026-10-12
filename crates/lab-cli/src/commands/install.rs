//! `lab install`: build the lab from a worktree and install it, by running
//! `cli\install.ps1` with the same arguments (LX-18).

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "Builds the lab from a worktree through the PowerShell lane, installs it, and refreshes the CLI copy: runs <From>\\tools\\lab\\install.ps1 -Worktree <From>, with -SkipBuild when given and -InstallDir from labd.env's CIMMERIA_LAB_INSTALL_DIR when it is set. Then the installer copies the CLI from <From>, so the installed CLI matches the build.",
    "\n\n",
    "A lab build must come from a worktree you choose, so --from is required.",
);

const EXAMPLES: &str = "Examples:\n  lab install --from C:\\src\\Cimmeria\\.claude\\worktrees\\lab-fix\n  lab install --from C:\\src\\Cimmeria\\.claude\\worktrees\\lab-fix --skip-build";

/// Arguments of `lab install`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Build the lab from a worktree and install it",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The worktree to build from, e.g. C:\src\Cimmeria\.claude\worktrees\lab-fix (required)
    #[arg(long)]
    pub from: Option<String>,
    /// Install what the target dir already holds; build nothing
    #[arg(long)]
    pub skip_build: bool,
}

/// Runs `lab install`: `cli\install.ps1` with the same arguments.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "cli/install.ps1", &[])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn install(tokens: &[&str]) -> super::Args {
        let Command::Install(args) = parse_ok("install", tokens) else {
            panic!("not install")
        };
        args
    }

    /// The two EXAMPLES forms parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = install(&["-From", "C:\\src\\wt"]);
        assert_eq!(a.from.as_deref(), Some("C:\\src\\wt"));
        assert!(!a.skip_build);

        let a = install(&["-From", "C:\\src\\wt", "-SkipBuild"]);
        assert!(a.skip_build);
    }

    /// The PowerShell reports a missing `-From`, so clap must not.
    #[test]
    fn from_is_optional_to_clap() {
        assert_eq!(install(&[]).from, None);
    }
}
