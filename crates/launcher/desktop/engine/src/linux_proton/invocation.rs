//! The command lines that run Windows programs through a Proton build (C-3).
//!
//! Every invocation is bare: the host runs `<root>/proton` directly and never
//! wraps it in the runtime container's `_v2-entry-point` (D-PP3, PP-S1 ruling).
//! `ProtonInstall::runtime` is logged only and never reaches these functions.

use std::ffi::OsString;
use std::path::PathBuf;

use super::discovery::ProtonInstall;

/// A program and its arguments, ready for `Command::new` and `args`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// `<root>/proton`, the Proton script on the host.
    pub program: PathBuf,
    /// The arguments after the program.
    pub args: Vec<OsString>,
}

/// Runs `target` inside the prefix with Proton's environment (D-PP1):
/// `<root>/proton runinprefix <target> <extra...>`. Does not create the prefix.
pub fn runinprefix(install: &ProtonInstall, target: &str, extra: &[OsString]) -> Invocation {
    let mut args = vec![OsString::from("runinprefix"), OsString::from(target)];
    args.extend(extra.iter().cloned());
    Invocation {
        program: install.root.join("proton"),
        args,
    }
}

/// Creates or upgrades the prefix: `<root>/proton run wineboot.exe` (D-PP2).
pub fn setup(install: &ProtonInstall) -> Invocation {
    Invocation {
        program: install.root.join("proton"),
        args: vec![OsString::from("run"), OsString::from("wineboot.exe")],
    }
}

#[cfg(test)]
#[path = "invocation_tests.rs"]
mod tests;
