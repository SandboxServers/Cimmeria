//! Rosetta 2 auto-install (PD-09, D-PD1, D-PD10): `softwareupdate` behind the
//! standard macOS administrator prompt, then the PD-08 probe again.
//!
//! Exit codes from `softwareupdate` are unreliable (F9), so success is decided
//! only by the re-probe. The command is a constant: nothing from the player,
//! the launcher state or the environment is interpolated into it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::events::{self, Dependency, DependencyOutcome};
use super::RosettaProbe;
use crate::failure::FailureCode;

/// The `osascript` program that runs the install behind the admin prompt.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const OSASCRIPT: &str = "/usr/bin/osascript";

/// The AppleScript `osascript -e` runs. Never built from runtime values.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const INSTALL_SCRIPT: &str = "do shell script \"/usr/sbin/softwareupdate --install-rosetta --agree-to-license\" with administrator privileges";

/// How long the install (prompt included) may run before `osascript` is
/// killed. The kill reaches `osascript` only: the privileged `softwareupdate`
/// runs as root outside the launcher's reach and may keep running, and finish,
/// after the timeout.
#[cfg(target_os = "macos")]
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// How long a new install is refused after one timed out, so a retry does not
/// stack a second root `softwareupdate` on one that may still be running.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const TIMED_OUT_COOLDOWN: Duration = Duration::from_secs(5 * 60);

/// The single-flight gate every [`install_rosetta`] call passes.
#[cfg(target_os = "macos")]
static INSTALL_GATE: InstallGate = InstallGate::new();

/// How much of the installer's stderr is kept for [`classify_install`]. It is
/// never logged.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const STDERR_TAIL_BYTES: usize = 512;

/// How a Rosetta install ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    Failed {
        exit_code: Option<i32>,
    },
    Declined,
    /// The install outlived [`INSTALL_TIMEOUT`] and the re-probe still did
    /// not find Rosetta. The privileged installer may still be running and may
    /// finish later, so the caller should probe again rather than trust this.
    TimedOut,
}

/// Lets one Rosetta install run at a time, and holds new ones off for a
/// cool-down after a timeout (the root installer may still be running).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) struct InstallGate {
    in_flight: AtomicBool,
    cooldown_until: Mutex<Option<Instant>>,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
impl InstallGate {
    pub(crate) const fn new() -> Self {
        Self {
            in_flight: AtomicBool::new(false),
            cooldown_until: Mutex::new(None),
        }
    }

    /// Take the gate, or `None` while another install runs or a timed-out
    /// one's cool-down has not passed at `now`.
    pub(crate) fn try_acquire(&self, now: Instant) -> Option<InstallPermit<'_>> {
        self.in_flight
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()?;
        let permit = InstallPermit { gate: self };
        let cooling = self
            .cooldown_until
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some_and(|until| now < until);
        (!cooling).then_some(permit)
    }
}

/// Held for one install; dropping it, on any path including a cancelled
/// future, frees the gate.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) struct InstallPermit<'a> {
    gate: &'a InstallGate,
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
impl InstallPermit<'_> {
    /// Free the gate, but refuse new installs until `until`.
    pub(crate) fn release_with_cooldown(self, until: Instant) {
        *self
            .gate
            .cooldown_until
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(until);
    }
}

impl Drop for InstallPermit<'_> {
    fn drop(&mut self) {
        self.gate.in_flight.store(false, Ordering::Release);
    }
}

/// Decide a timed-out install by the probe run after the kill: the root
/// installer may have finished just before it, and the probe is the only
/// trusted signal (F9).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn classify_timeout(probe_after: RosettaProbe) -> InstallOutcome {
    if probe_after == RosettaProbe::Present {
        InstallOutcome::Installed
    } else {
        InstallOutcome::TimedOut
    }
}

/// Decide a finished install: the re-probe wins whatever the exit code (F9);
/// otherwise AppleScript's "User canceled" `(-128)` means the player declined
/// the prompt; anything else failed.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn classify_install(
    exit_code: Option<i32>,
    stderr_tail: &str,
    probe_after: RosettaProbe,
) -> InstallOutcome {
    if probe_after == RosettaProbe::Present {
        InstallOutcome::Installed
    } else if stderr_tail.contains("(-128)") {
        InstallOutcome::Declined
    } else {
        InstallOutcome::Failed { exit_code }
    }
}

/// Emit `dependency_install_finished` for a Rosetta install outcome.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn emit_finished(outcome: InstallOutcome, exit_code: Option<i32>, elapsed_ms: u64) {
    let (code, failure) = match outcome {
        InstallOutcome::Installed => (DependencyOutcome::Installed, None),
        InstallOutcome::Failed { .. } => (
            DependencyOutcome::Failed,
            Some(FailureCode::RosettaInstallFailed),
        ),
        InstallOutcome::Declined => (
            DependencyOutcome::Declined,
            Some(FailureCode::RosettaInstallDeclined),
        ),
        InstallOutcome::TimedOut => (
            DependencyOutcome::TimedOut,
            Some(FailureCode::RosettaInstallFailed),
        ),
    };
    events::dependency_install_finished(Dependency::Rosetta, code, failure, elapsed_ms, exit_code);
}

/// Keep at most the last [`STDERR_TAIL_BYTES`] of `buf`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn keep_tail(buf: &mut Vec<u8>) {
    if buf.len() > STDERR_TAIL_BYTES {
        buf.drain(..buf.len() - STDERR_TAIL_BYTES);
    }
}

/// Install Rosetta 2 behind the administrator prompt and confirm it with the
/// probe. Logs the C-PD5 start and outcome on `launcher.install`. While
/// another install runs, or for [`TIMED_OUT_COOLDOWN`] after one timed out, it
/// returns `Failed { exit_code: None }` without spawning or logging. Off macOS
/// it installs nothing and returns `Failed { exit_code: None }`.
pub async fn install_rosetta() -> InstallOutcome {
    #[cfg(target_os = "macos")]
    {
        let Some(permit) = INSTALL_GATE.try_acquire(Instant::now()) else {
            return InstallOutcome::Failed { exit_code: None };
        };
        let started = Instant::now();
        events::dependency_install_started(Dependency::Rosetta, None);
        let (outcome, exit_code) = run_install().await;
        emit_finished(outcome, exit_code, started.elapsed().as_millis() as u64);
        if outcome == InstallOutcome::TimedOut {
            permit.release_with_cooldown(Instant::now() + TIMED_OUT_COOLDOWN);
        }
        outcome
    }
    #[cfg(not(target_os = "macos"))]
    {
        InstallOutcome::Failed { exit_code: None }
    }
}

/// Run the install command, then the re-probe, and classify the result.
#[cfg(target_os = "macos")]
async fn run_install() -> (InstallOutcome, Option<i32>) {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;

    let spawned = tokio::process::Command::new(OSASCRIPT)
        .args(["-e", INSTALL_SCRIPT])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn();
    let (exit_code, tail) = match spawned {
        Err(_) => (None, Vec::new()),
        Ok(mut child) => {
            let stderr = child.stderr.take();
            let read_tail = async move {
                let mut tail = Vec::new();
                if let Some(mut stderr) = stderr {
                    let mut chunk = [0u8; 1024];
                    while let Ok(n) = stderr.read(&mut chunk).await {
                        if n == 0 {
                            break;
                        }
                        tail.extend_from_slice(&chunk[..n]);
                        keep_tail(&mut tail);
                    }
                }
                tail
            };
            let run = async { tokio::join!(child.wait(), read_tail) };
            match tokio::time::timeout(INSTALL_TIMEOUT, run).await {
                Err(_) => {
                    let _ = child.kill().await;
                    return (classify_timeout(super::mac::rosetta().await), None);
                }
                Ok((status, tail)) => (status.ok().and_then(|status| status.code()), tail),
            }
        }
    };
    let probe_after = super::mac::rosetta().await;
    (
        classify_install(exit_code, &String::from_utf8_lossy(&tail), probe_after),
        exit_code,
    )
}

#[cfg(test)]
#[path = "rosetta_install_tests.rs"]
mod tests;
