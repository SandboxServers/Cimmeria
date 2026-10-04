use super::*;
use futures_util::FutureExt;
use std::{panic::AssertUnwindSafe, sync::Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
pub struct Worker {
    id: Uuid,
    state: Arc<Mutex<DesktopState>>,
    cancel: CancellationToken,
    pub observation: watch::Receiver<Observation>,
}
impl Worker {
    pub fn operation_id(&self) -> Uuid {
        self.id
    }
    /// Cancellation is supported only before native dispatch commits Running.
    pub fn request_cancel(&self) -> Result<(), IntentError> {
        let mut owner = self.state.lock().map_err(|_| StorageError::Io)?;
        let operation = owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if operation.id != self.id {
            return Err(ContractError::IdentityConflict.into());
        }
        if operation.state != OperationState::Starting {
            return Err(ContractError::InvalidTransition.into());
        }
        owner.operations_mut()?.request_cancel(self.id)?;
        self.cancel.cancel();
        Ok(())
    }
}
/// Retained task: dropping the view/Worker stops neither launch nor observation.
pub fn dispatch(state: Arc<Mutex<DesktopState>>, id: Uuid) -> Result<Worker, IntentError> {
    let handle = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (plan, root) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .launch_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        if owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state
            != OperationState::Starting
        {
            return Err(ContractError::InvalidTransition.into());
        }
        // Durable dispatch marker closes the gap before the task is polled.
        if owner.launch_observation()?.is_some() {
            return Err(ContractError::InvalidTransition.into());
        }
        owner.record_launch(&plan, Observation::Preparing)?;
        (plan, owner.state_root().to_path_buf())
    };
    let cancel = CancellationToken::new();
    let (updates, observation) = watch::channel(Observation::Preparing);
    let worker = Worker {
        id,
        state: state.clone(),
        cancel: cancel.clone(),
        observation,
    };
    handle.spawn(async move {
        let result = AssertUnwindSafe(execute(state.clone(), &plan, root, cancel, &updates))
            .catch_unwind()
            .await;
        let result = match result {
            Ok(Ok(result)) => result,
            _ => Observation::Unknown,
        };
        let commit = (|| {
            let mut owner = state.lock().map_err(|_| StorageError::Io)?;
            owner.record_launch(&plan, result)?;
            match result {
                Observation::ProcessExited { code: 0, .. } => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Succeeded)?;
                }
                Observation::ProcessExited { .. } | Observation::NotStarted => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Failed)?;
                }
                Observation::Cancelled => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Cancelled)?;
                }
                _ => {
                    owner.operations_mut()?.mark_uncertain(id)?;
                }
            }
            Ok::<_, IntentError>(())
        })();
        if commit.is_err() {
            if let Ok(mut owner) = state.lock() {
                if let Ok(operations) = owner.operations_mut() {
                    let _ = operations.mark_uncertain(id);
                }
            }
        }
        updates.send_replace(if commit.is_ok() {
            result
        } else {
            Observation::Unknown
        });
    });
    Ok(worker)
}
async fn execute(
    state: Arc<Mutex<DesktopState>>,
    plan: &Plan,
    root: PathBuf,
    cancel: CancellationToken,
    updates: &watch::Sender<Observation>,
) -> Result<Observation, IntentError> {
    if cancel.is_cancelled() {
        return Ok(Observation::Cancelled);
    }
    // All fallible preparation before a host spawn is a known NotStarted result.
    let resources = plan.resources.clone();
    if tokio::task::spawn_blocking(move || resources.verify())
        .await
        .map_err(|_| StorageError::Io)?
        .is_err()
    {
        return Ok(Observation::NotStarted);
    }
    let prepared = plan.clone();
    let setup = tokio::task::spawn_blocking(move || prepare(&prepared, &root))
        .await
        .map_err(|_| StorageError::Io)?;
    let Ok((spec, request, _ownership)) = setup else {
        return Ok(Observation::NotStarted);
    };
    {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        if cancel.is_cancelled() {
            return Ok(Observation::Cancelled);
        }
        owner
            .operations_mut()?
            .observe(plan.id, OperationState::Running)?;
    }
    let outcome = supervisor::run(spec, request, cancel, |observation| {
        state
            .lock()
            .map_err(|_| StorageError::Io)?
            .record_launch(plan, observation)?;
        updates.send_replace(observation);
        Ok(())
    })
    .await;
    Ok(outcome)
}
// Guards live through the helper's full guest lifetime, preventing Repair/uninstall.
enum Ownership {
    Native {
        _owner: File,
    },
    #[cfg(target_os = "macos")]
    Wine {
        _resources: crate::mac_wine::prerequisites::prefix::Resources,
    },
}
fn prepare(
    plan: &Plan,
    root: &Path,
) -> Result<
    (
        crate::helper_supervisor::HelperCommand,
        cimmeria_runtime_probe::game_launch::Request,
        Ownership,
    ),
    IntentError,
> {
    if plan.runtime.is_some() {
        #[cfg(target_os = "macos")]
        {
            let (spec, request, resources) = wine::prepare(plan, root)?;
            return Ok((
                spec,
                request,
                Ownership::Wine {
                    _resources: resources,
                },
            ));
        }
        #[cfg(not(target_os = "macos"))]
        return Err(StorageError::Corrupt.into());
    }
    let _ = root;
    if !cfg!(windows) {
        return Err(StorageError::Corrupt.into());
    }
    prepare_native(plan)
}
fn prepare_native(
    plan: &Plan,
) -> Result<
    (
        crate::helper_supervisor::HelperCommand,
        cimmeria_runtime_probe::game_launch::Request,
        Ownership,
    ),
    IntentError,
> {
    let owner_path = plan.installation.destination.join(".cimmeria-install.json");
    let owner = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&owner_path)
        .map_err(|_| StorageError::Io)?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    // Windows byte-range locks also exclude reads through a second handle.
    let saved: InstallIntent = read_open(&owner)?;
    if saved != plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    let directory = preparation::prepare(plan)?;
    let request = cimmeria_runtime_probe::game_launch::Request {
        schema_version: 1,
        operation_id: plan.id,
        exe: directory.join("SGW.exe"),
        directory: directory.clone(),
        dlls: plan
            .resources
            .client_patches
            .iter()
            .map(|p| p.path().to_path_buf())
            .collect(),
    };
    let environment = [
        "SystemRoot",
        "WINDIR",
        "PATH",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
    .collect();
    Ok((
        crate::helper_supervisor::HelperCommand {
            executable: plan.resources.helper.path().into(),
            arguments: vec![],
            directory,
            environment,
        },
        request,
        Ownership::Native { _owner: owner },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_preparation_reads_locked_owner_and_retains_exclusion() {
        let (root, _state, plan) = super::super::tests::fixture();
        let game = plan.installation.destination.join("game");
        let exe = game.join("Working/Binaries/SGW.exe");
        // Inert PE32 header, sufficient for real client preparation, never spawned.
        let mut bytes = vec![0u8; 0x200];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&0x128u32.to_le_bytes());
        bytes[0x128..0x12c].copy_from_slice(b"PE\0\0");
        bytes[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
        bytes[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
        bytes[0x186..0x188].copy_from_slice(&0x8140u16.to_le_bytes());
        std::fs::write(&exe, bytes).unwrap();

        // Windows exercises the dispatch preparation entry point; other hosts
        // exercise the same native preparation without enabling native dispatch.
        let prepared = if cfg!(windows) {
            prepare(&plan, root.path())
        } else {
            prepare_native(&plan)
        };
        let (command, request, ownership) = prepared.unwrap();
        assert_eq!(request.operation_id, plan.id);
        assert_eq!(request.exe, exe);
        assert_eq!(command.directory, game.join("Working/Binaries"));
        assert_eq!(command.executable, plan.resources.helper.path());
        assert_eq!(&std::fs::read(&exe).unwrap()[0x186..0x188], &[0, 0x81]);
        assert!(crate::client_setup::login_servers::path(&game).is_file());
        let competitor = OpenOptions::new()
            .read(true)
            .write(true)
            .open(plan.installation.destination.join(".cimmeria-install.json"))
            .unwrap();
        assert!(competitor.try_lock().is_err(), "ownership must stay locked");
        drop(ownership);
        competitor.try_lock().unwrap();
    }
}
