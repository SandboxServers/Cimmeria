//! Debug log upload: zip the game's debug logs and session logs and PUT them
//! to the compiled-in log store (`cimmeria_launcher_core::logs`). A repeat of
//! a past upload's content digest sends nothing, so pressing the button twice
//! costs one storage transaction, not two.
//!
//! The shell runs it in three steps so the PUT never holds the state mutex:
//! [`DesktopState::begin_log_upload`] (admission, zip, ledger check), then
//! [`LogStore::put`], then [`DesktopState::finish_log_upload`] (ledger).
use super::*;
use cimmeria_launcher_core::logs;
use cimmeria_launcher_core::state::UploadedLedger;
mod store;
pub use store::LogStore;
#[cfg(test)]
mod tests;

/// The digest ledger, in the launcher state directory. `UploadedLedger` keeps
/// the newest `LEDGER_KEEP` (100) entries.
const LEDGER: &str = "uploaded-logs.json";

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

/// What `Inspect` found or what `Upload` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogUploadState {
    /// This build has no log store compiled in; the view hides the button.
    Disabled,
    /// No owned installation, so there is no game directory to read.
    NotInstalled,
    /// `Inspect`: an upload can run.
    Ready,
    /// `Upload`: the zip was stored.
    Uploaded,
    /// `Upload`: the same logs as a past upload; nothing was sent.
    NothingNew,
    /// `Upload`: the game has written no debug or session logs.
    NoLogs,
}

/// Path-free and URL-free, safe for IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LogUploadStatus {
    pub schema_version: u32,
    pub state: LogUploadState,
}
impl LogUploadStatus {
    fn new(state: LogUploadState) -> Self {
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
    /// The PUT did not complete (connect, TLS, timeout).
    Network,
    /// The store answered with a non-success status.
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
            .finish()
    }
}

impl DesktopState {
    /// Answer `Inspect`, or start an `Upload`. `Upload` takes the full mutation
    /// admission (`ensure_no_active_operation`) before anything else, so the
    /// ledger written after the PUT is always behind it. Known limit: an
    /// operation stuck in `ReconciliationRequired` also refuses this with
    /// `busy`; the view tells the player to finish or reconcile it first.
    pub fn begin_log_upload(
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
        let game = installed.intent.destination.join("game");
        let Some(digest) = logs::compute_content_digest(&game).map_err(collect_failed)? else {
            return finished(LogUploadState::NoLogs);
        };
        if self.uploaded_ledger().contains(&digest) {
            tracing::debug!("log upload skipped: same content as a past upload");
            return finished(LogUploadState::NothingNew);
        }
        // A file deleted between the digest and the zip leaves nothing to send.
        let Some(body) = logs::build_log_zip(&game).map_err(collect_failed)? else {
            return finished(LogUploadState::NoLogs);
        };
        let blob_name = logs::blob_name_for(&digest);
        Ok(LogUploadStep::Send(PendingUpload {
            digest,
            blob_name,
            body,
        }))
    }

    /// Record a stored zip's digest so the same logs are not sent again. The
    /// upload has already succeeded, so a ledger write failure is logged, not
    /// reported: the worst case is one repeat upload of the same logs.
    pub fn finish_log_upload(&self, pending: PendingUpload) -> LogUploadStatus {
        let mut ledger = self.uploaded_ledger();
        ledger.record(pending.digest, pending.blob_name);
        if let Err(error) = ledger.save(&self.directory.root.join(LEDGER)) {
            tracing::warn!(%error, "log upload ledger was not saved");
        }
        LogUploadStatus::new(LogUploadState::Uploaded)
    }

    fn uploaded_ledger(&self) -> UploadedLedger {
        UploadedLedger::load(&self.directory.root.join(LEDGER))
    }
}

fn collect_failed(error: logs::LogError) -> LogUploadError {
    // Local IO and zip errors name only local paths, never the store URL.
    tracing::warn!(%error, "log upload could not collect the logs");
    LogUploadError::CollectFailed
}
