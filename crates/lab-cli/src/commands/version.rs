//! `lab version`: the CLI's version and, once the daemon client lands, the daemon's.

/// Prints `lab <version> (<git sha or "dev">)` and returns exit code 0. The sha
/// is set by `lab install`.
pub fn run() -> u8 {
    let sha = option_env!("CIMMERIA_LAB_CLI_SHA").unwrap_or("dev");
    anstream::println!("lab {} ({sha})", env!("CARGO_PKG_VERSION"));
    // LX-06: daemon version
    0
}
