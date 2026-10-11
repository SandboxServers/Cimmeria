//! PhysX System Software on native Windows (LX-12, D-LX13). The engine half
//! verifies the package and maps results; this file owns the one-at-a-time
//! guard and the last result, so a lost reply can be read back by Inspect.
use super::NativeHost;
use cimmeria_launcher_engine::runtime_setup::windows_physx::{
    self, Elevate, PhysxCommand, PhysxError, PhysxStatus, Probe,
};
use cimmeria_launcher_engine::StorageError;
use std::sync::atomic::Ordering;

impl NativeHost {
    pub fn physx_command(&self, request: PhysxCommand) -> Result<PhysxStatus, PhysxError> {
        #[cfg(windows)]
        {
            self.physx_command_with(
                request,
                &windows_physx::native::System,
                &windows_physx::native::ShellExecuteRunas,
            )
        }
        #[cfg(not(windows))]
        {
            // The macOS path sets PhysX up inside Wine through `prepare_runtime`.
            if request.schema_version() != 1 {
                return Err(StorageError::UnsupportedSchema.into());
            }
            match request {
                PhysxCommand::Inspect { .. } => Ok(PhysxStatus {
                    schema_version: 1,
                    supported: false,
                    detection: None,
                    installer_available: false,
                    installing: false,
                    last_result: None,
                }),
                PhysxCommand::Install { .. } => Err(PhysxError::Unavailable),
            }
        }
    }

    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub(super) fn physx_command_with(
        &self,
        request: PhysxCommand,
        probe: &dyn Probe,
        elevate: &dyn Elevate,
    ) -> Result<PhysxStatus, PhysxError> {
        if request.schema_version() != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        if let PhysxCommand::Install { .. } = request {
            if self.physx_installing.swap(true, Ordering::SeqCst) {
                // The shell has no log subscriber; stderr reaches a support run.
                eprintln!("PhysX install refused: one is already running");
                return Err(StorageError::Busy.into());
            }
            struct Running<'a>(&'a std::sync::atomic::AtomicBool);
            impl Drop for Running<'_> {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::SeqCst);
                }
            }
            let _running = Running(&self.physx_installing);
            let source = {
                let store = self.store()?;
                let mut state = store.lock().map_err(|_| StorageError::Io)?;
                state.physx_install_source()?
            };
            // The state lock is released: msiexec can wait on the player for minutes.
            let result = windows_physx::install(&source, elevate, probe)?;
            *self.physx_last.lock().map_err(|_| StorageError::Io)? = Some(result);
        }
        self.physx_status(probe)
    }

    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    fn physx_status(&self, probe: &dyn Probe) -> Result<PhysxStatus, PhysxError> {
        let installer_available = {
            let store = self.store()?;
            let mut state = store.lock().map_err(|_| StorageError::Io)?;
            state.physx_package()?.is_some()
        };
        Ok(PhysxStatus {
            schema_version: 1,
            supported: true,
            detection: Some(windows_physx::detect(probe)),
            installer_available,
            installing: self.physx_installing.load(Ordering::SeqCst),
            last_result: *self.physx_last.lock().map_err(|_| StorageError::Io)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::runtime_setup::windows_physx::{Detection, ElevateError};
    use std::cell::Cell;
    use std::path::{Path, PathBuf};

    struct Missing;
    impl Probe for Missing {
        fn core_path(&self) -> Option<PathBuf> {
            None
        }
    }
    #[derive(Default)]
    struct Counting(Cell<u32>);
    impl Elevate for Counting {
        fn run_msi(&self, _msi: &Path) -> Result<u32, ElevateError> {
            self.0.set(self.0.get() + 1);
            Ok(0)
        }
    }
    const INSPECT: PhysxCommand = PhysxCommand::Inspect { schema_version: 1 };
    const INSTALL: PhysxCommand = PhysxCommand::Install { schema_version: 1 };

    #[test]
    fn inspect_reports_missing_physx_and_no_installer_without_an_install() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let status = host
            .physx_command_with(INSPECT, &Missing, &Counting::default())
            .unwrap();
        assert!(status.supported && !status.installer_available && !status.installing);
        assert_eq!(
            status.detection,
            Some(Detection {
                core_registered: false,
                core_present: false
            })
        );
        assert_eq!(status.last_result, None);
    }

    #[test]
    fn install_takes_the_mutation_admission_before_looking_for_the_installer() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let launcher = Counting::default();
        assert_eq!(
            host.physx_command_with(INSTALL, &Missing, &launcher),
            Err(PhysxError::InstallerUnavailable)
        );
        {
            let store = host.store().unwrap();
            let mut state = store.lock().unwrap();
            let revision = state.operations().snapshot().revision;
            state
                .operations_mut()
                .unwrap()
                .begin(
                    uuid::Uuid::new_v4(),
                    cimmeria_launcher_engine::OperationKind::Install,
                    [7; 32],
                    revision,
                )
                .unwrap();
        }
        assert_eq!(
            host.physx_command_with(INSTALL, &Missing, &launcher),
            Err(PhysxError::Storage(StorageError::Busy)),
            "an active operation refuses Install"
        );
        assert_eq!(launcher.0.get(), 0);
        assert!(!host.physx_installing.load(Ordering::SeqCst));
    }

    #[test]
    fn a_second_install_while_one_runs_is_busy() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        host.physx_installing.store(true, Ordering::SeqCst);
        let launcher = Counting::default();
        assert_eq!(
            host.physx_command_with(INSTALL, &Missing, &launcher),
            Err(PhysxError::Storage(StorageError::Busy))
        );
        assert!(
            host.physx_installing.load(Ordering::SeqCst),
            "the refused press does not clear the running install's flag"
        );
        assert!(
            host.physx_command_with(INSPECT, &Missing, &launcher)
                .unwrap()
                .installing
        );
        assert_eq!(launcher.0.get(), 0);
    }
}
