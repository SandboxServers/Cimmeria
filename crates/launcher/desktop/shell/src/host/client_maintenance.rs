//! Reset the client cache or all client state (`engine` `client_maintenance`).
//! The webview names no folder; native code resolves the player's Documents
//! itself.
use super::NativeHost;
use cimmeria_launcher_engine::client_maintenance::{
    ClientMaintenanceCommand, ClientMaintenanceError, ClientMaintenanceStatus,
};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
    /// Run one maintenance request on the shared state, under its mutex.
    pub fn client_maintenance_command(
        &self,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        state.client_maintenance(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconfirmed_reset_all_reaches_the_engine_and_is_refused() {
        // Refused before any deletion, so this never touches the real
        // profile.
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let request = serde_json::from_str(
            r#"{"command":"reset_all_client_state","schema_version":1,"confirmed":false}"#,
        )
        .unwrap();
        assert_eq!(
            host.client_maintenance_command(request),
            Err(ClientMaintenanceError::ConfirmationRequired)
        );
    }
}
