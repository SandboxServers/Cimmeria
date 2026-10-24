//! Rosetta 2 auto-install (PD-09, D-PD1, D-PD10): `softwareupdate` behind the
//! standard macOS administrator prompt, then the PD-08 probe again.
//!
//! Exit codes from `softwareupdate` are unreliable (F9), so success is decided
//! only by the re-probe. The command is a constant: nothing from the player,
//! the launcher state or the environment is interpolated into it.

use super::events::{self, Dependency, DependencyOutcome};
use super::RosettaProbe;
use crate::failure::FailureCode;

/// The `osascript` program that runs the install behind the admin prompt.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const OSASCRIPT: &str = "/usr/bin/osascript";

/// The AppleScript `osascript -e` runs. Never built from runtime values.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const INSTALL_SCRIPT: &str = "do shell script \"/usr/sbin/softwareupdate --install-rosetta --agree-to-license\" with administrator privileges";

/// How long the install (prompt included) may run before it is killed.
#[cfg(target_os = "macos")]
const INSTALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// How much of the installer's stderr is kept for [`classify_install`]. It is
/// never logged.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) const STDERR_TAIL_BYTES: usize = 512;

/// How a Rosetta install ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallOutcome {
    Installed,
    Failed { exit_code: Option<i32> },
    Declined,
    TimedOut,
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
pub(crate) fn emit_finished(outcome: InstallOutcome, elapsed_ms: u64) {
    let (code, failure, exit_code) = match outcome {
        InstallOutcome::Installed => (DependencyOutcome::Installed, None, None),
        InstallOutcome::Failed { exit_code } => (
            DependencyOutcome::Failed,
            Some(FailureCode::RosettaInstallFailed),
            exit_code,
        ),
        InstallOutcome::Declined => (
            DependencyOutcome::Declined,
            Some(FailureCode::RosettaInstallDeclined),
            None,
        ),
        InstallOutcome::TimedOut => (
            DependencyOutcome::TimedOut,
            Some(FailureCode::RosettaInstallFailed),
            None,
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
/// probe. Logs the C-PD5 start and outcome on `launcher.install`. Off macOS it
/// installs nothing and returns `Failed { exit_code: None }`.
pub async fn install_rosetta() -> InstallOutcome {
    #[cfg(target_os = "macos")]
    {
        let started = std::time::Instant::now();
        events::dependency_install_started(Dependency::Rosetta, None);
        let outcome = run_install().await;
        emit_finished(outcome, started.elapsed().as_millis() as u64);
        outcome
    }
    #[cfg(not(target_os = "macos"))]
    {
        InstallOutcome::Failed { exit_code: None }
    }
}

/// Run the install command, then the re-probe, and classify the result.
#[cfg(target_os = "macos")]
async fn run_install() -> InstallOutcome {
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
                    return InstallOutcome::TimedOut;
                }
                Ok((status, tail)) => (status.ok().and_then(|status| status.code()), tail),
            }
        }
    };
    let probe_after = super::mac::rosetta().await;
    classify_install(exit_code, &String::from_utf8_lossy(&tail), probe_after)
}

#[cfg(test)]
#[path = "rosetta_install_tests.rs"]
mod tests;
