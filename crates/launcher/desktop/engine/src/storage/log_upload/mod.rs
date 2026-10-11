//! Debug log upload: zip the game's debug logs and session folders and PUT
//! them to the compiled-in log store. Scaffold only (LX-02); LX-05 ports
//! `cimmeria_launcher_core::logs` here.
use super::*;
#[cfg(test)]
mod tests;

/// A `log_upload_command` request from the webview. It names no file, folder
/// or URL: native code chooses all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum LogUploadCommand {
    /// Report what an upload would send and the last upload's result.
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

/// Path-free and safe for IPC. LX-05 adds what `Inspect` reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogUploadStatus {
    pub schema_version: u32,
}

/// Serializes as one flat code string, like `StorageError`: `"unavailable"`,
/// or the storage code itself (`"busy"`, `"unsupported_schema"`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogUploadError {
    /// This build cannot do it yet.
    Unavailable,
    /// Admission or state-directory failure, including an unknown schema.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for LogUploadError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl DesktopState {
    /// Answer one log-upload request. `Upload` takes the full mutation
    /// admission (`ensure_no_active_operation`) before anything is written.
    pub fn log_upload(
        &mut self,
        request: LogUploadCommand,
    ) -> Result<LogUploadStatus, LogUploadError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        match request {
            LogUploadCommand::Inspect { .. } => Err(LogUploadError::Unavailable),
            LogUploadCommand::Upload { .. } => {
                // The upload ledger is written to the state directory, so this
                // admission must stay ahead of the first write.
                self.ensure_no_active_operation()?;
                Err(LogUploadError::Unavailable)
            }
        }
    }
}
