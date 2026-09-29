//! The handoff between the launcher that downloaded an update and the one
//! it starts.
//!
//! Old process, after [`super::swap::swap_in`] ([`hand_off`]):
//!
//! 1. start the new exe with [`RELAUNCH_ENV`] (our tag) and
//!    [`RELAUNCH_PID_ENV`] (our pid); if it will not start, roll the swap
//!    back and keep running, still holding the lock;
//! 2. release `launcher.lock` ([`crate::instance_lock::release`]);
//! 3. exit the process at once, from the worker thread.
//!
//! Nothing touches install state between 2 and 3. The exit does not wait
//! for an egui frame: `launcher-20260929-676f314` closed its window with a
//! viewport command, which only runs on the next frame, and with no input
//! that frame never came. It kept the lock, and the new launcher gave up
//! after 30 s with "Another … instance appears to be running".
//!
//! The lock is released after the start, not before it, so a failed start
//! leaves this launcher running with its lock intact. The new launcher
//! simply waits the few milliseconds until step 2.
//!
//! New process, at startup ([`acquire_startup_lock`]): a normal start
//! tries the lock once and refuses a second instance, unchanged. A
//! relaunch waits [`LockWait::first`] for the lock; if the old launcher
//! still holds it (an older launcher that closes only on the next frame),
//! it ends that process, if and only if its image is this exe or
//! `<exe>.old` ([`super::old_process`]), and waits again.
//!
//! The launcher saves nothing on exit (no eframe persistence, no
//! `on_exit`; config and telemetry state are written when they change), so
//! the hard exit loses nothing a window close would have saved.

use std::ffi::OsString;
use std::path::Path;
use std::time::{Duration, Instant};

use tracing::{info, warn};

use super::swap::{self, RELAUNCH_ENV};
use super::{ApplyError, TARGET};

/// Set on the relaunched process: the pid of the launcher that started
/// it, so the new one can end exactly that process if it hangs on to the
/// lock.
pub const RELAUNCH_PID_ENV: &str = "SGW_LAUNCHER_UPDATED_FROM_PID";

/// The process operations of the handoff, injectable for tests.
pub trait HandoffHooks {
    /// Start the new exe with the relaunch variables set; its pid.
    fn spawn(&mut self, exe: &Path, from_tag: &str, old_pid: u32) -> std::io::Result<u32>;
    /// Let go of the single-instance lock; false when none was held.
    fn release_lock(&mut self) -> bool;
    /// End this process. The production hook never returns.
    fn exit(&mut self);
}

/// The real process: [`swap::relaunch`], [`crate::instance_lock`],
/// [`std::process::exit`].
#[derive(Debug, Default)]
pub struct ProcessHooks;

impl HandoffHooks for ProcessHooks {
    fn spawn(&mut self, exe: &Path, from_tag: &str, old_pid: u32) -> std::io::Result<u32> {
        swap::relaunch(exe, from_tag, old_pid).map(|child| child.id())
    }

    fn release_lock(&mut self) -> bool {
        crate::instance_lock::release()
    }

    fn exit(&mut self) {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        std::process::exit(0);
    }
}

/// Start the swapped-in exe, release the lock and exit (see the module
/// docs). Returns only when the start failed (after rolling back) or when
/// a test's exit hook returns.
pub fn hand_off(
    exe: &Path,
    from_tag: &str,
    target_tag: &str,
    hooks: &mut impl HandoffHooks,
) -> Result<(), ApplyError> {
    let old_pid = std::process::id();
    info!(
        target: TARGET,
        event = "update_handoff_started",
        exe = %exe.display(),
        from = %from_tag,
        target_tag = %target_tag,
        old_pid,
        "starting the new launcher"
    );
    let pid = match hooks.spawn(exe, from_tag, old_pid) {
        Ok(pid) => pid,
        Err(error) => {
            let rolled_back = swap::roll_back(exe).is_ok();
            warn!(
                target: TARGET,
                event = "update_rollback",
                reason = "relaunch_failed",
                rolled_back,
                error = %error,
                "new launcher would not start; rolling back"
            );
            return Err(ApplyError::Relaunch { error, rolled_back });
        }
    };
    info!(
        target: TARGET,
        event = "update_relaunched",
        pid,
        target_tag = %target_tag,
        "new launcher started"
    );
    let held = hooks.release_lock();
    if held {
        info!(
            target: TARGET,
            event = "update_lock_released",
            pid,
            "released launcher.lock for the new launcher; exiting"
        );
    } else {
        warn!(
            target: TARGET,
            event = "update_lock_released",
            reason = "lock_not_held",
            pid,
            "no launcher.lock was held at handoff; exiting anyway"
        );
    }
    hooks.exit();
    Ok(())
}

/// What the relaunch variables say, when this process was started by a
/// self-update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaunchInfo {
    pub from_tag: String,
    /// `None` when started by a launcher older than the pid variable
    /// (`launcher-20260929-676f314`, `launcher-20260929-4fcae33`).
    pub old_pid: Option<u32>,
}

impl RelaunchInfo {
    pub fn from_env() -> Option<Self> {
        Self::from_values(
            std::env::var_os(RELAUNCH_ENV),
            std::env::var_os(RELAUNCH_PID_ENV),
        )
    }

    fn from_values(tag: Option<OsString>, pid: Option<OsString>) -> Option<Self> {
        let tag = tag?;
        Some(Self {
            from_tag: tag.to_string_lossy().into_owned(),
            old_pid: pid.and_then(|p| p.to_str().and_then(|s| s.trim().parse().ok())),
        })
    }
}

/// How long a relaunch waits for the lock.
#[derive(Debug, Clone, Copy)]
pub struct LockWait {
    /// For the old launcher to exit by itself. A current launcher exits
    /// within milliseconds of starting this one; the long tail is an
    /// older launcher waiting for its next frame.
    pub first: Duration,
    /// After ending the old launcher, for Windows to drop its lock.
    pub after_kill: Duration,
    pub poll: Duration,
}

impl LockWait {
    pub const PRODUCTION: LockWait = LockWait {
        first: Duration::from_secs(10),
        after_kill: Duration::from_secs(10),
        poll: Duration::from_millis(250),
    };
}

/// Result of the fallback that ends the old launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillOutcome {
    Killed {
        pid: u32,
    },
    /// Nothing was ended; `reason` says why (for the log).
    NotKilled {
        reason: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupLock {
    Acquired {
        waited: Duration,
        killed_pid: Option<u32>,
    },
    /// A normal start found another launcher running.
    HeldByAnotherInstance,
    /// A relaunch could not get the lock even with the fallback.
    UpdateHandoffStuck { reason: &'static str },
}

/// Take the single-instance lock at startup. `relaunch` is
/// [`RelaunchInfo::from_env`]; without it this is one `try_lock`, as
/// before. `kill_old` is the fallback for a relaunch whose old launcher
/// outlives [`LockWait::first`].
pub fn acquire_startup_lock(
    relaunch: Option<&RelaunchInfo>,
    mut try_lock: impl FnMut() -> bool,
    mut kill_old: impl FnMut(&RelaunchInfo) -> KillOutcome,
    wait: &LockWait,
) -> StartupLock {
    let Some(info) = relaunch else {
        return if try_lock() {
            StartupLock::Acquired {
                waited: Duration::ZERO,
                killed_pid: None,
            }
        } else {
            StartupLock::HeldByAnotherInstance
        };
    };
    let start = Instant::now();
    if swap::acquire_lock(&mut try_lock, wait.first, wait.poll) {
        let waited = start.elapsed();
        info!(
            target: TARGET,
            event = "update_relaunch_lock_acquired",
            waited_ms = waited.as_millis() as u64,
            from = %info.from_tag,
            old_pid = ?info.old_pid,
            killed = false,
            "relaunched launcher took launcher.lock"
        );
        return StartupLock::Acquired {
            waited,
            killed_pid: None,
        };
    }
    warn!(
        target: TARGET,
        event = "update_relaunch_lock_timeout",
        reason = "old_launcher_still_running",
        waited_ms = start.elapsed().as_millis() as u64,
        from = %info.from_tag,
        old_pid = ?info.old_pid,
        "the previous launcher still holds launcher.lock; ending it"
    );
    let pid = match kill_old(info) {
        KillOutcome::Killed { pid } => pid,
        KillOutcome::NotKilled { reason } => {
            warn!(
                target: TARGET,
                event = "update_fallback_kill",
                outcome = "skipped",
                reason,
                old_pid = ?info.old_pid,
                "could not end the previous launcher"
            );
            return StartupLock::UpdateHandoffStuck { reason };
        }
    };
    warn!(
        target: TARGET,
        event = "update_fallback_kill",
        outcome = "killed",
        reason = "lock_held_after_wait",
        pid,
        "ended the previous launcher, which still held launcher.lock"
    );
    if swap::acquire_lock(&mut try_lock, wait.after_kill, wait.poll) {
        let waited = start.elapsed();
        info!(
            target: TARGET,
            event = "update_relaunch_lock_acquired",
            waited_ms = waited.as_millis() as u64,
            from = %info.from_tag,
            old_pid = pid,
            killed = true,
            "relaunched launcher took launcher.lock"
        );
        StartupLock::Acquired {
            waited,
            killed_pid: Some(pid),
        }
    } else {
        warn!(
            target: TARGET,
            event = "update_relaunch_lock_failed",
            reason = "held_after_kill",
            waited_ms = start.elapsed().as_millis() as u64,
            pid,
            "launcher.lock is still held after ending the previous launcher"
        );
        StartupLock::UpdateHandoffStuck {
            reason: "held_after_kill",
        }
    }
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;
