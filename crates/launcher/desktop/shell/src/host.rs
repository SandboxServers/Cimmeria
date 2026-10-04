//! One native-selected store, opened lazily on a blocking worker.
use cimmeria_launcher_engine::{DesktopState, NativeCommand, NativeSnapshot, StorageError};
use std::{path::PathBuf, sync::Mutex};

pub struct NativeHost {
    root: PathBuf,
    state: Mutex<Option<DesktopState>>,
}
impl NativeHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Mutex::new(None),
        }
    }

    fn with_state<T>(
        &self,
        run: impl FnOnce(&mut DesktopState) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let mut guard = self.state.lock().map_err(|_| StorageError::Io)?;
        if guard.is_none() {
            *guard = Some(DesktopState::open(&self.root)?);
        }
        run(guard.as_mut().ok_or(StorageError::Io)?)
    }

    pub fn dispatch(&self, command: NativeCommand) -> Result<NativeSnapshot, StorageError> {
        self.with_state(|state| state.dispatch(command))
    }

    /// Opening folders is limited to the saved directory; no arbitrary IPC path.
    pub fn install_folder(&self) -> Result<PathBuf, StorageError> {
        self.with_state(|state| {
            let folder = state
                .preferences()
                .install_directory
                .as_ref()
                .ok_or(StorageError::InvalidDirectory)?;
            let folder = folder
                .canonicalize()
                .map_err(|_| StorageError::InvalidDirectory)?;
            if !folder.is_dir() {
                return Err(StorageError::InvalidDirectory);
            }
            Ok(folder)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_reuses_one_store_and_only_opens_the_saved_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        assert!(!root.path().join("state").exists());
        assert!(matches!(
            host.install_folder(),
            Err(StorageError::InvalidDirectory)
        ));
        let folder = root.path().join("game");
        std::fs::create_dir(&folder).unwrap();
        host.dispatch(NativeCommand::SavePreferences {
            schema_version: 1,
            expected_revision: 0,
            install_directory: Some(folder.clone()),
            launcher_summary_consent: false,
        })
        .unwrap();
        assert_eq!(
            host.install_folder().unwrap(),
            folder.canonicalize().unwrap()
        );
        assert_eq!(
            host.dispatch(NativeCommand::Inspect { schema_version: 1 })
                .unwrap()
                .preferences
                .revision,
            1
        );
        assert!(matches!(
            DesktopState::open(&root.path().join("state")),
            Err(StorageError::InUse)
        ));
        drop(host);
        assert_eq!(
            DesktopState::open(&root.path().join("state"))
                .unwrap()
                .preferences()
                .revision,
            1
        );
    }

    #[test]
    fn failed_lazy_open_can_retry_after_other_owner_releases() {
        let root = tempfile::tempdir().unwrap();
        let owner = DesktopState::open(root.path()).unwrap();
        let host = NativeHost::new(root.path().into());
        assert!(matches!(
            host.dispatch(NativeCommand::Inspect { schema_version: 1 }),
            Err(StorageError::InUse)
        ));
        drop(owner);
        assert!(host
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .is_ok());
    }
}
