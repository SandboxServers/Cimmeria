//! "Changes to your client": what the launcher set up, patched, injected and
//! what the server pushes. Read-only. Scaffold only (LX-02); LX-07 ports
//! `cimmeria_launcher_core::client_changes` here.
use super::*;
#[cfg(test)]
mod tests;

/// A `client_changes_command` request from the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientChangesCommand {
    /// List the changes.
    Inspect { schema_version: u32 },
}

/// Path-free and safe for IPC. LX-07 adds the rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientChangesStatus {
    pub schema_version: u32,
}

/// Serializes as one flat code string, like `StorageError`: `"unavailable"`,
/// or the storage code itself (`"io"`, `"unsupported_schema"`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientChangesError {
    /// This build cannot do it yet.
    Unavailable,
    /// State-directory failure, including an unknown schema.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for ClientChangesError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl DesktopState {
    /// Reads only, so it takes no mutation admission. Anything LX-07 adds
    /// that writes must call `ensure_no_active_operation` first.
    pub fn client_changes(
        &self,
        request: ClientChangesCommand,
    ) -> Result<ClientChangesStatus, ClientChangesError> {
        let ClientChangesCommand::Inspect { schema_version } = request;
        if schema_version != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        Err(ClientChangesError::Unavailable)
    }
}
