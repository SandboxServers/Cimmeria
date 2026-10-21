//! PhysX System Software on native Windows (LX-12, D-LX13). The engine half
//! verifies the package and maps results; this file owns the one-at-a-time
//! guard and the last result, so a lost reply can be read back by Inspect.
use super::setup_events;
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

    /// One PhysX command. The engine's `install()` logs the install's start and
    /// outcome, so only an Inspect failure is logged here (LT-15).
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub(super) fn physx_command_with(
        &self,
        request: PhysxCommand,
        probe: &dyn Probe,
        elevate: &dyn Elevate,
    ) -> Result<PhysxStatus, PhysxError> {
        let action = match &request {
            PhysxCommand::Inspect { .. } => "inspect",
            PhysxCommand::Install { .. } => "install",
        };
        let result = self.physx_run(request, probe, elevate);
        if let Err(error) = &result {
            if action == "inspect" {
                let reason = setup_events::code(error);
                setup_events::physx_failed(action, &reason, "other");
            }
        }
        result
    }

    fn physx_run(
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
                setup_events::physx_failed("install", "busy", "other");
                return Err(StorageError::Busy.into());
            }
            struct Running<'a>(&'a std::sync::atomic::AtomicBool);
            impl Drop for Running<'_> {
                fn drop(&mut self) {
                    self.0.store(false, Ordering::SeqCst);
                }
            }
            let _running = Running(&self.physx_installing);
            let source = self.physx_source().inspect_err(log_source_failure)?;
            // The state lock is released: msiexec can wait on the player for minutes.
            let result = windows_physx::install(&source, elevate, probe)?;
            *self.physx_last.lock().map_err(|_| StorageError::Io)? = Some(result);
        }
        self.physx_status(probe)
    }

    /// The install source, read under the store's lock; the lock is released before msiexec.
    fn physx_source(&self) -> Result<windows_physx::InstallSource, PhysxError> {
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        state.physx_install_source()
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

/// Logs an install that the shell's own reads refused. `InstallerUnavailable` is
/// skipped: the engine already logged it; a storage failure is logged here by its code.
fn log_source_failure(error: &PhysxError) {
    let reason = match error {
        PhysxError::InstallerUnavailable => return,
        PhysxError::Storage(storage) => setup_events::code(storage),
        other => setup_events::code(other),
    };
    setup_events::physx_failed("install", &reason, "other");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
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

    /// Starts an install operation in the store, so the next install's read refuses.
    fn begin_operation(host: &NativeHost) {
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

    /// A refused source read (an active operation) logs one `physx_failed` with the
    /// storage code, and no `physx_started`: the engine never ran.
    #[test]
    fn a_refused_source_read_logs_one_physx_failed_and_no_start() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        begin_operation(&host);
        let (result, records) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            host.physx_command_with(INSTALL, &Missing, &Counting::default())
        });
        assert_eq!(result, Err(PhysxError::Storage(StorageError::Busy)));
        let events: Vec<&str> = records
            .iter()
            .filter_map(|r| r.field("event"))
            .filter(|event| event.starts_with("physx_"))
            .collect();
        assert_eq!(events, ["physx_failed"]);
        assert_eq!(records[0].field("reason"), Some("busy"));
        assert_eq!(records[0].field("action"), Some("install"));
    }

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
        begin_operation(&host);
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
        let (result, records) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            host.physx_command_with(INSTALL, &Missing, &launcher)
        });
        assert_eq!(result, Err(PhysxError::Storage(StorageError::Busy)));
        let failed: Vec<_> = records
            .iter()
            .filter(|r| r.field("event") == Some("physx_failed"))
            .collect();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].field("reason"), Some("busy"));
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

    /// A successful Inspect is polled, so it logs no started or finished row.
    #[test]
    fn inspect_success_logs_no_started_or_finished_row() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let (result, records) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            host.physx_command_with(INSPECT, &Missing, &Counting::default())
        });
        assert!(result.is_ok());
        assert!(records
            .iter()
            .all(|r| !matches!(r.field("event"), Some("physx_started" | "physx_finished"))));
    }
}
