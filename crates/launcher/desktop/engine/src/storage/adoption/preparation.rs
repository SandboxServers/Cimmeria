//! Durable reference ownership before extraction; interrupted work never replays.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationRecord {
    pub schema_version: u32,
    pub id: Uuid,
    /// Extraction descriptor only, never an installed owner or Install receipt.
    pub descriptor: InstallIntent,
    pub directory: PathBuf,
    pub directory_identity: inventory::Identity,
}
impl PreparationRecord {
    pub(crate) fn stage(&self) -> PathBuf {
        self.directory.join("raw")
    }
    pub(crate) fn cache(&self) -> PathBuf {
        self.directory.join("cache")
    }
    pub(crate) fn digest(&self) -> Result<[u8; 32], Error> {
        digest(self)
    }
}
fn name(id: Uuid) -> String {
    format!("adoption-reference-{id}.json")
}
pub(crate) fn read_record(
    state: &DesktopState,
    id: Uuid,
) -> Result<Option<PreparationRecord>, StorageError> {
    let record: Option<PreparationRecord> = read(&state.directory.root.join(name(id)))?;
    if let Some(record) = &record {
        if record.schema_version != 1
            || record.id != id
            || record.directory.parent() != Some(state.directory.root.as_path())
            || !record
                .directory
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(".adoption-reference-"))
            || record.descriptor.destination != record.directory
        {
            return Err(StorageError::Corrupt);
        }
    }
    Ok(record)
}
pub(super) struct Ownership {
    state: Arc<Mutex<DesktopState>>,
    pub record: PreparationRecord,
    // Held through the worker and reviewed Preview, including observer loss.
    _lock: File,
    pub uncertain: bool,
    released: bool,
}
impl Ownership {
    pub fn claim(
        state: Arc<Mutex<DesktopState>>,
        request: &PreviewRequest,
        backend: ExtractionBackend,
        servers: Vec<crate::client_setup::LoginServer>,
    ) -> Result<Self, Error> {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        idle(
            &owner,
            request.operation_revision,
            request.preferences_revision,
        )?;
        let work = tempfile::Builder::new()
            .prefix(".adoption-reference-")
            .tempdir_in(&owner.directory.root)?;
        let directory = work.path().to_path_buf();
        let id = Uuid::new_v4();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(
                owner
                    .directory
                    .root
                    .join(format!("adoption-reference-owner-{id}.lock")),
            )?;
        lock.try_lock().map_err(|_| StorageError::InUse)?;
        lock.sync_all()?;
        let record = PreparationRecord {
            schema_version: 1,
            id,
            descriptor: InstallIntent {
                schema_version: 1,
                operation_id: Uuid::new_v4(),
                preferences_revision: request.preferences_revision,
                destination: directory.clone(),
                manifest_digest: request.release.digest(),
                login_servers: servers,
                backend,
            },
            directory_identity: inventory::identity(&directory)?,
            directory,
        };
        owner.save_release_evidence(record.descriptor.operation_id, &request.release)?;
        atomic::write(&owner.directory.root, &name(id), &record)?;
        // Once a durable descriptor exists, failures retain its exact directory.
        let _ = work.keep();
        owner.operations_mut()?.begin(
            id,
            OperationKind::Adopt,
            record.digest()?,
            request.operation_revision,
        )?;
        owner
            .operations_mut()?
            .observe(id, OperationState::Running)?;
        std::fs::create_dir(record.cache())?;
        drop(owner);
        Ok(Self {
            state,
            record,
            _lock: lock,
            uncertain: false,
            released: false,
        })
    }
    pub fn validate(&self, state: &DesktopState) -> Result<(), Error> {
        state.ensure_updater_idle()?;
        if read_record(state, self.record.id)?.as_ref() != Some(&self.record) {
            return Err(StorageError::Corrupt.into());
        }
        let op = state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if op.id != self.record.id
            || op.kind != OperationKind::Adopt
            || op.intent_digest != self.record.digest()?
            || op.state != OperationState::Running
        {
            return Err(ContractError::Busy.into());
        }
        inventory::same_directory(&self.record.directory, &self.record.directory_identity)
    }
    /// Finish reference ownership under the same mutex as copy admission.
    pub fn handoff(&self, state: &mut DesktopState) -> Result<u64, Error> {
        self.validate(state)?;
        state
            .operations_mut()?
            .observe(self.record.id, OperationState::Succeeded)?;
        Ok(state.operations.snapshot().revision)
    }
}
impl Ownership {
    /// Settle this reference from a retained worker that holds no state guard:
    /// wait for the mutex instead of leaving a Running owner with no worker.
    pub fn release(&mut self) {
        if std::mem::replace(&mut self.released, true) {
            return;
        }
        let state = self.state.clone();
        if let Ok(mut state) = state.lock() {
            self.settle(&mut state);
        };
    }
    fn settle(&self, state: &mut DesktopState) {
        if state.ensure_updater_idle().is_err() {
            return;
        }
        if self.uncertain {
            let _ = state.operations_mut().and_then(|ops| {
                ops.mark_uncertain(self.record.id)
                    .map(|_| ())
                    .map_err(|_| StorageError::PersistenceUncertain)
            });
            return;
        }
        if cleanup(state, &self.record).is_err() {
            return;
        }
        if state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == self.record.id && !op.state.terminal())
        {
            if let Ok(ops) = state.operations_mut() {
                let _ = ops.request_cancel(self.record.id);
                let _ = ops.observe(self.record.id, OperationState::Cancelled);
            }
        }
    }
}
impl Drop for Ownership {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        // Never block recursively on a caller's state guard. Failure leaves the
        // durable nonterminal owner for explicit reopen reconciliation.
        let state = self.state.clone();
        let Ok(mut state) = state.try_lock() else {
            return;
        };
        self.settle(&mut state);
    }
}
fn cleanup(state: &DesktopState, record: &PreparationRecord) -> Result<(), Error> {
    let name = format!("adoption-reference-cleanup-{}.json", record.id);
    let saved: Option<PreparationRecord> = read(&state.directory.root.join(&name))?;
    if saved.as_ref().is_some_and(|saved| saved != record) {
        return Err(StorageError::Corrupt.into());
    }
    // Absence is only completion evidence after this exact cleanup was durably
    // authorized. A crash during removal or terminal publication is retryable.
    match std::fs::symlink_metadata(&record.directory) {
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound && saved.as_ref() == Some(record) =>
        {
            return Ok(())
        }
        Err(_) => return Err(StorageError::UnsafeFile.into()),
        Ok(_) => inventory::same_directory(&record.directory, &record.directory_identity)?,
    }
    if saved.is_none() {
        atomic::write(&state.directory.root, &name, record)?;
    }
    // Metadata-only validation avoids hashing an entire multi-gigabyte reference
    // a third time merely to discard it. Links and special files remain refused.
    fn validate(path: &Path, depth: usize, count: &mut usize) -> Result<(), Error> {
        if depth > 64 {
            return Err(StorageError::TooLarge.into());
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            *count += 1;
            if *count > 400_000 {
                return Err(StorageError::TooLarge.into());
            }
            inventory::identity(&entry.path())?;
            if entry.file_type()?.is_dir() {
                validate(&entry.path(), depth + 1, count)?;
            }
        }
        Ok(())
    }
    validate(&record.directory, 0, &mut 0)?;
    std::fs::remove_dir_all(&record.directory)?;
    Ok(())
}
/// Inspect without dispatching extraction or claiming an installed destination.
pub fn inspect_preparation(state: &DesktopState, id: Uuid) -> Result<PreparationRecord, Error> {
    read_record(state, id)?.ok_or(StorageError::Corrupt.into())
}
/// Discover retained preparations, including a record admitted before its journal
/// operation or left after a later operation. Completed cleanup history is hidden.
pub fn list_preparations(state: &DesktopState) -> Result<Vec<PreparationRecord>, Error> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&state.directory.root)? {
        let name = entry?.file_name();
        let Some(id) = name
            .to_str()
            .and_then(|s| s.strip_prefix("adoption-reference-"))
            .and_then(|s| s.strip_suffix(".json"))
        else {
            continue;
        };
        // The cleanup checkpoint has its own namespace.
        if id.starts_with("cleanup-") {
            continue;
        }
        let id = Uuid::parse_str(id).map_err(|_| StorageError::Corrupt)?;
        let record = inspect_preparation(state, id)?;
        let cleaned: Option<PreparationRecord> = read(
            &state
                .directory
                .root
                .join(format!("adoption-reference-cleanup-{id}.json")),
        )?;
        if matches!(std::fs::symlink_metadata(&record.directory), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
            && cleaned.as_ref() == Some(&record)
        {
            continue;
        }
        records.push(record);
        if records.len() > 1024 {
            return Err(StorageError::TooLarge.into());
        }
    }
    records.sort_by_key(|record| record.id);
    Ok(records)
}
/// Explicit cleanup of failed/reopened/orphaned reference work. Wine must first
/// prove helper host absence and stop its exact owned prefix; never re-extract.
pub fn abandon_preparation(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    if state.operations.snapshot().revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let record = inspect_preparation(state, id)?;
    let op = state.operations.snapshot().operation.as_ref();
    let current = op.is_some_and(|op| op.id == id);
    if let Some(op) = op {
        if current {
            if op.kind != OperationKind::Adopt || op.intent_digest != record.digest()? {
                return Err(StorageError::Corrupt.into());
            }
        } else if !op.state.terminal() {
            return Err(ContractError::Busy.into());
        }
    }
    let lock = inventory::open(
        &state
            .directory
            .root
            .join(format!("adoption-reference-owner-{id}.lock")),
    )?;
    lock.try_lock().map_err(|_| StorageError::InUse)?;
    #[cfg(target_os = "macos")]
    let _stopped = if !record.descriptor.backend.is_native() {
        Some(
            crate::mac_wine::repair_recovery::stop_reference(state, id)
                .map_err(|_| StorageError::UnsafeFile)?,
        )
    } else {
        None
    };
    cleanup(state, &record)?;
    if current {
        let op = state.operations.snapshot().operation.as_ref().unwrap();
        if !op.state.terminal() {
            state.operations_mut()?.mark_uncertain(id)?;
            state
                .operations_mut()?
                .reconcile(id, OperationState::Cancelled)?;
        }
    }
    Ok(())
}
