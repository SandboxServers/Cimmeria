//! Native Windows prerequisite detection and explicit, authenticated DirectX setup.
use super::*;
use cimmeria_launcher_engine::runtime_setup::{windows_directx as dx, windows_physx as px};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DirectxStatus {
    schema_version: u32,
    supported: bool,
    missing: Vec<&'static str>,
    installing: bool,
    last_result: Option<px::InstallResult>,
}

/// The state lock protects admission; it is released during downloads and vendor UI.
pub(super) struct SetupLease(pub(super) Arc<Mutex<DesktopState>>);
impl SetupLease {
    #[cfg(any(windows, test))]
    pub(super) fn begin(state: &Arc<Mutex<DesktopState>>) -> Result<Self, StorageError> {
        state
            .lock()
            .map_err(|_| StorageError::Io)?
            .begin_prerequisite_setup()?;
        Ok(Self(state.clone()))
    }
}
impl Drop for SetupLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.end_prerequisite_setup();
        }
    }
}

impl NativeHost {
    pub(super) fn missing_prerequisites(&self, game: Option<&Path>) -> Vec<&'static str> {
        #[cfg(test)]
        if let Some(missing) = &self.prerequisites_fixture {
            return missing.clone();
        }
        #[cfg(windows)]
        {
            let mut missing = Vec::new();
            let binaries = game.map(cimmeria_launcher_engine::install_layout::binaries_dir);
            if !dx::detect(&dx::native::System, binaries.as_deref()).installed() {
                missing.push("directx");
            }
            if !px::detect(&px::native::System).core_present {
                missing.push("physx");
            }
            missing
        }
        #[cfg(not(windows))]
        {
            let _ = game;
            Vec::new()
        }
    }

    pub fn directx_command(&self, request: dx::Command) -> Result<DirectxStatus, dx::Error> {
        let result = self.directx_run(request);
        if let Err(reason) = &result {
            tracing::warn!(target:"launcher.install",event="directx_failed",reason=?reason,"DirectX setup/status request could not complete");
        }
        result
    }

    fn directx_run(&self, request: dx::Command) -> Result<DirectxStatus, dx::Error> {
        if request.schema_version() != 1 {
            return Err(dx::Error::UnsupportedSchema);
        }
        #[cfg(windows)]
        {
            let state = self.store().map_err(|_| dx::Error::Io)?;
            if matches!(request, dx::Command::Install { .. }) {
                let _lease = SetupLease::begin(&state).map_err(|error| {
                    if matches!(error, StorageError::Busy | StorageError::InUse) {
                        dx::Error::Busy
                    } else {
                        dx::Error::Io
                    }
                })?;
                tracing::info!(target:"launcher.install",event="directx_started","DirectX setup started");
                let result = self.install_directx();
                *self.directx_last.lock().map_err(|_| dx::Error::Io)? = Some(result?);
            }
            let mut owner = state.lock().map_err(|_| dx::Error::Io)?;
            let active = owner
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|operation| !operation.state.terminal());
            let game = if active
                && owner
                    .operations()
                    .snapshot()
                    .operation
                    .as_ref()
                    .is_some_and(|operation| {
                        operation.kind == cimmeria_launcher_engine::OperationKind::Launch
                    }) {
                owner.launch_plan().map_err(|_| dx::Error::Io)?.map(|plan| {
                    cimmeria_launcher_engine::install_layout::binaries_dir(
                        &plan.installation.content_root(),
                    )
                })
            } else if owner.prerequisite_setup_active() || active {
                None
            } else {
                owner
                    .installed_content()
                    .map_err(|_| dx::Error::Io)?
                    .map(|installed| {
                        cimmeria_launcher_engine::install_layout::binaries_dir(
                            &installed.intent.content_root(),
                        )
                    })
            };
            Ok(DirectxStatus {
                schema_version: 1,
                supported: true,
                missing: dx::detect(&dx::native::System, game.as_deref()).missing,
                installing: owner.prerequisite_setup_active(),
                last_result: *self.directx_last.lock().map_err(|_| dx::Error::Io)?,
            })
        }
        #[cfg(not(windows))]
        {
            if matches!(request, dx::Command::Install { .. }) {
                return Err(dx::Error::Unavailable);
            }
            Ok(DirectxStatus {
                schema_version: 1,
                supported: false,
                missing: vec![],
                installing: false,
                last_result: None,
            })
        }
    }

    #[cfg(windows)]
    fn install_directx(&self) -> Result<px::InstallResult, dx::Error> {
        let bytes = tokio::runtime::Handle::current().block_on(dx::download())?;
        let (path, guard) = dx::stage(&self.root.join("directx-setup"), &bytes)?;
        let exit = dx::native::run(&path);
        // Close the deny-delete guard before TempPath removes the authenticated installer.
        drop(guard);
        drop(path);
        let result = match exit {
            Err(px::ElevateError::Cancelled) => px::InstallResult::CancelledAtUac {},
            Err(px::ElevateError::Win32(win32_error)) => {
                px::InstallResult::StartFailed { win32_error }
            }
            Ok(code @ (0 | 3010)) => {
                if dx::detect(&dx::native::System, None).installed() {
                    px::InstallResult::Installed {
                        restart_required: code == 3010,
                    }
                } else {
                    px::InstallResult::NotDetected { exit_code: code }
                }
            }
            Ok(1602) => px::InstallResult::CancelledInInstaller {},
            Ok(exit_code) => px::InstallResult::Failed { exit_code },
        };
        tracing::info!(target:"launcher.install",event="directx_finished",outcome=?result,"DirectX installer finished; detection determines readiness");
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vendor_setup_blocks_other_mutations_until_its_lease_is_released() {
        let root = tempfile::tempdir().unwrap();
        let state = Arc::new(Mutex::new(DesktopState::open(root.path()).unwrap()));
        let lease = SetupLease::begin(&state).unwrap();
        assert_eq!(
            state.lock().unwrap().ensure_updater_idle(),
            Err(StorageError::Busy)
        );
        assert!(matches!(SetupLease::begin(&state), Err(StorageError::Busy)));
        drop(lease);
        assert_eq!(state.lock().unwrap().ensure_updater_idle(), Ok(()));
        assert!(SetupLease::begin(&state).is_ok());
    }
}
