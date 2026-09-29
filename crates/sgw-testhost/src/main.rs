//! `sgw-testhost`: a 32-bit stand-in for `SGW.exe`.
//!
//! The client DLLs (`cimmeria-client-telemetry`, `cimmeria-client-patches`)
//! are injected into this process by the tests in `tests/`. It imitates
//! the parts of the game a DLL touches at boot, and nothing else:
//!
//! - it is a 32-bit process, started suspended by `sgw-start32` like the
//!   game, with the DLLs loaded before its main thread runs;
//! - it stays alive and runs a main loop (16 ms frames) for as long as
//!   `--run-ms` says;
//! - it has none of the game's code at the hook addresses, so every DLL's
//!   fingerprint gate must fail closed here;
//! - it exits by returning from `main` with `--exit-code` (0 by default),
//!   so a test can tell a clean exit from a crash.
//!
//! When it exits it writes `sgw-testhost.out` next to itself:
//! `frames=<n> exit_code=<c>`. A DLL that froze the main thread shows as
//! too few frames.
//!
//! Usage: `sgw-testhost [--run-ms <ms>] [--exit-code <code>]`
//!
//! A GUI-subsystem executable, like `SGW.exe`, with no window. That also
//! matters to the harness: Windows gives a console-subsystem child a copy
//! of its parent's standard handles, so a console host would hold
//! `sgw-start32`'s stdout pipe open and the launcher's `start32::run`
//! would return only when the host exited.

#![windows_subsystem = "windows"]

use std::process::ExitCode;
use std::time::{Duration, Instant};

const FRAME: Duration = Duration::from_millis(16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Options {
    run: Duration,
    exit_code: u8,
}

fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut opts = Options {
        run: Duration::from_millis(3_000),
        exit_code: 0,
    };
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--run-ms" => {
                let v = value("--run-ms")?;
                opts.run =
                    Duration::from_millis(v.parse().map_err(|_| format!("bad --run-ms {v}"))?);
            }
            "--exit-code" => {
                let v = value("--exit-code")?;
                opts.exit_code = v.parse().map_err(|_| format!("bad --exit-code {v}"))?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(opts)
}

fn main() -> ExitCode {
    let opts = match parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("sgw-testhost: {e}");
            return ExitCode::from(2);
        }
    };

    // The fake main loop. Nothing here knows about the DLLs: they run on
    // their own threads, as they do in the game.
    let started = Instant::now();
    let mut frames: u64 = 0;
    while started.elapsed() < opts.run {
        std::thread::sleep(FRAME);
        frames += 1;
    }

    let exe = std::env::current_exe().ok();
    if let Some(dir) = exe.as_deref().and_then(std::path::Path::parent) {
        let _ = std::fs::write(
            dir.join("sgw-testhost.out"),
            format!("frames={frames} exit_code={}\n", opts.exit_code),
        );
    }
    ExitCode::from(opts.exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults() {
        assert_eq!(
            parse(args(&[])).unwrap(),
            Options {
                run: Duration::from_millis(3_000),
                exit_code: 0
            }
        );
    }

    #[test]
    fn run_and_exit_code() {
        let o = parse(args(&["--run-ms", "250", "--exit-code", "7"])).unwrap();
        assert_eq!(o.run, Duration::from_millis(250));
        assert_eq!(o.exit_code, 7);
    }

    #[test]
    fn bad_arguments_are_refused() {
        assert!(parse(args(&["--run-ms"])).is_err());
        assert!(parse(args(&["--run-ms", "soon"])).is_err());
        assert!(parse(args(&["--exit-code", "300"])).is_err());
        assert!(parse(args(&["--frobnicate"])).is_err());
    }
}
