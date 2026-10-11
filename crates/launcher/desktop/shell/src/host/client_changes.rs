//! The read-only "Changes to your client" panel. The engine half builds the
//! rows from the installed copy's verified state.
use super::NativeHost;
use cimmeria_launcher_engine::client_changes::{
    ClientChangesCommand, ClientChangesError, ClientChangesStatus,
};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
    /// Run one read-only client-changes request on the shared state.
    pub fn client_changes_command(
        &self,
        request: ClientChangesCommand,
    ) -> Result<ClientChangesStatus, ClientChangesError> {
        let store = self.store()?;
        let state = store.lock().map_err(|_| StorageError::Io)?;
        state.client_changes(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_reaches_the_engine() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":1}"#).unwrap();
        let status = host.client_changes_command(request).unwrap();
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({"schema_version": 1, "installed": false, "changes": []})
        );
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":2}"#).unwrap();
        assert_eq!(
            host.client_changes_command(request),
            Err(ClientChangesError::Storage(StorageError::UnsupportedSchema))
        );
    }
}
