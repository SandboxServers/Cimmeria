//! Wait for the launched game process to exit, off the async runtime.
//!
//! The launcher hands either the [`std::process::Child`] from a plain
//! launch to [`wait_for_exit`], or the [`RunningProcess`] from an
//! injected launch to [`wait_for_running_exit`]; each waits in
//! `spawn_blocking`. When the game exits the future resolves; the
//! orchestrator then does its final flush + bundle upload.
//!
//! Dropping the future does NOT kill the game — `std::process::Child`
//! has no kill-on-drop semantic (that's the tokio variant),
//! `RunningProcess` only closes its handle, and we deliberately don't
//! use a Job Object with KILL_ON_JOB_CLOSE. Closing the launcher window
//! mid-session leaves the game alive.

use std::future::Future;
use std::pin::Pin;
use std::process::Child;

use cimmeria_client_launch::inject::RunningProcess;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum WatchError {
    #[error("Wait failed: {0}")]
    Wait(#[from] std::io::Error),
    #[error("Wait task panicked")]
    JoinPanic,
}

#[derive(Debug, Clone)]
pub struct ExitReport {
    pub pid: u32,
    pub exit_code: Option<i32>,
}

/// The game-exit future a telemetry session runs until, whichever way
/// the game was launched.
pub type ExitWaiter = Pin<Box<dyn Future<Output = Result<ExitReport, WatchError>> + Send>>;

/// Wait on a game launched with DLLs injected.
pub async fn wait_for_running_exit(process: RunningProcess) -> Result<ExitReport, WatchError> {
    let pid = process.pid();
    let exit_code = tokio::task::spawn_blocking(move || process.wait())
        .await
        .map_err(|_| WatchError::JoinPanic)??;
    Ok(ExitReport {
        pid,
        exit_code: Some(exit_code),
    })
}

pub async fn wait_for_exit(mut child: Child) -> Result<ExitReport, WatchError> {
    let pid = child.id();
    let status = tokio::task::spawn_blocking(move || child.wait())
        .await
        .map_err(|_| WatchError::JoinPanic)??;
    Ok(ExitReport {
        pid,
        exit_code: status.code(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn spawn_sleeper() -> Child {
        #[cfg(windows)]
        {
            // `cmd /C exit 0` returns immediately with code 0 — fine
            // for the happy-path test without needing a sleep utility.
            Command::new("cmd").args(["/C", "exit 0"]).spawn().unwrap()
        }
        #[cfg(not(windows))]
        {
            Command::new("sh").args(["-c", "exit 0"]).spawn().unwrap()
        }
    }

    fn spawn_failer() -> Child {
        #[cfg(windows)]
        {
            Command::new("cmd").args(["/C", "exit 7"]).spawn().unwrap()
        }
        #[cfg(not(windows))]
        {
            Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap()
        }
    }

    #[tokio::test]
    async fn wait_for_exit_resolves_with_pid_and_exit_code() {
        let child = spawn_sleeper();
        let report = wait_for_exit(child).await.unwrap();
        assert!(report.pid > 0);
        assert_eq!(report.exit_code, Some(0));
    }

    #[tokio::test]
    async fn wait_for_exit_surfaces_non_zero_exit_code() {
        let child = spawn_failer();
        let report = wait_for_exit(child).await.unwrap();
        assert_eq!(report.exit_code, Some(7));
    }
}
