//! Source-preserving staging and explicit checkpoint recovery. No redispatch.
use super::in_place::{sync_files, SETUP_FILES};
use super::*;
use std::io::Write;

const LIMIT: usize = 32 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    Plan,
    Staged,
    BeforePromotion,
    /// In place only: one more release file is now inside the owned tree.
    Moved,
    /// In place only: a modified or unneeded original is set aside and its
    /// replacement is not yet copied into `game`.
    Replaced,
    AfterPromotion,
    /// In place only: the launcher's setup and ledger are written in `game`.
    SetUp,
    Receipt,
    Preferences,
    Published,
    Terminal,
}
pub(super) fn name(id: Uuid) -> String {
    format!("adoption-{id}.json")
}
pub(super) fn plan_name(id: Uuid) -> String {
    format!("adoption-plan-{id}.json")
}
pub(super) fn write_large(root: &Path, name: &str, value: &impl Serialize) -> Result<(), Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::Corrupt)?;
    atomic::write_bytes(root, name, &bytes, LIMIT)?;
    Ok(())
}
pub(super) fn read_large<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let mut bytes = Vec::new();
    inventory::open(path)?
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(StorageError::TooLarge.into());
    }
    serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt.into())
}
fn sync(path: &Path) -> Result<(), Error> {
    // Windows cannot open a directory as a file, so it has no directory flush to
    // make; the entries themselves were already flushed when each file was written.
    if cfg!(windows) {
        return Ok(());
    }
    File::open(path)?.sync_all()?;
    Ok(())
}
fn sync_tree(path: &Path) -> Result<(), Error> {
    for child in std::fs::read_dir(path)? {
        let child = child?;
        if child.file_type()?.is_dir() {
            sync_tree(&child.path())?;
        }
    }
    sync(path)
}
pub(super) fn confirm(
    mut preview: Preview,
    id: Uuid,
    handle: Uuid,
    choices: Choices,
    cancel: CancellationToken,
    progress: &crate::install_progress::ProgressSink,
    hook: impl FnMut(Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    let result = if preview.report.in_place() {
        super::in_place::publish(&preview, id, handle, choices, &cancel, progress, hook)
    } else {
        copy_and_publish(&preview, id, handle, choices, &cancel, progress, hook)
    };
    // Staging no longer needs the reference, and recovery finishes from the stage
    // alone. No state guard is held here, so settle its owner deterministically.
    preview.preparation.release();
    if result.is_ok() {
        if let Ok(state) = preview.state.lock() {
            artifacts::discard_store(&state.directory.root);
        }
    }
    result
}
fn copy_and_publish(
    preview: &Preview,
    id: Uuid,
    handle: Uuid,
    choices: Choices,
    cancel: &CancellationToken,
    progress: &crate::install_progress::ProgressSink,
    mut hook: impl FnMut(Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    use crate::install_progress::ProgressReporter;
    if id.is_nil() || handle != preview.report.preview_handle {
        return Err(ContractError::IdentityConflict.into());
    }
    preview.consent(&choices)?;
    inventory::check_cancel(cancel)?;
    if let AdoptionSource::Legacy(imported) = &preview.source {
        source::verify_import(imported)?;
    }
    if inventory::scan(source_root(&preview.source), cancel)? != preview.scanned {
        return Err(Error::SourceChanged);
    }
    // Admission: the journal's Running Adopt operation, not this guard, excludes
    // other work for the rest of the copy.
    let mut state = preview.state.lock().map_err(|_| StorageError::Io)?;
    preview.preparation.validate(&state)?;
    if state.preferences.revision != preview.before.revision {
        return Err(StorageError::StaleRevision.into());
    }
    if state.preferences != preview.before {
        return Err(StorageError::StaleRevision.into());
    }
    if state.compatibility.for_release(&preview.release).blocks() {
        return Err(Error::LauncherTooOld);
    }
    if std::fs::symlink_metadata(state.directory.root.join(plan_name(id))).is_ok()
        || state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == id)
    {
        return Err(ContractError::IdentityConflict.into());
    }
    let destination = super::super::install_intent::fresh_destination(
        &preview.report.destination,
        &state.directory.root,
    )?;
    if destination != preview.report.destination {
        return Err(StorageError::InvalidDirectory.into());
    }
    // Require an absent destination, rather than claiming a preexisting empty
    // folder whose identity another application may replace during admission.
    std::fs::create_dir(&destination).map_err(|_| StorageError::InUse)?;
    let owner_id = Uuid::new_v4();
    let installation = InstallIntent {
        schema_version: 1,
        operation_id: owner_id,
        preferences_revision: preview.before.revision,
        destination: destination.clone(),
        manifest_digest: preview.release.digest(),
        login_servers: source::servers_of(&preview.source, &preview.before),
        backend: preview.preparation.record.descriptor.backend.clone(),
    };
    let mut owner = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(destination.join(".cimmeria-install.json"))?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    owner.write_all(&serde_json::to_vec(&installation).map_err(|_| StorageError::Corrupt)?)?;
    owner.sync_all()?;
    let stage = destination.join(format!(".cimmeria-adopt-{id}"));
    std::fs::create_dir(&stage)?;
    let (legacy_install_id, import_digest, folder_scan_digest) = pins(&preview.source);
    let provenance = Provenance {
        work_id: id,
        legacy_install_id,
        import_digest,
        reference_digest: preview.expected_digest,
        setup_policy_version: 1,
        folder_scan_digest,
        file_index_sha256: preview.file_index_sha256.clone(),
        check: FileCheck::Full,
    };
    let mut after = preview.before.clone();
    after.revision = after.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    after.install_directory = Some(destination.clone());
    let (imported, folder) = planned(&preview.source);
    let plan = Plan {
        schema_version: 1,
        installation,
        provenance: provenance.clone(),
        imported,
        folder,
        before: preview.before.clone(),
        after,
        report: preview.report.clone(),
        choices,
        source_digest: digest(&preview.scanned)?,
        stage_identity: inventory::identity(&stage)?,
        destination_identity: inventory::identity(&destination)?,
        replacements: Vec::new(),
        incomplete_patches: Vec::new(),
    };
    state.save_release_evidence(owner_id, &preview.release)?;
    atomic::write(
        &state.directory.root,
        &format!("install-intent-{owner_id}.json"),
        &plan.installation,
    )?;
    write_large(&state.directory.root, &plan_name(id), &plan)?;
    sync(&destination)?;
    sync(destination.parent().ok_or(StorageError::InvalidDirectory)?)?;
    let operation_revision = preview.preparation.handoff(&mut state)?;
    state
        .operations_mut()?
        .begin(id, OperationKind::Adopt, plan.digest()?, operation_revision)?;
    state
        .operations_mut()?
        .observe(id, OperationState::Running)?;
    drop(state);
    let result = (|| {
        hook(Point::Plan)?;
        // The multi-gigabyte copy runs without the state guard, so status reads
        // and other views stay responsive. Never invoke this on a command thread.
        let total = preview.expected_paths.len();
        for (current, path) in preview.expected_paths.iter().enumerate() {
            progress.report(crate::install::Progress::Extracting {
                label: "Verified copy".into(),
                current,
                total,
                filename: path.clone(),
            });
            let reusable = preview.report.files.iter().find(|d| {
                &d.path == path
                    && matches!(
                        d.classification,
                        Classification::Matched | Classification::KnownTransform
                    )
            });
            let replacement = preview.replacements().iter().find(|r| &r.path == path);
            if let Some(source_path) = reusable.and_then(|d| d.source_path.as_ref()) {
                inventory::copy(
                    &source_root(&preview.source).join(source_path),
                    &stage.join(path),
                    &preview.scanned[source_path],
                    cancel,
                )?;
            } else if let Some(replacement) = replacement {
                // The replay stored the verified bytes; check them again before use.
                let stored = inventory::native(
                    &preview.preparation.record.directory.join("replacements"),
                    path,
                );
                let entry = inventory::hash(&stored, cancel)?;
                if entry.size != replacement.size || entry.sha256 != replacement.sha256 {
                    return Err(Error::InvalidArtifact);
                }
                inventory::copy(&stored, &stage.join(path), &entry, cancel)?;
            } else if preview.reference.is_none()
                && stage.join(path) == crate::client_setup::login_servers::path(&stage)
            {
                // Setup below writes the login file from the server list whatever
                // the folder held, and the digest check still verifies it.
            } else {
                // Without a reconstructed reference only a replacement supplies the
                // file; a missing one is never skipped.
                let reference = preview.reference.as_ref().ok_or(Error::InvalidArtifact)?;
                let expected = reference.index.get(path).ok_or(Error::InvalidArtifact)?;
                inventory::copy(
                    &reference.prepared.join(path),
                    &stage.join(path),
                    expected,
                    cancel,
                )?;
            }
        }
        if preview.reference.is_none() {
            // The ledger a reconstruction would have saved, synthesized from the release.
            reference::ledger(&preview.release, &[])
                .save(&stage)
                .map_err(|_| StorageError::Io)?;
        }
        progress.report(crate::install::Progress::Extracting {
            label: "Verified copy".into(),
            current: total,
            total,
            filename: String::new(),
        });
        crate::client_setup::prepare(&stage, &plan.installation.login_servers)?;
        if inventory::content_digest(&inventory::scan(&stage, cancel)?)?
            != provenance.reference_digest
        {
            return Err(Error::InvalidArtifact);
        }
        if inventory::scan(source_root(&preview.source), cancel)? != preview.scanned {
            return Err(Error::SourceChanged);
        }
        if let AdoptionSource::Legacy(imported) = &preview.source {
            source::verify_import(imported)?;
        }
        sync_tree(&stage)?;
        sync_files(&stage, &SETUP_FILES)?;
        // Publication is serialized with preference edits again. The operation
        // must still be the one admitted above before any checkpoint is written.
        let mut state = preview.state.lock().map_err(|_| StorageError::Io)?;
        if state.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if !state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == id && op.state == OperationState::Running)
        {
            return Err(ContractError::InvalidTransition.into());
        }
        write_large(
            &state.directory.root,
            &name(id),
            &Record {
                plan: plan.clone(),
                phase: Phase::Staged,
            },
        )?;
        hook(Point::Staged)?;
        inventory::check_cancel(cancel)?;
        finish(&mut state, &plan, &mut hook)?;
        Ok(provenance)
    })();
    let mut state = preview.state.lock().map_err(|_| StorageError::Io)?;
    if result == Err(Error::Cancelled) {
        state.operations_mut()?.request_cancel(id)?;
        state
            .operations_mut()?
            .observe(id, OperationState::Cancelled)?;
    } else if result.is_err() {
        let _ = state.operations_mut().and_then(|op| {
            op.mark_uncertain(id)
                .map(|_| ())
                .map_err(|_| StorageError::PersistenceUncertain)
        });
    }
    drop(state);
    drop(owner);
    result
}
/// Receipt is separately typed provenance, not a fabricated successful Install.
fn finish(
    state: &mut DesktopState,
    plan: &Plan,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let id = plan.provenance.work_id;
    let root = &plan.installation.destination;
    let game = root.join("game");
    let stage = plan.stage();
    inventory::same_directory(root, &plan.destination_identity)?;
    let mut record: Record = read_large(&state.directory.root.join(name(id)))?;
    if record.plan != *plan {
        return Err(StorageError::Corrupt.into());
    }
    let release = state
        .release_for_intent(&plan.installation)
        .map_err(|_| StorageError::Corrupt)?;
    if state.compatibility.for_release(&release).blocks() {
        return Err(Error::LauncherTooOld);
    }
    if state.preferences != plan.before && state.preferences != plan.after {
        return Err(StorageError::StaleRevision.into());
    }
    // A source need not still exist after a durable Staged checkpoint: every
    // copied byte and the captured source were verified before that checkpoint.
    match (
        std::fs::symlink_metadata(&stage),
        std::fs::symlink_metadata(&game),
    ) {
        (Ok(_), Err(e))
            if e.kind() == std::io::ErrorKind::NotFound && record.phase == Phase::Staged =>
        {
            validate_tree(&stage, plan)?;
            hook(Point::BeforePromotion)?;
            rename_exclusive(&stage, &game)?;
            hook(Point::AfterPromotion)?;
            sync(root)?;
        }
        (Err(e), Ok(_)) if e.kind() == std::io::ErrorKind::NotFound => {
            validate_tree(&game, plan)?;
        }
        _ => return Err(StorageError::Corrupt.into()),
    }
    validate_tree(&game, plan)?;
    record.phase = Phase::Promoted;
    write_large(&state.directory.root, &name(id), &record)?;
    atomic::write(root, "content-ready.json", &plan.installation)?;
    state.remember_adopted_content(&plan.installation, &plan.provenance)?;
    hook(Point::Receipt)?;
    if state.preferences == plan.before {
        atomic::write(&state.directory.root, "preferences.json", &plan.after)?;
        state.preferences = plan.after.clone();
    }
    hook(Point::Preferences)?;
    record.phase = Phase::Published;
    write_large(&state.directory.root, &name(id), &record)?;
    hook(Point::Published)?;
    if state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| op.state == OperationState::ReconciliationRequired)
    {
        state
            .operations_mut()?
            .reconcile(id, OperationState::Succeeded)?;
    } else {
        state
            .operations_mut()?
            .observe(id, OperationState::Succeeded)?;
    }
    hook(Point::Terminal)?;
    Ok(())
}
fn validate_tree(path: &Path, plan: &Plan) -> Result<(), Error> {
    inventory::same_directory(path, &plan.stage_identity)?;
    if inventory::content_digest(&inventory::scan(path, &CancellationToken::new())?)?
        != plan.provenance.reference_digest
    {
        return Err(StorageError::Corrupt.into());
    }
    Ok(())
}
fn rename_exclusive(source: &Path, destination: &Path) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source =
            CString::new(source.as_os_str().as_bytes()).map_err(|_| StorageError::UnsafeFile)?;
        let destination = CString::new(destination.as_os_str().as_bytes())
            .map_err(|_| StorageError::UnsafeFile)?;
        // RENAME_EXCL forbids overwriting even an empty racing destination.
        if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) }
            != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let wide =
            |path: &Path| -> Vec<u16> { path.as_os_str().encode_wide().chain([0]).collect() };
        // Without MOVEFILE_REPLACE_EXISTING the move fails on any existing destination.
        if unsafe { MoveFileExW(wide(source).as_ptr(), wide(destination).as_ptr(), 0) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = (source, destination);
        Err(Error::UnsupportedConfiguration)
    }
}
pub fn inspect(state: &DesktopState, id: Uuid) -> Result<Record, Error> {
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id
        || op.kind != OperationKind::Adopt
        || op.intent_digest != plan.digest()?
        || plan.schema_version != 1
        || plan.provenance.work_id != id
    {
        return Err(StorageError::Corrupt.into());
    }
    state
        .release_for_intent(&plan.installation)
        .map_err(|_| StorageError::Corrupt)?;
    let record: Record = read_large(&state.directory.root.join(name(id)))?;
    if record.plan != plan {
        return Err(StorageError::Corrupt.into());
    }
    Ok(record)
}
/// What an interrupted copy still allows. Reading this never mutates or redispatches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interrupted {
    pub directory: PathBuf,
    /// A verified Staged checkpoint exists, so `recover` can finish publication.
    pub recoverable: bool,
    /// Nothing was promoted, so `abandon` can end the operation.
    pub abandonable: bool,
}
pub fn interrupted(state: &DesktopState, id: Uuid) -> Result<Interrupted, Error> {
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id || op.kind != OperationKind::Adopt || op.intent_digest != plan.digest()? {
        return Err(StorageError::Corrupt.into());
    }
    Ok(Interrupted {
        recoverable: inspect(state, id).is_ok(),
        abandonable: if plan.report.in_place() {
            super::in_place::abandonable(state, &plan)
        } else {
            matches!(
                std::fs::symlink_metadata(plan.installation.destination.join("game")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound
            )
        },
        directory: plan.installation.destination,
    })
}
/// Explicit reopen recovery finishes only an already verified Staged checkpoint.
/// Missing checkpoints are quarantined; they never trigger extraction or copying.
pub fn recover(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.operations.snapshot().revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let record = inspect(state, id)?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.state == OperationState::Succeeded {
        return Ok(());
    }
    if op.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    let owner = inventory::open(
        &record
            .plan
            .installation
            .destination
            .join(".cimmeria-install.json"),
    )?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    let saved: InstallIntent = super::super::read_open(&owner)?;
    if saved != record.plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    let result = if record.plan.report.in_place() {
        super::in_place::finish(state, &record.plan, &mut |_| Ok(()))
    } else {
        finish(state, &record.plan, &mut |_| Ok(()))
    };
    if result.is_err() {
        let _ = state.operations_mut().and_then(|op| {
            op.mark_uncertain(id)
                .map(|_| ())
                .map_err(|_| StorageError::PersistenceUncertain)
        });
    }
    result
}
/// Explicit abandonment retains quarantined bytes. It never removes an entire
/// destination based on a missing receipt or a renderer's path claim.
pub fn abandon(state: &mut DesktopState, id: Uuid, revision: u64) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.operations.snapshot().revision != revision {
        return Err(ContractError::StaleRevision.into());
    }
    let plan: Plan = read_large(&state.directory.root.join(plan_name(id)))?;
    let op = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .ok_or(ContractError::UnknownOperation)?;
    if op.id != id || op.kind != OperationKind::Adopt || op.intent_digest != plan.digest()? {
        return Err(StorageError::Corrupt.into());
    }
    if op.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    // In place, abandonment moves the release files back where they were.
    if plan.report.in_place() {
        return super::in_place::abandon(state, &plan);
    }
    if std::fs::symlink_metadata(plan.installation.destination.join("game")).is_ok() {
        return Err(ContractError::InvalidTransition.into());
    }
    state
        .operations_mut()?
        .reconcile(id, OperationState::Cancelled)?;
    Ok(())
}

/// The folder a source is read from: the import's game folder, or the picked root.
fn source_root(source: &AdoptionSource) -> &Path {
    match source {
        AdoptionSource::Legacy(imported) => &imported.source.game_directory,
        AdoptionSource::Folder(folder) => &folder.root,
    }
}

/// What a source pins in its provenance: a legacy import by its install and
/// confirmation, a folder by the scan the player reviewed.
pub(super) fn pins(source: &AdoptionSource) -> (Option<Uuid>, Option<String>, Option<[u8; 32]>) {
    match source {
        AdoptionSource::Legacy(imported) => (
            Some(imported.identity.install_id),
            Some(imported.confirmation.clone()),
            None,
        ),
        AdoptionSource::Folder(folder) => (None, None, Some(folder.scan_digest)),
    }
}

/// The plan's source fields: exactly one of the import and the folder is set.
pub(super) fn planned(
    source: &AdoptionSource,
) -> (Option<migration::LegacyImport>, Option<FolderSource>) {
    match source {
        AdoptionSource::Legacy(imported) => (Some(imported.clone()), None),
        AdoptionSource::Folder(folder) => (None, Some(folder.clone())),
    }
}

pub(super) fn verify_provenance(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<(), Error> {
    published_record(state, intent, provenance).map(|_| ())
}
/// Read-only view for effective settings. Beyond provenance, the Published plan
/// must equal the checkpoint written before the operation began, and the journal
/// digest while that operation is still the latest one.
pub(super) fn published_plan(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<Plan, Error> {
    let plan = published_record(state, intent, provenance)?.plan;
    let checkpoint: Plan = read_large(&state.directory.root.join(plan_name(provenance.work_id)))?;
    let journal = state.operations.snapshot().operation.as_ref();
    if checkpoint != plan
        || journal
            .filter(|op| op.id == provenance.work_id)
            .is_some_and(|op| {
                op.kind != OperationKind::Adopt || Ok(op.intent_digest) != plan.digest()
            })
    {
        return Err(StorageError::Corrupt.into());
    }
    Ok(plan)
}
fn published_record(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<Record, Error> {
    let record: Record = read_large(&state.directory.root.join(name(provenance.work_id)))?;
    if record.plan.schema_version != 1 || provenance.setup_policy_version != 1 {
        return Err(StorageError::UnsupportedSchema.into());
    }
    match record.plan.source()? {
        AdoptionSourceRef::Legacy(legacy) => {
            if provenance.import_digest.as_ref() != Some(&legacy.confirmation)
                || provenance.legacy_install_id != Some(legacy.identity.install_id)
                || provenance.folder_scan_digest.is_some()
            {
                return Err(StorageError::Corrupt.into());
            }
        }
        AdoptionSourceRef::Folder(folder) => {
            if provenance.folder_scan_digest != Some(folder.scan_digest)
                || provenance.import_digest.is_some()
                || provenance.legacy_install_id.is_some()
            {
                return Err(StorageError::Corrupt.into());
            }
        }
    }
    if record.phase != Phase::Published
        || record.plan.installation != *intent
        || record.plan.provenance != *provenance
    {
        return Err(StorageError::Busy.into());
    }
    if state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| op.id == provenance.work_id && op.state != OperationState::Succeeded)
    {
        return Err(StorageError::Busy.into());
    }
    Ok(record)
}
