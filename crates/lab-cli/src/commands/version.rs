//! `lab version`: the CLI's version and, once the daemon client lands, the daemon's.

use std::process::ExitCode;

/// Prints `lab <version> (<git sha or "dev">)`. The sha is set by `lab install`.
pub fn run() -> ExitCode {
    let sha = option_env!("CIMMERIA_LAB_CLI_SHA").unwrap_or("dev");
    anstream::println!("lab {} ({sha})", env!("CARGO_PKG_VERSION"));
    // LX-06: daemon version
    ExitCode::SUCCESS
}
