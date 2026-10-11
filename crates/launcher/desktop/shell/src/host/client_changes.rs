//! The read-only "Changes to your client" panel. Scaffold (LX-02): the engine
//! half answers `unavailable` until LX-07.
use super::NativeHost;
use cimmeria_launcher_engine::client_changes::{
    ClientChangesCommand, ClientChangesError, ClientChangesStatus,
};
use cimmeria_launcher_engine::StorageError;

impl NativeHost {
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
    fn the_command_reaches_the_engine_scaffold() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let request = serde_json::from_str(r#"{"command":"inspect","schema_version":1}"#).unwrap();
        assert_eq!(
            host.client_changes_command(request),
            Err(ClientChangesError::Unavailable)
        );
    }
}
