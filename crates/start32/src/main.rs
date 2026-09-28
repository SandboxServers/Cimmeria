//! `sgw-start32`: start or attach to a 32-bit process and inject DLLs
//! into it, at the target's bitness. The whole contract (arguments,
//! the one-line stdout answer, exit codes) is `cimmeria_client_launch::start32`.
//!
//! Built for `i686-pc-windows-msvc`, embedded in the 64-bit launcher, and
//! run by it for every `SGW.exe` launch that loads DLLs. Off Windows it
//! only answers `unsupported`.

use std::process::ExitCode;

use cimmeria_client_launch::start32::{parse_args, ErrorKind, Outcome};

fn main() -> ExitCode {
    let outcome = match parse_args(std::env::args_os().skip(1)) {
        Ok(request) => carry_out(&request),
        Err(why) => Outcome::Failed {
            kind: ErrorKind::Usage,
            detail: why,
        },
    };
    println!("{}", outcome.format());
    ExitCode::from(outcome.exit_code() as u8)
}

#[cfg(windows)]
fn carry_out(request: &cimmeria_client_launch::start32::Request) -> Outcome {
    cimmeria_client_launch::start32::execute(request)
}

#[cfg(not(windows))]
fn carry_out(_request: &cimmeria_client_launch::start32::Request) -> Outcome {
    Outcome::Failed {
        kind: ErrorKind::Unsupported,
        detail: "sgw-start32 only runs on Windows".into(),
    }
}
