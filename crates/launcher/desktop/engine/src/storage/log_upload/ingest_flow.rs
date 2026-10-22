//! "Upload debug logs" as a log ingest (LF-15): the launcher's own log lines go
//! to SigNoz through a `purpose = "logs"` session instead of a zip to the blob
//! store. The press is the player's consent (D-LU1), so neither the diagnostics
//! opt-in nor the operation gate matters: an upload sends even while an
//! install is stuck in `ReconciliationRequired`, which is when the logs are
//! most wanted. One upload runs at a time; a second press gets `busy`.
//!
//! Steps, as [`DesktopState::begin_log_upload`] and [`LogIngest::run`] split them:
//! the state mutex is held only to decide and build the target; the file reads,
//! the paced posts and the offsets writes run without it. Nothing here logs a
//! path, URL, token or error text (R5).
use super::ingest::{self, IngestOutcome, Pacer, TokioPacer};
use super::lines::{self, Collected};
use super::offsets::Offsets;
use super::*;
use crate::storage::game_telemetry::launcher_session::{LauncherSender, LauncherTarget};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The app-wide upload state: the one log sender, the cancel that app shutdown
/// pulls, and the flag that makes a second press `busy`. Build it once per run.
pub struct LogUploader {
    /// Exactly one `LauncherSender::logs()` per app run: its session, hold and
    /// mint-failure count outlive a single press.
    sender: Arc<LauncherSender>,
    cancel: CancellationToken,
    busy: Arc<AtomicBool>,
}

impl std::fmt::Debug for LogUploader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogUploader")
            .field("busy", &self.busy.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Default for LogUploader {
    fn default() -> Self {
        Self::new()
    }
}

impl LogUploader {
    /// Builds the one upload state for this app run: one log sender, a fresh cancel token, no upload running.
    pub fn new() -> Self {
        Self {
            sender: Arc::new(LauncherSender::logs()),
            cancel: CancellationToken::new(),
            busy: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ends a running upload at its next chunk boundary with what was sent so far.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// `None` while another upload holds the flag.
    fn try_begin(&self) -> Option<BusyGuard> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| BusyGuard(Arc::clone(&self.busy)))
    }
}

impl Drop for LogUploader {
    // The app is closing: stop a running upload instead of leaving it posting.
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Holds `busy` for the whole upload, however it ends (panic included).
struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// An admitted upload. Holds no state lock; `run` does the file reads and posts.
pub struct LogIngest {
    target: LauncherTarget,
    sender: Arc<LauncherSender>,
    cancel: CancellationToken,
    /// Where `uploaded-log-offsets.json` lives.
    state_dir: PathBuf,
    /// `<app_data>/logs`, where the shell writes the launcher's own logs.
    log_dir: Option<PathBuf>,
    _busy: BusyGuard,
}

impl std::fmt::Debug for LogIngest {
    // The target holds the install identity and the login servers: never dump it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogIngest").finish_non_exhaustive()
    }
}

impl DesktopState {
    /// Answers `Inspect` (`Ready`, or `Unreachable` when no telemetry target can
    /// be built for this install), or admits an `Upload`. `Upload` takes no
    /// operation admission: it writes only its own offsets file. A second
    /// `Upload` while one runs returns `busy`.
    pub fn begin_log_upload(
        &self,
        request: LogUploadCommand,
        uploader: &LogUploader,
    ) -> Result<LogUploadStep, LogUploadError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let finished = |state| Ok(LogUploadStep::Finished(LogUploadStatus::new(state)));
        match request {
            LogUploadCommand::Inspect { .. } => finished(if self.launcher_log_target().is_some() {
                LogUploadState::Ready
            } else {
                LogUploadState::Unreachable
            }),
            LogUploadCommand::Upload { .. } => {
                let Some(busy) = uploader.try_begin() else {
                    return Err(StorageError::Busy.into());
                };
                let Some(target) = self.launcher_log_target() else {
                    return finished(LogUploadState::Unreachable);
                };
                Ok(LogUploadStep::Ingest(LogIngest {
                    target,
                    sender: Arc::clone(&uploader.sender),
                    cancel: uploader.cancel.clone(),
                    state_dir: self.directory.root.clone(),
                    log_dir: self.directory.root.parent().map(|p| p.join("logs")),
                    _busy: busy,
                }))
            }
        }
    }
}

/// What the file reads found, before anything is sent.
struct Pending {
    offsets: Offsets,
    collected: Collected,
    /// Whether the log folder holds any launcher log file at all.
    has_files: bool,
}

impl LogIngest {
    /// Reads the new lines, posts them paced, and moves the offsets forward
    /// after each accepted chunk. Call it without the state mutex.
    pub async fn run(self) -> Result<LogUploadStatus, LogUploadError> {
        self.run_with(&mut TokioPacer::new()).await
    }

    /// `run` with the clock and sleep a test supplies.
    pub(crate) async fn run_with(
        self,
        pacer: &mut impl Pacer,
    ) -> Result<LogUploadStatus, LogUploadError> {
        let Self {
            target,
            sender,
            cancel,
            state_dir,
            log_dir,
            _busy,
        } = self;
        let Some(log_dir) = log_dir else {
            return Ok(status(LogUploadState::NoLogs));
        };
        let read = tokio::task::spawn_blocking({
            let (state_dir, cancel) = (state_dir.clone(), cancel.clone());
            move || read_pending(&state_dir, &log_dir, &cancel)
        })
        .await;
        let read = match read {
            Ok(Ok(read)) => read,
            Ok(Err(error)) => {
                events::collect_failed("lines_read");
                return Err(error);
            }
            Err(_) => {
                events::collect_failed("read_task_failed");
                return Err(LogUploadError::CollectFailed);
            }
        };
        let Pending {
            mut offsets,
            collected,
            has_files,
        } = read;
        if collected.lines.is_empty() {
            return Ok(status(if has_files {
                LogUploadState::NothingNew
            } else {
                LogUploadState::NoLogs
            }));
        }
        let upload_id = new_upload_id();
        let version = format!(
            "desktop-{}",
            crate::launcher_compatibility::compiled_version()
        );
        let rows = ingest::rows(
            &collected.lines,
            ingest::LAUNCHER_LOG_TARGET,
            &upload_id,
            &version,
            std::env::consts::OS,
        );
        if rows.len() != collected.lines.len() {
            events::failed("rows_mismatch");
            return Err(LogUploadError::Network);
        }
        let mut done = 0usize;
        let lines = &collected.lines;
        // The offsets file is a few dozen bytes, so the synchronous save between
        // two posts (3.5 s apart) costs nothing worth a blocking-pool hop.
        let outcome = ingest::send_logs(&sender, &target, &rows, pacer, &cancel, |chunk| {
            let end = (done + chunk).min(lines.len());
            for line in &lines[done..end] {
                offsets.advance(line);
            }
            done = end;
            if offsets.save(&state_dir).is_err() {
                events::offsets_save_failed();
            }
        })
        .await;
        let lines_total = collected.lines_total;
        match outcome {
            IngestOutcome::Sent { lines_sent } => Ok(status(LogUploadState::Sent {
                lines_sent,
                lines_total,
                upload_id,
            })),
            IngestOutcome::Partial {
                lines_sent,
                retry_after_secs,
            } => Ok(status(LogUploadState::Partial {
                lines_sent,
                lines_total,
                upload_id,
                retry_after_secs,
            })),
            // A failed mint is a transient outage (server down, DNS, offline), not a
            // missing target: the player can press again (LF-15 item 3).
            IngestOutcome::Unreachable => Err(LogUploadError::Network),
            // The sender does not keep the status code, so a rejection and a
            // transport failure read the same to the player: try again later.
            IngestOutcome::Failed => Err(LogUploadError::Network),
        }
    }
}

fn status(state: LogUploadState) -> LogUploadStatus {
    LogUploadStatus::new(state)
}

fn read_pending(
    state_dir: &Path,
    log_dir: &Path,
    cancel: &CancellationToken,
) -> Result<Pending, LogUploadError> {
    let offsets = Offsets::load(state_dir);
    let collected = lines::collect(log_dir, &offsets, cancel)?;
    Ok(Pending {
        has_files: !lines::launcher_files(log_dir).is_empty(),
        offsets,
        collected,
    })
}

/// The server's C-LF5 `upload_id`: 16 lowercase hex digits.
fn new_upload_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..16].to_string()
}

#[cfg(test)]
mod id_tests {
    use super::*;

    #[test]
    fn upload_id_is_sixteen_lowercase_hex_digits() {
        let id = new_upload_id();
        assert_eq!(id.len(), 16);
        assert!(id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
        assert_ne!(id, new_upload_id());
    }
}
