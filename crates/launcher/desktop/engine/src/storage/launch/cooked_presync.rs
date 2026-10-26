//! The launch's cooked-data presync step (LP-22). It runs the launcher-core
//! presync before client setup, publishes its progress for the shell, gates
//! Play on its outcome and logs its events. A presync never returns an error:
//! every failure is a report (D-LP18, D-LP26).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cimmeria_launcher_core::client_setup::login_servers::LoginServer;
use cimmeria_launcher_core::cooked_presync::fetch::{server_label, Fetcher, Timeouts};
use cimmeria_launcher_core::cooked_presync::report::{
    CategoryAction, Failure, Outcome, Progress, Report, Stage, Step,
};
use cimmeria_launcher_core::cooked_presync::run;
use cimmeria_launcher_core::install_layout;
use serde::Serialize;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::operation_numbers;
use crate::phase_timing;

/// The presync's progress for the shell: the `content_sync` key of the launch
/// status. Every key is always present; `outcome`, `reason` and `failure` are
/// null until the phase is `done`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentSync {
    /// The step the presync is in.
    pub phase: ContentSyncPhase,
    /// Categories staged so far.
    pub categories_done: u32,
    /// Categories the run writes.
    pub categories_total: u32,
    /// Body bytes read so far.
    pub bytes_done: u64,
    /// Body bytes the run reads, in all.
    pub bytes_total: u64,
    /// How the run ended, once it is done.
    pub outcome: Option<Outcome>,
    /// The reason code, once the run is done.
    pub reason: Option<&'static str>,
    /// The first login server's `host:port`.
    pub server: Option<String>,
    /// The failure, when the outcome is `failed`.
    pub failure: Option<Failure>,
}

/// The phase a presync is in, as the shell shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSyncPhase {
    /// Checking the cache folder and the index.
    Checking,
    /// Fetching a category body.
    Downloading,
    /// Composing and writing a category's temp file.
    Writing,
    /// Renaming the temps over their PAKs.
    Committing,
    /// The run has ended.
    Done,
}

/// Where the presync reads and writes: the Firesky folder, or why there is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Profile {
    /// The `My Games\Firesky` folder under the user's Documents.
    Firesky(PathBuf),
    /// Windows, but no Documents folder could be found.
    Unavailable,
    /// Not a Windows launch (or a test build): the presync is skipped.
    NotThisPlatform,
}

/// The Firesky profile of this launch. Tests never reach the real Documents
/// folder (R6): test builds always get `NotThisPlatform`.
#[cfg(all(windows, not(any(test, feature = "test-support"))))]
#[allow(dead_code)] // wired by LP-27
pub(crate) fn profile() -> Profile {
    match cimmeria_launcher_core::client_paths::firesky_root() {
        Some(root) => Profile::Firesky(root),
        None => Profile::Unavailable,
    }
}

/// The Firesky profile of this launch; off Windows and in tests, `NotThisPlatform`.
#[cfg(not(all(windows, not(any(test, feature = "test-support")))))]
#[allow(dead_code)] // wired by LP-27
pub(crate) fn profile() -> Profile {
    Profile::NotThisPlatform
}

/// Runs the presync with the shipped limits and records its events for launch
/// `id`. The worker calls it; LP-27 passes `plan.id`.
#[allow(dead_code)] // wired by LP-27
pub(crate) async fn presync(
    id: Uuid,
    profile: Profile,
    content_root: &Path,
    servers: &[LoginServer],
    cancel: &CancellationToken,
    updates: &watch::Sender<Option<ContentSync>>,
) -> Report {
    let report = presync_at(
        profile,
        content_root,
        servers,
        Timeouts::DEFAULT,
        run::DEADLINE,
        cancel,
        updates,
    )
    .await;
    emit_events(id, &report);
    report
}

/// Runs the presync under the given limits. Every path ends by sending the
/// `done` update, so the shell always sees the outcome.
#[allow(dead_code)] // wired by LP-27
pub(crate) async fn presync_at(
    profile: Profile,
    content_root: &Path,
    servers: &[LoginServer],
    timeouts: Timeouts,
    deadline: Duration,
    cancel: &CancellationToken,
    updates: &watch::Sender<Option<ContentSync>>,
) -> Report {
    let first = servers.first();
    let server = first.and_then(|first| server_label(&first.url));
    let report = match (profile, first) {
        (Profile::NotThisPlatform, _) => Report::skipped("platform_unsupported"),
        (Profile::Unavailable, _) => {
            Report::failed(Failure::new(Step::Prepare, "profile_unavailable"))
        }
        (Profile::Firesky(_), None) => {
            Report::failed(Failure::new(Step::Prepare, "no_login_server"))
        }
        (Profile::Firesky(firesky), Some(first)) => {
            let urls: Vec<String> = servers.iter().map(|s| s.url.clone()).collect();
            match Fetcher::with_timeouts(&first.url, &urls, timeouts) {
                Err(_) => Report::failed(Failure::new(Step::Prepare, "endpoint_refused")),
                Ok(fetcher) => {
                    let source =
                        install_layout::sgwgame_dir(content_root).join("SourceCache.en-us");
                    let progress = |p: Progress| {
                        updates.send_replace(Some(in_progress(&p, server.clone())));
                    };
                    run::run(
                        &fetcher,
                        &firesky,
                        &source,
                        Instant::now() + deadline,
                        cancel,
                        &progress,
                    )
                    .await
                }
            }
        }
    };
    let last = updates.borrow().clone();
    updates.send_replace(Some(finished(last, &report, server)));
    report
}

/// The shell's gate on a presync report (LP-27 applies it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    /// Client setup may run.
    Continue,
    /// Play is blocked: the launch does not start.
    Blocked,
    /// The player cancelled: the launch stops quietly.
    Cancelled,
}

/// The gate for a report: `up_to_date`, `synced` and `skipped` continue,
/// `failed` blocks, `cancelled` cancels.
pub(crate) fn gate(report: &Report) -> Gate {
    match report.outcome {
        Outcome::UpToDate | Outcome::Synced | Outcome::Skipped => Gate::Continue,
        Outcome::Failed => Gate::Blocked,
        Outcome::Cancelled => Gate::Cancelled,
    }
}

/// The phase-timer outcome for a report.
pub(crate) fn timer_outcome(report: &Report) -> phase_timing::Outcome {
    match report.outcome {
        Outcome::UpToDate | Outcome::Synced => phase_timing::Outcome::Succeeded,
        Outcome::Skipped => phase_timing::Outcome::Unavailable,
        Outcome::Failed => phase_timing::Outcome::Failed,
        Outcome::Cancelled => phase_timing::Outcome::Cancelled,
    }
}

/// The update for a progress snapshot.
fn in_progress(progress: &Progress, server: Option<String>) -> ContentSync {
    let phase = match progress.stage {
        Stage::Checking => ContentSyncPhase::Checking,
        Stage::Downloading => ContentSyncPhase::Downloading,
        Stage::Writing => ContentSyncPhase::Writing,
        Stage::Committing => ContentSyncPhase::Committing,
    };
    ContentSync {
        phase,
        categories_done: progress.categories_done,
        categories_total: progress.categories_total,
        bytes_done: progress.bytes_done,
        bytes_total: progress.bytes_total,
        outcome: None,
        reason: None,
        server,
        failure: None,
    }
}

/// The final update. The counts carry over from the last progress update, so
/// an early exit with no progress shows zeros.
fn finished(last: Option<ContentSync>, report: &Report, server: Option<String>) -> ContentSync {
    let (categories_done, categories_total, bytes_done, bytes_total) = match last {
        Some(last) => (
            last.categories_done,
            last.categories_total,
            last.bytes_done,
            last.bytes_total,
        ),
        None => (0, 0, 0, 0),
    };
    ContentSync {
        phase: ContentSyncPhase::Done,
        categories_done,
        categories_total,
        bytes_done,
        bytes_total,
        outcome: Some(report.outcome),
        reason: Some(report.reason),
        server,
        failure: match report.outcome {
            Outcome::Failed => report.failure.clone(),
            _ => None,
        },
    }
}

/// One `cooked_presync_category` line per category, then the
/// `cooked_presync_finished` line. Closed codes and numbers only (F19).
pub(super) fn emit_events(id: Uuid, report: &Report) {
    let operation = operation_numbers::number(id);
    for category in &report.categories {
        let action = match category.action {
            CategoryAction::UpToDate => "up_to_date",
            CategoryAction::Written => "written",
            CategoryAction::Failed => "failed",
        };
        let pak = category.pak.as_str();
        tracing::info!(
            target: "launcher.launch",
            event = "cooked_presync_category",
            operation,
            category_id = category.category_id,
            pak,
            action,
            cache_version = category.cache_version,
            source_version = category.source_version,
            served_version = category.served_version,
            changed = category.changed,
            bytes = category.bytes,
            "cooked presync: category"
        );
    }
    let failure = report.failure.as_ref();
    let step = failure.map(|f| step_code(f.step));
    let category_id = failure.and_then(|f| f.category_id);
    let http_status = failure.and_then(|f| f.http_status);
    let io_kind = failure.and_then(|f| f.io_kind);
    let os_code = failure.and_then(|f| f.os_code);
    let outcome = outcome_code(report.outcome);
    let reason = report.reason;
    let attempts = report.attempts;
    let categories_checked = u32::try_from(report.categories.len()).unwrap_or(u32::MAX);
    let categories_written = report
        .categories
        .iter()
        .filter(|c| c.action == CategoryAction::Written)
        .count();
    let categories_written = u32::try_from(categories_written).unwrap_or(u32::MAX);
    let bytes_downloaded = report.bytes_downloaded;
    let duration_ms = report.duration_ms;
    if report.outcome == Outcome::Failed {
        tracing::warn!(
            target: "launcher.launch",
            event = "cooked_presync_finished",
            operation,
            outcome,
            reason,
            step,
            category_id,
            http_status,
            io_kind,
            os_code,
            attempts,
            categories_checked,
            categories_written,
            bytes_downloaded,
            duration_ms,
            "cooked presync: finished"
        );
    } else {
        tracing::info!(
            target: "launcher.launch",
            event = "cooked_presync_finished",
            operation,
            outcome,
            reason,
            step,
            category_id,
            http_status,
            io_kind,
            os_code,
            attempts,
            categories_checked,
            categories_written,
            bytes_downloaded,
            duration_ms,
            "cooked presync: finished"
        );
    }
}

/// The closed code of a step.
fn step_code(step: Step) -> &'static str {
    match step {
        Step::Prepare => "prepare",
        Step::Index => "index",
        Step::Plan => "plan",
        Step::Download => "download",
        Step::Stage => "stage",
        Step::Commit => "commit",
    }
}

/// The closed code of an outcome.
fn outcome_code(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::UpToDate => "up_to_date",
        Outcome::Synced => "synced",
        Outcome::Skipped => "skipped",
        Outcome::Failed => "failed",
        Outcome::Cancelled => "cancelled",
    }
}
