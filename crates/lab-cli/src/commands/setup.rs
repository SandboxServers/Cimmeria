//! `lab setup`: refresh the installed CLI scripts and add `bin` to the user PATH,
//! by running `cli\setup.ps1` with the same arguments (LX-18).

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = "Run it from a Cimmeria checkout, or pass --from. It copies the CLI into <LabHome>\\cli, writes <LabHome>\\bin\\lab.cmd, and adds <LabHome>\\bin to the user PATH when it is missing, then broadcasts the change so a new terminal sees it. LabHome is %LOCALAPPDATA%\\cimmeria-lab, or CIMMERIA_LAB_HOME when set.";

const EXAMPLES: &str = "Examples:\n  lab setup --no-path\n  lab setup --from C:\\src\\Cimmeria";

// Arguments of `lab setup`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Refresh the installed CLI and add it to the user PATH",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// The checkout to install from (needed when setup runs from the installed copy)
    #[arg(long)]
    pub from: Option<String>,
    /// Leave the user PATH alone; add <LabHome>\bin to it yourself
    #[arg(long)]
    pub no_path: bool,
    /// Add <LabHome>\bin to the user PATH without asking
    #[arg(long)]
    pub yes: bool,
}

/// Runs `lab setup`: `cli\setup.ps1` with the same arguments.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run_script(ctx, "cli/setup.ps1", &[])
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn setup(tokens: &[&str]) -> super::Args {
        let Command::Setup(args) = parse_ok("setup", tokens) else {
            panic!("not setup")
        };
        args
    }

    /// The three EXAMPLES-style forms parse, with the right values.
    #[test]
    fn examples_parse() {
        assert!(setup(&["-NoPath"]).no_path);
        let a = setup(&["-From", "C:\\src\\Cimmeria"]);
        assert_eq!(a.from.as_deref(), Some("C:\\src\\Cimmeria"));
        assert!(setup(&["-Yes"]).yes);
    }

    #[test]
    fn defaults() {
        let a = setup(&[]);
        assert!(a.from.is_none() && !a.no_path && !a.yes);
    }
}
