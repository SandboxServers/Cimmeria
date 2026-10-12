//! `lab doctor`: check the install, the daemon and each instance.
//! `checks.rs` holds the ten pure checks, `system.rs` reads the machine.

pub mod checks;
pub mod system;

use checks::{Check, Status};

use crate::commands::Ctx;
use crate::ui::{Mark, Tone, Ui};

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

/// Runs `lab doctor`: gathers the facts, runs the ten checks, prints them and
/// returns 1 when any check failed (a warning does not fail).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    let results = checks::run_all(&system::gather());
    anstream::print!("{}", render(&ctx.ui, &results));
    exit_code(&results)
}

/// 1 when any check is a FAIL, else 0.
pub fn exit_code(results: &[Check]) -> u8 {
    u8::from(results.iter().any(|check| check.status == Status::Fail))
}

/// The text `lab doctor` prints.
///
/// Plain mode (what the PowerShell printed): one line per check,
/// `<PASS|WARN|FAIL>  <check>  <detail>` with two spaces between the parts
/// and the hint after `; ` in the detail, and nothing else.
///
/// Fancy mode: a glyph and the check name; under a warning or failure the
/// detail, then the hint (a command), each indented four spaces and dim; then
/// a count line, `N check(s) · P passed · W warning(s) · F failed`.
pub fn render(ui: &Ui, results: &[Check]) -> String {
    let mut out = String::new();
    for check in results {
        if !ui.fancy {
            out.push_str(&format!(
                "{}  {}  {}\n",
                check.status.label(),
                check.name,
                check.plain_detail()
            ));
            continue;
        }
        let (mark, tone) = match check.status {
            Status::Pass => (Mark::Pass, Tone::Ok),
            Status::Warn => (Mark::Warn, Tone::Warn),
            Status::Fail => (Mark::Fail, Tone::Fail),
        };
        out.push_str(&format!(
            "{} {}\n",
            ui.paint(tone, ui.mark(mark)),
            check.name
        ));
        if check.status != Status::Pass {
            out.push_str(&format!("    {}\n", ui.paint(Tone::Dim, &check.detail)));
            if let Some(hint) = &check.fancy_hint {
                out.push_str(&format!(
                    "    {}\n",
                    ui.paint(Tone::Dim, &format!("hint: {hint}"))
                ));
            }
        }
    }
    if ui.fancy {
        let count = |status: Status| results.iter().filter(|c| c.status == status).count();
        let warnings = count(Status::Warn);
        let noun = if warnings == 1 { "warning" } else { "warnings" };
        let total = results.len();
        let checks = if total == 1 { "check" } else { "checks" };
        let line = format!(
            "{total} {checks} · {} passed · {warnings} {noun} · {} failed",
            count(Status::Pass),
            count(Status::Fail)
        );
        out.push_str(&format!("{}\n", ui.paint(Tone::Dim, &line)));
    }
    out
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
