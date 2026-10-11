//! Reset the client cache or all client state (`engine` `client_maintenance`).
//! The webview names no folder; native code resolves the player's Documents
//! itself.
use super::NativeHost;
use cimmeria_launcher_engine::client_maintenance::{
    ClientMaintenanceCommand, ClientMaintenanceError, ClientMaintenanceStatus,
};
use cimmeria_launcher_engine::{DesktopState, StorageError};

impl NativeHost {
    /// Run one maintenance request on the shared state, under its mutex.
    pub fn client_maintenance_command(
        &self,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        self.run_client_maintenance(&mut state, request)
    }

    #[cfg(not(test))]
    fn run_client_maintenance(
        &self,
        state: &mut DesktopState,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        state.client_maintenance(request)
    }

    /// Tests never resolve the player's Documents: one that forgot the
    /// fixture panics instead of deleting.
    #[cfg(test)]
    fn run_client_maintenance(
        &self,
        state: &mut DesktopState,
        request: ClientMaintenanceCommand,
    ) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
        let firesky = self
            .firesky_fixture
            .as_deref()
            .expect("client-maintenance tests inject a Firesky root");
        state.client_maintenance_fixture(request, firesky)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webview_requests_reach_the_engine_against_the_injected_tree() {
        let root = tempfile::tempdir().unwrap();
        let firesky = root.path().join("Documents/My Games/Firesky");
        let pak = firesky.join("SGWGame/Cache.en-US/override.pak");
        std::fs::create_dir_all(pak.parent().unwrap()).unwrap();
        std::fs::write(&pak, b"cache").unwrap();
        let mut host = NativeHost::new(root.path().join("state"));
        host.firesky_fixture = Some(firesky.clone());
        let request = |json: &str| serde_json::from_str(json).unwrap();
        assert_eq!(
            host.client_maintenance_command(request(
                r#"{"command":"reset_all_client_state","schema_version":1,"confirmed":false}"#
            )),
            Err(ClientMaintenanceError::ConfirmationRequired)
        );
        assert!(pak.exists());
        let status = host
            .client_maintenance_command(request(r#"{"command":"reset_cache","schema_version":1}"#))
            .unwrap();
        assert_eq!(status.removed, Some(true));
        assert_eq!(
            status.cache.path,
            firesky.join("SGWGame").join("Cache.en-US")
        );
        assert!(!pak.exists());
        assert!(firesky.exists());
    }
}
