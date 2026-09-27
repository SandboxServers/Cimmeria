//! Bakes the git commit into the binary as `CIMMERIA_BUILD_SHA`, which
//! `otel.rs` exports as the `service.version` resource attribute so a
//! SigNoz row can be tied to the build that produced it (audit gap T2).
//!
//! Source, in order:
//! 1. `CIMMERIA_GIT_SHA` from the environment. The container build sets it
//!    from a build arg (`docker/Dockerfile`), because `.git` is not in the
//!    Docker build context.
//! 2. `git rev-parse HEAD` for a normal checkout or worktree.
//! 3. `"unknown"`. Never fails the build.

use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CIMMERIA_GIT_SHA");

    let sha = std::env::var("CIMMERIA_GIT_SHA")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "unknown")
        .or_else(sha_from_git)
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=CIMMERIA_BUILD_SHA={sha}");
}

fn sha_from_git() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if sha.is_empty() {
        return None;
    }

    // Re-run when HEAD moves. In a worktree the per-worktree git dir holds
    // `HEAD` (branch switches) and `logs/HEAD` (appended on every commit,
    // checkout and reset). Only name files that exist: a missing
    // rerun-if-changed path makes cargo re-run the script on every build.
    if let Ok(o) = Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .output()
    {
        if o.status.success() {
            if let Ok(dir) = String::from_utf8(o.stdout) {
                let dir = Path::new(dir.trim());
                for f in [dir.join("HEAD"), dir.join("logs").join("HEAD")] {
                    if f.exists() {
                        println!("cargo:rerun-if-changed={}", f.display());
                    }
                }
            }
        }
    }
    Some(sha)
}
