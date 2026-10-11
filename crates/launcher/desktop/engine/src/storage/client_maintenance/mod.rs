//! Reset the game client's cache, or all of its per-user state, under the
//! player's Documents. Scaffold only (LX-02); LX-06 ports
//! `cimmeria_launcher_core::client_paths` here.
use super::*;
#[cfg(test)]
mod tests;

/// A `client_maintenance_command` request from the webview. It names no
/// folder: native code resolves the player's Documents itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMaintenanceCommand {
    /// Report what each reset would remove.
    Inspect { schema_version: u32 },
    /// `Firesky\SGWGame\Cache.en-US` only.
    ResetCache { schema_version: u32 },
    /// The whole `Firesky` tree. Refused unless `confirmed`.
    ResetAllClientState {
        schema_version: u32,
        confirmed: bool,
    },
}
impl ClientMaintenanceCommand {
    fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version }
            | Self::ResetCache { schema_version }
            | Self::ResetAllClientState { schema_version, .. } => *schema_version,
        }
    }
}

/// Path-free and safe for IPC. LX-06 adds what `Inspect` reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClientMaintenanceStatus {
    pub schema_version: u32,
}

/// Serializes as one flat code string, like `StorageError`: `"unavailable"`,
/// or the storage code itself (`"busy"`, `"unsupported_schema"`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientMaintenanceError {
    /// This build cannot do it yet.
    Unavailable,
    /// Admission or state-directory failure, including an unknown schema.
    #[serde(untagged)]
    Storage(StorageError),
}
impl From<StorageError> for ClientMaintenanceError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl DesktopState {
    /// Answer one maintenance request. Both resets take the full mutation
    /// admission (`ensure_no_active_operation`) before anything is deleted.
    pub fn client_maintenance(
        &mut self,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        match request {
            ClientMaintenanceCommand::Inspect { .. } => Err(ClientMaintenanceError::Unavailable),
            ClientMaintenanceCommand::ResetCache { .. }
            | ClientMaintenanceCommand::ResetAllClientState { .. } => {
                // Both resets delete files: this admission (the updater gate
                // and a refusal while Play or any operation runs) stays ahead
                // of the first deletion.
                // TODO(LX-06): refuse `confirmed: false` before deleting anything.
                self.ensure_no_active_operation()?;
                Err(ClientMaintenanceError::Unavailable)
            }
        }
    }
}
