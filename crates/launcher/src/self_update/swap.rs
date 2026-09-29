//! Replace the running exe with a verified download, relaunch, and clean
//! up afterwards.
//!
//! Windows will not delete or overwrite a running image, but it does let it
//! be renamed within its volume. So:
//!
//! 1. delete a leftover `<exe>.old` from an earlier update;
//! 2. rename the running `<exe>` to `<exe>.old`;
//! 3. rename the verified download to `<exe>`; if that fails, rename
//!    `<exe>.old` back (rollback);
//! 4. start `<exe>` with the same arguments and [`RELAUNCH_ENV`] set, and
//!    let this process exit; if the start fails, undo 2 and 3.
//!
//! The new process waits for this one's `launcher.lock` (see
//! [`acquire_lock`]) and deletes `<exe>.old` once this process is gone
//! ([`remove_old_exe`]). The exe keeps whatever name the player gave it:
//! every path here comes from `std::env::current_exe`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

/// Set on the relaunched process. Its value is the tag of the launcher
/// that started it, so the new window can say what it updated from.
pub const RELAUNCH_ENV: &str = "SGW_LAUNCHER_UPDATED_FROM";

/// `<exe>.old`, e.g. `sgw-launcher-launcher-20260929-f518b57.exe.old`.
pub fn old_path_for(exe: &Path) -> PathBuf {
    let mut name = exe.file_name().map(OsString::from).unwrap_or_default();
    name.push(".old");
    exe.with_file_name(name)
}

#[derive(Debug, Error)]
pub enum SwapError {
    #[error("could not remove the previous update's leftover {path}: {source}")]
    StaleOld {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not move the running launcher aside ({0}); nothing was changed")]
    MoveAside(std::io::Error),
    #[error("could not put the new launcher in place ({0}); the old one was restored")]
    Install(std::io::Error),
    #[error(
        "could not put the new launcher in place ({install}) and could not restore the old one \
         ({rollback}); rename {old} back to {exe} by hand"
    )]
    RollbackFailed {
        install: std::io::Error,
        rollback: std::io::Error,
        old: PathBuf,
        exe: PathBuf,
    },
}

impl SwapError {
    pub fn reason(&self) -> &'static str {
        match self {
            SwapError::StaleOld { .. } => "stale_old_locked",
            SwapError::MoveAside(_) => "move_aside_failed",
            SwapError::Install(_) => "install_failed_rolled_back",
            SwapError::RollbackFailed { .. } => "rollback_failed",
        }
    }
}

/// Steps 1-3: put `new_file` at `exe`, keeping the old one at
/// `<exe>.old`. On any failure the running exe is back at `exe`.
pub fn swap_in(exe: &Path, new_file: &Path) -> Result<(), SwapError> {
    let old = old_path_for(exe);
    if old.exists() {
        std::fs::remove_file(&old).map_err(|source| SwapError::StaleOld {
            path: old.clone(),
            source,
        })?;
    }
    std::fs::rename(exe, &old).map_err(SwapError::MoveAside)?;
    if let Err(install) = std::fs::rename(new_file, exe) {
        return match std::fs::rename(&old, exe) {
            Ok(()) => Err(SwapError::Install(install)),
            Err(rollback) => Err(SwapError::RollbackFailed {
                install,
                rollback,
                old,
                exe: exe.to_path_buf(),
            }),
        };
    }
    Ok(())
}

/// Undo a completed [`swap_in`]: delete the new `exe` and put
/// `<exe>.old` back. Used when the new exe will not start.
pub fn roll_back(exe: &Path) -> std::io::Result<()> {
    let old = old_path_for(exe);
    std::fs::remove_file(exe)?;
    std::fs::rename(old, exe)
}

/// Step 4: start the new exe with this process's arguments.
pub fn relaunch(exe: &Path, from_tag: &str) -> std::io::Result<std::process::Child> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).env(RELAUNCH_ENV, from_tag);
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn()
}

/// What happened to `<exe>.old` at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldCleanup {
    NoneFound,
    Removed {
        attempts: u32,
    },
    /// Still locked after every attempt (the old process is slow to exit,
    /// or antivirus is scanning it). The next start tries again.
    StillLocked,
}

/// Delete `<exe>.old`, retrying while it is locked.
pub fn remove_old_exe(exe: &Path, attempts: u32, delay: Duration) -> OldCleanup {
    let old = old_path_for(exe);
    for attempt in 1..=attempts.max(1) {
        match std::fs::remove_file(&old) {
            Ok(()) => return OldCleanup::Removed { attempts: attempt },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return if attempt == 1 {
                    OldCleanup::NoneFound
                } else {
                    OldCleanup::Removed { attempts: attempt }
                };
            }
            Err(_) if attempt < attempts => std::thread::sleep(delay),
            Err(_) => {}
        }
    }
    OldCleanup::StillLocked
}

/// Take the single-instance lock. A relaunched process waits up to `wait`
/// for the launcher that started it to exit and release the lock; a
/// normal start does not wait. `try_lock` returns true once the lock is
/// held.
pub fn acquire_lock(mut try_lock: impl FnMut() -> bool, wait: Duration, poll: Duration) -> bool {
    let deadline = std::time::Instant::now() + wait;
    loop {
        if try_lock() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(poll);
    }
}

#[cfg(test)]
#[path = "swap_tests.rs"]
mod tests;
