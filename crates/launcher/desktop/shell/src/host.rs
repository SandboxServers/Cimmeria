//! One native-selected store, opened lazily on a blocking worker.
use crate::panic_hook;
use crate::startup_event::{self, StartupFacts};
use cimmeria_launcher_engine::{DesktopState, NativeCommand, NativeSnapshot, StorageError};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod adoption;
pub(crate) use adoption::events as adoption_events;
pub(crate) use adoption::refusal::refusal as adoption_refusal;
mod client_changes;
mod client_maintenance;
mod game_telemetry;
mod game_update;
#[cfg(test)]
mod held_download;
mod install;
mod launcher_telemetry;
#[cfg(test)]
mod launcher_telemetry_uat;
pub use adoption::{AdoptionCommand, AdoptionError, AdoptionStatus};
pub use game_telemetry::{GameTelemetryCommand, GameTelemetryStatus};
pub use game_update::{GameUpdateCommand, GameUpdateStatus};
mod updater;
pub use updater::UpdaterCommand;
mod launch;
mod log_upload;
mod migration;
mod physx;
mod windows_prerequisites;
pub use launch::{LaunchCommand, LaunchStatus};
pub use migration::{MigrationCommand, MigrationStatus};
pub use windows_prerequisites::DirectxStatus;
mod repair;
mod runtime_setup;
mod seed_archive;
mod setup_events;
mod summary;
pub(crate) mod telemetry_status;
pub(crate) mod ui_event;
pub use install::{InstallCommand, InstallStatus, JobError};
pub use seed_archive::{SeedArchiveCommand, SeedArchiveError, SeedArchiveStatus};

pub struct NativeHost {
    game_update_offer: Mutex<game_update::Offers>,
    game_update_worker: Mutex<Option<game_update::dispatch::Worker>>,
    updater_config: Option<cimmeria_launcher_engine::updater::Config>,
    updater_shutdown: Option<Arc<dyn Fn() + Send + Sync>>,
    root: PathBuf,
    default_install_directory: Option<PathBuf>,
    #[cfg(target_os = "macos")]
    helper: Option<cimmeria_launcher_engine::mac_wine::HelperResource>,
    #[cfg(target_os = "macos")]
    prerequisite_helper: Option<cimmeria_launcher_engine::mac_wine::PrerequisiteResource>,
    #[cfg(target_os = "macos")]
    runtime_worker: Mutex<Option<cimmeria_launcher_engine::mac_wine::prerequisites::Worker>>,
    state: Mutex<Option<Arc<Mutex<DesktopState>>>>,
    migration_preview: Mutex<Option<migration::Preview>>,
    adoption: Mutex<adoption::Adoption>,
    seed_archive: Mutex<seed_archive::SeedArchive>,
    #[cfg(test)]
    repair_fixture: Option<repair::TestDispatch>,
    #[cfg(test)]
    game_update_fixture: Option<game_update::TestDispatch>,
    #[cfg(test)]
    adoption_fixture: Option<adoption::TestDispatch>,
    /// The `Firesky` tree client-maintenance tests act on, so they never
    /// resolve the player's.
    #[cfg(test)]
    firesky_fixture: Option<PathBuf>,
    launch_resources: Option<cimmeria_launcher_engine::launch::Resources>,
    /// Linux: the verified prerequisite worker in the payload cache; PP-08 reads it.
    #[cfg(target_os = "linux")]
    #[allow(dead_code)]
    prerequisite_worker: Option<PathBuf>,
    launch_worker: Mutex<Option<cimmeria_launcher_engine::launch::Worker>>,
    #[cfg(windows)]
    directx_last:
        Mutex<Option<cimmeria_launcher_engine::runtime_setup::windows_physx::InstallResult>>,
    #[cfg(test)]
    prerequisites_fixture: Option<Vec<&'static str>>,
    /// Home and system-tools roots Proton discovery reads in tests; None = no Proton, so tests never read the player's Steam (R3).
    #[cfg(test)]
    proton_fixture: Option<(PathBuf, PathBuf)>,
    /// The last PhysX result; it outlives a lost reply. The install lock is the shared prerequisite lease.
    physx_last:
        Mutex<Option<cimmeria_launcher_engine::runtime_setup::windows_physx::InstallResult>>,
    repair_worker: Mutex<Option<repair::Worker>>,
    worker: Mutex<Option<cimmeria_launcher_engine::install_worker::Worker>>,
    /// Whether the launcher opened its log file at start; reported on `launcher.startup`.
    log_file_ok: bool,
    /// The one log-ingest sender for this run (LF-15), and its busy flag.
    log_uploader: cimmeria_launcher_engine::log_upload::LogUploader,
}
impl NativeHost {
    pub fn new(root: PathBuf) -> Self {
        Self {
            game_update_offer: Mutex::new(game_update::Offers::default()),
            game_update_worker: Mutex::new(None),
            updater_config: None,
            updater_shutdown: None,
            root,
            default_install_directory: None,
            #[cfg(target_os = "macos")]
            helper: None,
            #[cfg(target_os = "macos")]
            prerequisite_helper: None,
            #[cfg(target_os = "macos")]
            runtime_worker: Mutex::new(None),
            state: Mutex::new(None),
            migration_preview: Mutex::new(None),
            adoption: Mutex::new(adoption::Adoption::default()),
            seed_archive: Mutex::new(seed_archive::SeedArchive::default()),
            #[cfg(test)]
            repair_fixture: None,
            #[cfg(test)]
            game_update_fixture: None,
            #[cfg(test)]
            adoption_fixture: None,
            #[cfg(test)]
            firesky_fixture: None,
            launch_resources: None,
            #[cfg(target_os = "linux")]
            prerequisite_worker: None,
            launch_worker: Mutex::new(None),
            #[cfg(windows)]
            directx_last: Mutex::new(None),
            #[cfg(test)]
            prerequisites_fixture: None,
            #[cfg(test)]
            proton_fixture: None,
            physx_last: Mutex::new(None),
            repair_worker: Mutex::new(None),
            worker: Mutex::new(None),
            log_file_ok: false,
            log_uploader: cimmeria_launcher_engine::log_upload::LogUploader::new(),
        }
    }

    /// Only native app-data resolution supplies this path. It seeds untouched
    /// preferences once, without creating a game directory or changing consent.
    pub fn with_default_install_directory(mut self, directory: PathBuf) -> Self {
        self.default_install_directory = Some(directory);
        self
    }

    /// Set once at start from the telemetry log file; reported on `launcher.startup`.
    pub fn with_log_file(mut self, ok: bool) -> Self {
        self.log_file_ok = ok;
        self
    }

    /// What this start knows about the build and its recorded settings.
    fn startup_facts(&self, state: &DesktopState, previous_run_panicked: bool) -> StartupFacts {
        let preferences = state.preferences();
        StartupFacts {
            version: updater::compiled_version(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            release_build: !cfg!(debug_assertions),
            dll_bundled: launch::dll_bundled(),
            game_diagnostics: state.game_telemetry().is_ok_and(|choice| choice.opted_in),
            setup_diagnostics: preferences.launcher_summary_consent,
            login_servers: u64::try_from(preferences.login_servers.len()).unwrap_or(u64::MAX),
            log_file_ok: self.log_file_ok,
            previous_run_panicked,
        }
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

    /// Linux: the prerequisite worker the payload cache extracted. A build that
    /// pins none, or a root without the file, leaves it absent.
    #[cfg(target_os = "linux")]
    pub fn with_prerequisite_worker(mut self, resource_directory: &std::path::Path) -> Self {
        self.prerequisite_worker = option_env!("CIMMERIA_PREREQUISITE_HELPER_SHA256")
            .map(|_| resource_directory.join("windows/cimmeria-prerequisite-worker.exe"))
            .filter(|path| path.is_file());
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
            let restart = std::env::args_os()
                .any(|arg| arg == cimmeria_launcher_engine::updater::RESTART_ARG);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let mut state = loop {
                match DesktopState::open(&self.root) {
                    Err(StorageError::InUse) if restart && std::time::Instant::now() < deadline => {
                        std::thread::sleep(std::time::Duration::from_millis(50))
                    }
                    result => break result?,
                }
            };
            if let Ok(target) = cimmeria_launcher_engine::updater::InstalledTarget::current() {
                // Only the running binary's compiled version acknowledges a handoff.
                // Failure leaves the updater owner visible and excludes game work.
                // That is the D-LX8 release version the feed signs, not the package's.
                if let Err(error) =
                    state.reconcile_launcher_update(&target, updater::compiled_version())
                {
                    startup_event::update_reconcile_failed(startup_event::reconcile_reason(error));
                }
            }
            // A repair stuck before it claimed any files ends failed (D-PD3).
            if let Err(error) = state.reconcile_unstarted_repair() {
                startup_event::repair_reconcile_failed(error.code());
            }
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
            // Before any Play can queue: an opted-in player's queue is kept for
            // the next session, an opted-out player's deleted unsent.
            if matches!(
                cimmeria_launcher_engine::game_telemetry::transport::recover_now(&state),
                cimmeria_launcher_engine::game_telemetry::transport::Recovery::Untouched
            ) {
                startup_event::update_recovery_failed("choice_unreadable");
            }
            let previous_panic = panic_hook::take_previous_panic(&self.root);
            if let Some(location) = &previous_panic {
                panic_hook::report_previous_panic(location);
            }
            startup_event::emit_startup(&self.startup_facts(&state, previous_panic.is_some()));
            let state = Arc::new(Mutex::new(state));
            summary::start(&state);
            *guard = Some(state);
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
    fn handoffs_are_acknowledged_with_the_release_version_not_the_package_version() {
        // A test build has no CIMMERIA_LAUNCHER_VERSION, so compiled_version()
        // equals the package version and no runtime check can tell them apart.
        // A release is `YYYY.M.D` while the package stays `0.1.0`: acknowledging
        // with the package version would leave every handoff unreconciled. So
        // this pins the startup call itself.
        let source = include_str!("host.rs");
        let start = source.find("reconcile_launcher_update(").unwrap();
        let call = &source[start..start + source[start..].find(");").unwrap()];
        assert!(call.contains("compiled_version()"), "{call}");
        assert!(!call.contains("CARGO_PKG_VERSION"), "{call}");
        // The needle is split so this assert does not match its own text.
        assert!(!source.contains(concat!("version: env!(", "\"CARGO_PKG_VERSION\")")));
    }

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
