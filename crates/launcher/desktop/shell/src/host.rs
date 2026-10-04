//! One native-selected store, opened lazily on a blocking worker.
use cimmeria_launcher_engine::{DesktopState, NativeCommand, NativeSnapshot, StorageError};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod install;
mod repair;
mod runtime_setup;
pub use install::{InstallCommand, InstallStatus, JobError};

pub struct NativeHost {
    root: PathBuf,
    default_install_directory: Option<PathBuf>,
    #[cfg(target_os = "macos")]
    helper: Option<cimmeria_launcher_engine::mac_wine::HelperResource>,
    #[cfg(target_os = "macos")]
    prerequisite_helper: Option<cimmeria_launcher_engine::mac_wine::PrerequisiteResource>,
    #[cfg(target_os = "macos")]
    runtime_worker: Mutex<Option<cimmeria_launcher_engine::mac_wine::prerequisites::Worker>>,
    state: Mutex<Option<Arc<Mutex<DesktopState>>>>,
    #[cfg(test)]
    repair_fixture: Option<repair::TestDispatch>,
    repair_worker: Mutex<Option<repair::Worker>>,
    worker: Mutex<Option<cimmeria_launcher_engine::install_worker::Worker>>,
}
impl NativeHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            default_install_directory: None,
            #[cfg(target_os = "macos")]
            helper: None,
            #[cfg(target_os = "macos")]
            prerequisite_helper: None,
            #[cfg(target_os = "macos")]
            runtime_worker: Mutex::new(None),
            state: Mutex::new(None),
            #[cfg(test)]
            repair_fixture: None,
            repair_worker: Mutex::new(None),
            worker: Mutex::new(None),
        }
    }

    /// Only native app-data resolution supplies this path. It seeds untouched
    /// preferences once, without creating a game directory or changing consent.
    pub fn with_default_install_directory(mut self, directory: PathBuf) -> Self {
        self.default_install_directory = Some(directory);
        self
    }

    #[cfg(target_os = "macos")]
    pub fn with_bundled_helper(mut self, resource_directory: PathBuf) -> Self {
        self.helper = option_env!("CIMMERIA_WINDOWS_HELPER_SHA256").and_then(|expected| {
            cimmeria_launcher_engine::mac_wine::HelperResource::open(
                resource_directory.join("windows/cimmeria-archive-worker.exe"),
                expected,
            )
            .ok()
        });
        self.prerequisite_helper =
            option_env!("CIMMERIA_PREREQUISITE_HELPER_SHA256").and_then(|expected| {
                cimmeria_launcher_engine::mac_wine::PrerequisiteResource::open(
                    resource_directory.join("windows/cimmeria-prerequisite-worker.exe"),
                    expected,
                )
                .ok()
            });
        self
    }

    fn with_state<T>(
        &self,
        run: impl FnOnce(&mut DesktopState) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let state = self.store()?;
        let mut state = state.lock().map_err(|_| StorageError::Io)?;
        run(&mut state)
    }

    fn store(&self) -> Result<Arc<Mutex<DesktopState>>, StorageError> {
        let mut guard = self.state.lock().map_err(|_| StorageError::Io)?;
        if guard.is_none() {
            let mut state = DesktopState::open(&self.root)?;
            if state.preferences().revision == 0
                && state.preferences().install_directory.is_none()
                && state.operations().snapshot().operation.is_none()
            {
                if let Some(directory) = &self.default_install_directory {
                    // Windows game data belongs in LocalAppData, not a roaming
                    // profile. Its app parent may differ from the settings root.
                    if !directory.is_absolute() {
                        return Err(StorageError::InvalidDirectory);
                    }
                    let parent = directory.parent().ok_or(StorageError::InvalidDirectory)?;
                    std::fs::create_dir_all(parent).map_err(|_| StorageError::Io)?;
                    let consent = state.preferences().launcher_summary_consent;
                    state.save_preferences(Some(directory.clone()), consent, 0)?;
                }
            }
            *guard = Some(Arc::new(Mutex::new(state)));
        }
        guard.as_ref().cloned().ok_or(StorageError::Io)
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
    #[test]
    fn fresh_host_defaults_once_without_creating_game_files_or_enabling_consent() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("Stargate Worlds");
        let host = NativeHost::new(root.path().join("state"))
            .with_default_install_directory(directory.clone());
        let snapshot = host
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .unwrap();
        assert_eq!(
            snapshot.preferences.install_directory,
            Some(directory.clone())
        );
        assert_eq!(snapshot.preferences.revision, 1);
        assert!(!snapshot.preferences.launcher_summary_consent);
        assert!(!directory.exists());
        drop(host);
        let host = NativeHost::new(root.path().join("state"))
            .with_default_install_directory(directory.clone());
        let reopened = host
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .unwrap();
        assert_eq!(
            reopened.preferences.install_directory,
            Some(directory.clone())
        );
        assert_eq!(reopened.preferences.revision, 1);
        host.dispatch(NativeCommand::SavePreferences {
            schema_version: 1,
            expected_revision: 1,
            install_directory: None,
            launcher_summary_consent: true,
        })
        .unwrap();
        drop(host);
        let reopened =
            NativeHost::new(root.path().join("state")).with_default_install_directory(directory);
        let snapshot = reopened
            .dispatch(NativeCommand::Inspect { schema_version: 1 })
            .unwrap();
        assert_eq!(snapshot.preferences.install_directory, None);
        assert_eq!(snapshot.preferences.revision, 2);
        assert!(snapshot.preferences.launcher_summary_consent);
    }
}
