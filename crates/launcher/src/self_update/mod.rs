//! Launcher self-update: find a newer launcher release on GitHub, download
//! and verify it, swap it in for the running exe and relaunch.
//!
//! - [`build_info`]: the tag and build time the release workflow stamps in.
//! - [`releases`]: the GitHub Releases lookup and the download host policy.
//! - [`version`]: the ordering rule (never downgrade) and the manifest's
//!   optional `min_launcher` gate.
//! - [`download`]: download beside the exe, verify size and SHA-256.
//! - [`swap`]: rename-aside swap, rollback, relaunch, `.old` cleanup.
//!
//! Trust is HTTPS to GitHub plus the SHA-256 published in the same release,
//! the same trust as downloading the exe by hand. Design notes:
//! docs/client/sgw-launcher.md, "Self-update".
//!
//! Every step logs on target `launcher.update` with an `event` field;
//! refusals and failures carry `reason`.

pub mod build_info;
pub mod download;
pub mod releases;
pub mod swap;
pub mod version;

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tracing::{info, warn};

pub use build_info::LauncherBuild;
pub use releases::{LauncherRelease, UpdateEndpoints};
pub use version::{MinLauncherGate, UpdateDecision};

use download::DownloadError;
use releases::FetchError;
use swap::SwapError;

const TARGET: &str = "launcher.update";

/// User-Agent for every updater request. GitHub's API refuses requests
/// without one.
pub fn user_agent(build: &LauncherBuild) -> String {
    format!("sgw-launcher/{}", build.tag.as_deref().unwrap_or("dev"))
}

/// The result of a successful check: the decision, plus every candidate
/// release (the `min_launcher` gate looks up same-day tags in it).
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub decision: UpdateDecision,
    pub releases: Vec<LauncherRelease>,
}

/// Ask GitHub for launcher releases and decide. A development build makes
/// no request at all.
pub async fn check(
    http: &reqwest::Client,
    endpoints: &UpdateEndpoints,
    build: &LauncherBuild,
) -> Result<CheckOutcome, FetchError> {
    if !build.is_release() {
        info!(
            target: TARGET,
            event = "update_check",
            outcome = "dev_build",
            "development build; updates disabled"
        );
        return Ok(CheckOutcome {
            decision: UpdateDecision::DevBuild,
            releases: Vec::new(),
        });
    }
    let releases = match releases::fetch_launcher_releases(http, endpoints).await {
        Ok(r) => r,
        Err(e) => {
            warn!(
                target: TARGET,
                event = "update_check_failed",
                reason = e.reason(),
                running = %build.display(),
                error = %e,
                "launcher update check failed"
            );
            return Err(e);
        }
    };
    let decision = version::decide(build, &releases);
    let (outcome, newest) = match &decision {
        UpdateDecision::DevBuild => ("dev_build", String::new()),
        UpdateDecision::NoRelease => ("no_release", String::new()),
        UpdateDecision::UpToDate { newest } => ("up_to_date", newest.clone()),
        UpdateDecision::Available(r) => ("available", r.tag.clone()),
    };
    info!(
        target: TARGET,
        event = "update_check",
        outcome,
        running = %build.display(),
        newest = %newest,
        candidates = releases.len(),
        "launcher update check"
    );
    Ok(CheckOutcome { decision, releases })
}

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error(
        "the launcher's folder ({0}) is not writable, so it cannot update itself; \
         download the new version from the release page"
    )]
    NotWritable(PathBuf),
    #[error("{0}")]
    Download(#[from] DownloadError),
    #[error("{0}")]
    Swap(#[from] SwapError),
    #[error("the new launcher would not start ({error}); {}", if *rolled_back { "the old one was restored" } else { "restoring the old one also failed" })]
    Relaunch {
        error: std::io::Error,
        rolled_back: bool,
    },
    #[error("could not find the running launcher's path: {0}")]
    NoExePath(std::io::Error),
}

impl ApplyError {
    pub fn reason(&self) -> &'static str {
        match self {
            ApplyError::NotWritable(_) => "dir_not_writable",
            ApplyError::Download(e) => e.reason(),
            ApplyError::Swap(e) => e.reason(),
            ApplyError::Relaunch { .. } => "relaunch_failed",
            ApplyError::NoExePath(_) => "no_exe_path",
        }
    }
}

/// Download, verify and swap in `release` for `exe`, then start it. On
/// `Ok` the new launcher is running and this process should exit.
pub async fn apply(
    http: &reqwest::Client,
    endpoints: &UpdateEndpoints,
    build: &LauncherBuild,
    release: &LauncherRelease,
    exe: &Path,
    progress: &tokio::sync::mpsc::UnboundedSender<crate::install::Progress>,
) -> Result<(), ApplyError> {
    let result = apply_inner(http, endpoints, build, release, exe, progress).await;
    if let Err(e) = &result {
        warn!(
            target: TARGET,
            event = "update_failed",
            reason = e.reason(),
            running = %build.display(),
            target_tag = %release.tag,
            error = %e,
            "launcher update failed"
        );
    }
    result
}

async fn apply_inner(
    http: &reqwest::Client,
    endpoints: &UpdateEndpoints,
    build: &LauncherBuild,
    release: &LauncherRelease,
    exe: &Path,
    progress: &tokio::sync::mpsc::UnboundedSender<crate::install::Progress>,
) -> Result<(), ApplyError> {
    let dir = exe.parent().unwrap_or(Path::new(".")).to_path_buf();
    if !crate::launch::install_dir_writable(&dir) {
        return Err(ApplyError::NotWritable(dir));
    }
    info!(
        target: TARGET,
        event = "update_download_started",
        running = %build.display(),
        target_tag = %release.tag,
        bytes = release.exe.size,
        "downloading launcher update"
    );
    let new_file = download::download_verified(http, endpoints, release, &dir, progress).await?;
    info!(
        target: TARGET,
        event = "update_verified",
        target_tag = %release.tag,
        bytes = release.exe.size,
        "launcher update verified (size + sha256)"
    );

    if let Err(e) = swap::swap_in(exe, &new_file) {
        let _ = std::fs::remove_file(&new_file);
        return Err(e.into());
    }
    info!(
        target: TARGET,
        event = "update_swapped",
        exe = %exe.display(),
        target_tag = %release.tag,
        "launcher exe swapped; previous kept as .old"
    );

    let from = build.tag.clone().unwrap_or_default();
    match swap::relaunch(exe, &from) {
        Ok(child) => {
            info!(
                target: TARGET,
                event = "update_relaunched",
                pid = child.id(),
                target_tag = %release.tag,
                "new launcher started; this one exits"
            );
            Ok(())
        }
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
            Err(ApplyError::Relaunch { error, rolled_back })
        }
    }
}

/// Startup housekeeping after an update: delete `<exe>.old` in the
/// background, retrying while the old process lets go of it.
pub fn spawn_startup_cleanup() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    if let Ok(from) = std::env::var(swap::RELAUNCH_ENV) {
        info!(
            target: TARGET,
            event = "update_started_new",
            from = %from,
            running = %LauncherBuild::current().display(),
            "launcher started after a self-update"
        );
    }
    std::thread::spawn(
        move || match swap::remove_old_exe(&exe, 20, Duration::from_millis(500)) {
            swap::OldCleanup::NoneFound => {}
            swap::OldCleanup::Removed { attempts } => info!(
                target: TARGET,
                event = "update_old_removed",
                attempts,
                "removed the previous launcher's .old file"
            ),
            swap::OldCleanup::StillLocked => warn!(
                target: TARGET,
                event = "update_old_remove_failed",
                reason = "still_locked",
                path = %swap::old_path_for(&exe).display(),
                "the previous launcher's .old file is still locked; the next start retries"
            ),
        },
    );
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
