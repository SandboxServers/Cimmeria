//! Debug log upload: the game's and the launcher's new log lines are read from
//! their saved offsets and sent to the telemetry server as a `purpose = logs`
//! session (LF-15, LF-19). Nothing is zipped or stored as a file: the blob
//! upload that LF-18 retired is gone.
//!
//! The shell runs it in two steps so neither the log reads nor the send hold
//! the state mutex: [`DesktopState::begin_log_upload`] (admission, game path;
//! locked), then [`LogIngest::run`] (unlocked).
use super::*;
mod events;
mod game_lines;
mod ingest;
mod ingest_flow;
#[cfg(test)]
mod ingest_flow_tests;
mod lines;
mod offsets;
pub use ingest_flow::{LogIngest, LogUploader};
#[cfg(test)]
mod tests;

/// A `log_upload_command` request from the webview. It names no file, folder
/// or URL: native code chooses all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogUploadCommand {
    /// Report whether an upload can run.
    Inspect { schema_version: u32 },
    /// Read the new log lines and send them now.
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
    /// No owned installation, so there is no game directory to read.
    NotInstalled,
    /// `Inspect`: an upload can run.
    Ready,
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
/// text: a transport error's text can include a request URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogUploadError {
    /// The log files could not be read.
    CollectFailed,
    /// The telemetry request did not complete (connect, TLS, timeout).
    Network,
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
    /// Run [`LogIngest::run`] without the state mutex (LF-15).
    Ingest(LogIngest),
}
