//! Debug log upload: zip the game's debug logs and session logs and PUT them
//! to the compiled-in log store (`cimmeria_launcher_core::logs`). A repeat of
//! a past upload's content digest sends nothing, so pressing the button twice
//! costs one storage transaction, not two.
//!
//! The shell runs it in four steps so neither the log reads nor the PUT hold
//! the state mutex: [`DesktopState::begin_log_upload`] (admission, game path,
//! ledger load; locked), [`LogCollection::collect`] (size cap, digest, ledger
//! check, zip; unlocked), [`LogStore::put`] (unlocked), then
//! [`DesktopState::finish_log_upload`] (ledger; locked).
use super::*;
use crate::storage::game_telemetry::launcher_event::Redaction;
use cimmeria_launcher_core::logs;
use cimmeria_launcher_core::state::UploadedLedger;
mod events;
mod ingest;
mod ingest_flow;
#[cfg(test)]
mod ingest_flow_tests;
mod launcher_logs;
mod lines;
mod offsets;
mod store;
pub use ingest_flow::{LogIngest, LogUploader};
pub use store::{LogStore, StoredUpload};
#[cfg(test)]
mod tests;

/// The digest ledger, in the launcher state directory. `UploadedLedger` keeps
/// the newest `LEDGER_KEEP` (100) entries.
const LEDGER: &str = "uploaded-logs.json";

/// The most log bytes one upload reads and sends. The game keeps every
/// session's logs, so an old install can hold far more than a triage needs;
/// the oldest are left out first.
pub const MAX_UPLOAD_BYTES: u64 = 64 * 1024 * 1024;

/// A `log_upload_command` request from the webview. It names no file, folder
/// or URL: native code chooses all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogUploadCommand {
    /// Report whether an upload can run.
    Inspect { schema_version: u32 },
    /// Zip and send the logs now.
    Upload { schema_version: u32 },
}
impl LogUploadCommand {
    fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version } | Self::Upload { schema_version } => *schema_version,
        }
    }
}

/// What `Inspect` found or what `Upload` did. Serializes flat with the status:
/// `{"schema_version":1,"state":"sent","lines_sent":3,...}`; a state with no
/// fields adds none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LogUploadState {
    /// This build has no log store compiled in; the view hides the button.
    Disabled,
    /// No owned installation, so there is no game directory to read.
    NotInstalled,
    /// `Inspect`: an upload can run.
    Ready,
    /// `Upload`: the zip was stored.
    Uploaded,
    /// `Upload`: the zip was stored, but older logs were left out to stay
    /// under [`MAX_UPLOAD_BYTES`].
    UploadedNewest,
    /// `Upload`: the same logs as a past upload; nothing was sent.
    NothingNew,
    /// `Upload`: the game has written no debug or session logs.
    NoLogs,
    /// `Upload` (LF-15): every new launcher log line was accepted.
    /// `lines_total` is how many new lines there were before the 20,000-line cap.
    Sent {
        lines_sent: u64,
        lines_total: u64,
        upload_id: String,
    },
    /// `Upload` (LF-15): the server asked for a pause (or the app is closing);
    /// the lines not yet sent go on the next press, after `retry_after_secs`.
    Partial {
        lines_sent: u64,
        lines_total: u64,
        upload_id: String,
        retry_after_secs: u64,
    },
    /// `Inspect` or `Upload` (LF-15): no telemetry target can be built for
    /// this install. A failed mint is `LogUploadError::Network`, not this state.
    Unreachable,
}

/// Path-free and URL-free, safe for IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogUploadStatus {
    pub schema_version: u32,
    #[serde(flatten)]
    pub state: LogUploadState,
}
impl LogUploadStatus {
    pub(crate) fn new(state: LogUploadState) -> Self {
        Self {
            schema_version: 1,
            state,
        }
    }
}

/// Serializes as one flat code string, like `StorageError`. No variant carries
/// text: a transport error's text includes the request URL, and with it the
/// SAS signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogUploadError {
    /// The log files could not be read or zipped.
    CollectFailed,
    /// Every log file is larger than [`MAX_UPLOAD_BYTES`] on its own.
    LogsTooLarge,
    /// The PUT did not complete (connect, TLS, timeout).
    Network,
    /// The store answered with a non-success status, or a redirect.
    Rejected,
    /// Admission or state-directory failure, including an unknown schema and
    /// `busy` while another operation is nonterminal.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for LogUploadError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

/// What `begin_log_upload` decided.
#[derive(Debug)]
pub enum LogUploadStep {
    /// Nothing to collect; report this status.
    Finished(LogUploadStatus),
    /// Run [`LogCollection::collect`] without the state mutex. Only the blob
    /// path (`begin_blob_upload`, removed by LF-18) returns this.
    Collect(LogCollection),
    /// Run [`LogIngest::run`] without the state mutex (LF-15).
    Ingest(LogIngest),
}

/// An admitted upload: where the logs are and which digests were sent before.
/// Holds no lock; it is a snapshot taken under one.
#[derive(Debug)]
pub struct LogCollection {
    game: PathBuf,
    ledger: UploadedLedger,
    max_bytes: u64,
    /// `<app_data>/logs`, where the shell writes the launcher's own logs.
    launcher_logs: Option<PathBuf>,
    redaction: Redaction,
}

/// What `collect` decided.
#[derive(Debug)]
pub enum CollectedLogs {
    /// Nothing to send; report this status.
    Finished(LogUploadStatus),
    /// PUT this with [`LogStore::put`], then record it with
    /// [`DesktopState::finish_log_upload`].
    Send(PendingUpload),
}

/// One zip waiting for its PUT.
pub struct PendingUpload {
    digest: String,
    blob_name: String,
    body: Vec<u8>,
    trimmed: bool,
}
impl PendingUpload {
    /// `logs/<host>-<utc>-<digest prefix>.zip`.
    pub fn blob_name(&self) -> &str {
        &self.blob_name
    }
}
impl std::fmt::Debug for PendingUpload {
    // The body is a zip of the player's logs: never dump it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingUpload")
            .field("blob_name", &self.blob_name)
            .field("bytes", &self.body.len())
            .field("trimmed", &self.trimmed)
            .finish()
    }
}

impl DesktopState {
    /// The pre-LF-15 blob upload: answer `Inspect`, or admit an `Upload`.
    /// `Upload` takes the full mutation admission (`ensure_no_active_operation`)
    /// before anything else. Nothing in the shell calls it any more; LF-18
    /// deletes it with the blob store.
    /// The log reads and the PUT run after the lock is released, so
    /// [`Self::finish_log_upload`] takes the admission again before it writes
    /// the ledger. Known limit: an operation stuck in `ReconciliationRequired`
    /// also refuses this with `busy`; the view tells the player to finish or
    /// reconcile it first.
    pub fn begin_blob_upload(
        &self,
        request: LogUploadCommand,
        store: Option<&LogStore>,
    ) -> Result<LogUploadStep, LogUploadError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let upload = matches!(request, LogUploadCommand::Upload { .. });
        if upload {
            self.ensure_no_active_operation()?;
        }
        let finished = |state| Ok(LogUploadStep::Finished(LogUploadStatus::new(state)));
        if store.is_none() {
            return finished(LogUploadState::Disabled);
        }
        let Some((installed, _)) = self.installed_for_launch()? else {
            return finished(LogUploadState::NotInstalled);
        };
        if !upload {
            return finished(LogUploadState::Ready);
        }
        Ok(LogUploadStep::Collect(LogCollection {
            game: installed.intent.destination.join("game"),
            ledger: self.uploaded_ledger(),
            max_bytes: MAX_UPLOAD_BYTES,
            launcher_logs: self.directory.root.parent().map(|p| p.join("logs")),
            redaction: Redaction::from_env(),
        }))
    }

    /// Record a stored zip's digest so the same logs are not sent again. The
    /// upload has already succeeded, so it reports its upload state whatever
    /// happens here: a ledger write failure is logged, and so is an operation
    /// that became active during the unlocked PUT (the ledger is a disk write,
    /// so it waits behind the gate like any other). Either way the worst case
    /// is one repeat upload of the same logs.
    pub fn finish_log_upload(&self, stored: StoredUpload) -> LogUploadStatus {
        let state = if stored.trimmed {
            LogUploadState::UploadedNewest
        } else {
            LogUploadState::Uploaded
        };
        if let Err(error) = self.ensure_no_active_operation() {
            events::ledger_skipped(error.code());
            return LogUploadStatus::new(state);
        }
        let mut ledger = self.uploaded_ledger();
        ledger.record(stored.digest, stored.blob_name);
        if ledger.save(&self.directory.root.join(LEDGER)).is_err() {
            events::ledger_save_failed();
        }
        LogUploadStatus::new(state)
    }

    fn uploaded_ledger(&self) -> UploadedLedger {
        UploadedLedger::load(&self.directory.root.join(LEDGER))
    }
}

impl LogCollection {
    /// Blocking file IO; call it without the state mutex. The cap is applied
    /// from file metadata before any log is read.
    pub fn collect(self) -> Result<CollectedLogs, LogUploadError> {
        let finished = |state| Ok(CollectedLogs::Finished(LogUploadStatus::new(state)));
        let selection =
            logs::select_log_inputs(&self.game, self.max_bytes).map_err(collect_failed)?;
        if selection.is_empty() {
            if selection.left_out() > 0 {
                events::logs_too_large(selection.left_out(), self.max_bytes);
                return Err(LogUploadError::LogsTooLarge);
            }
            return finished(LogUploadState::NoLogs);
        }
        if selection.left_out() > 0 {
            events::logs_trimmed(selection.left_out(), self.max_bytes);
        }
        let Some(digest) = selection.digest(&self.game).map_err(collect_failed)? else {
            return finished(LogUploadState::NoLogs);
        };
        if self.ledger.contains(&digest) {
            events::nothing_new();
            return finished(LogUploadState::NothingNew);
        }
        let Some(body) = selection.zip(&self.game).map_err(collect_failed)? else {
            return finished(LogUploadState::NoLogs);
        };
        let entries = match &self.launcher_logs {
            Some(dir) => {
                launcher_logs::entries(dir, &self.redaction, launcher_logs::LAUNCHER_LOG_CAP)
            }
            None => Vec::new(),
        };
        let body = launcher_logs::append(body, &entries)?;
        let blob_name = logs::blob_name_for(&digest);
        Ok(CollectedLogs::Send(PendingUpload {
            digest,
            blob_name,
            body,
            trimmed: selection.left_out() > 0,
        }))
    }

    #[cfg(test)]
    fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }
}

fn collect_failed(error: logs::LogError) -> LogUploadError {
    // Local IO and zip errors name only local paths, never the store URL.
    events::collect_failed(events::log_error_reason(&error));
    LogUploadError::CollectFailed
}
