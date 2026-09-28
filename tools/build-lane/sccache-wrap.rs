//! `RUSTC_WRAPPER` shim for the build lane (`tools/build-lane/lane.sh`, issue #1023).
//!
//! sccache hashes every `CARGO_*` environment variable into a Rust cache key (except
//! `CARGO_MAKEFLAGS`, `CARGO_REGISTRIES_*`, `CARGO_BUILD_JOBS` and
//! `CARGO_ENCODED_RUSTFLAGS`). The lane gives each worktree its own target dir through
//! `CARGO_TARGET_DIR`, so without this shim every worktree got its own key for the same
//! third-party crate, and the shared cache scored close to 0% hits.
//!
//! This shim runs sccache with the target-dir variables removed from its environment.
//! Cargo has already read them by the time it spawns the wrapper: rustc gets every path it
//! needs on the command line (`--out-dir`, `-L`, `--extern`), and sccache leaves those
//! arguments out of the key. What stays in the key keeps unsafe replays out: a crate that
//! reads `OUT_DIR` at compile time (`include!(concat!(env!("OUT_DIR"), ...))`) lists it in
//! its dep-info, sccache hashes the value, and the value names the worktree's target dir.
//!
//! The lane compiles this file with plain `rustc` on first use, into
//! `$LANE_ROOT/bin/sccache-wrap/<source hash>/sccache[.exe]`. The file is named `sccache`
//! so that cc-rs, which routes C compiles through `RUSTC_WRAPPER` only when its file stem
//! is `sccache`, keeps doing so. The real sccache is `CIMMERIA_SCCACHE_REAL`.

use std::env;
use std::process::{exit, Command};

/// The variables that name a target or build dir.
const HIDDEN: [&str; 3] = [
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET_DIR",
    "CARGO_BUILD_BUILD_DIR",
];

fn main() {
    let Some(sccache) = env::var_os("CIMMERIA_SCCACHE_REAL") else {
        eprintln!(
            "sccache-wrap: CIMMERIA_SCCACHE_REAL is not set; run cargo through tools/build-lane/lane.sh"
        );
        exit(2);
    };
    let mut cmd = Command::new(&sccache);
    cmd.args(env::args_os().skip(1));
    for var in HIDDEN {
        cmd.env_remove(var);
    }
    match cmd.status() {
        // A process killed by a signal (Unix) has no code; report a plain failure.
        Ok(status) => exit(status.code().unwrap_or(1)),
        Err(err) => {
            eprintln!(
                "sccache-wrap: cannot run {}: {err}",
                sccache.to_string_lossy()
            );
            exit(2);
        }
    }
}
