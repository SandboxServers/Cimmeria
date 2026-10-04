//! Source-preserving staging and explicit checkpoint recovery. No redispatch.
use super::*;
use std::io::Write;

const LIMIT: usize = 32 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    Plan,
    Staged,
    BeforePromotion,
    AfterPromotion,
    Receipt,
    Preferences,
    Published,
    Terminal,
}
fn name(id: Uuid) -> String {
    format!("adoption-{id}.json")
}
fn plan_name(id: Uuid) -> String {
    format!("adoption-plan-{id}.json")
}
fn write_large(root: &Path, name: &str, value: &impl Serialize) -> Result<(), Error> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::Corrupt)?;
    atomic::write_bytes(root, name, &bytes, LIMIT)?;
    Ok(())
}
fn read_large<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Error> {
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
    preview: Preview,
    id: Uuid,
    handle: Uuid,
    choices: Choices,
    cancel: CancellationToken,
    mut hook: impl FnMut(Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    if id.is_nil() || handle != preview.report.preview_handle {
        return Err(ContractError::IdentityConflict.into());
    }
    if !choices.old_game_closed
        || (!choices.normalize_managed_files
            && preview.report.files.iter().any(|d| {
                matches!(
                    d.classification,
                    Classification::Modified
                        | Classification::Missing
                        | Classification::KnownTransform
                )
            }))
        || (preview.imported.config.telemetry.opted_in
            && !choices.accept_unavailable_game_telemetry)
    {
        return Err(Error::ConsentRequired);
    }
    inventory::check_cancel(&cancel)?;
    verify_import(&preview.imported)?;
    if inventory::scan(&preview.imported.source.game_directory, &cancel)? != preview.source {
        return Err(Error::SourceChanged);
    }
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
        login_servers: servers(&preview.imported),
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
    let provenance = Provenance {
        work_id: id,
        legacy_install_id: preview.imported.identity.install_id,
        import_digest: preview.imported.confirmation.clone(),
        reference_digest: inventory::content_digest(&preview.reference.index)?,
        setup_policy_version: 1,
    };
    let mut after = preview.before.clone();
    after.revision = after.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    after.install_directory = Some(destination.clone());
    let plan = Plan {
        schema_version: 1,
        installation,
        provenance: provenance.clone(),
        imported: preview.imported.clone(),
        before: preview.before.clone(),
        after,
        report: preview.report.clone(),
        choices,
        source_digest: digest(&preview.source)?,
        stage_identity: inventory::identity(&stage)?,
        destination_identity: inventory::identity(&destination)?,
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
    let result = (|| {
        hook(Point::Plan)?;
        // The state guard serializes publication with preference edits. This is a
        // blocking worker API; never invoke it on the UI/command thread.
        for (path, expected) in &preview.reference.index {
            let reusable = preview.report.files.iter().find(|d| {
                &d.path == path
                    && matches!(
                        d.classification,
                        Classification::Matched | Classification::KnownTransform
                    )
            });
            if let Some(source_path) = reusable.and_then(|d| d.source_path.as_ref()) {
                inventory::copy(
                    &preview.imported.source.game_directory.join(source_path),
                    &stage.join(path),
                    &preview.source[source_path],
                    &cancel,
                )?;
            } else {
                inventory::copy(
                    &preview.reference.prepared.join(path),
                    &stage.join(path),
                    expected,
                    &cancel,
                )?;
            }
        }
        crate::client_setup::prepare(&stage, &plan.installation.login_servers)?;
        if inventory::content_digest(&inventory::scan(&stage, &cancel)?)?
            != provenance.reference_digest
        {
            return Err(Error::InvalidArtifact);
        }
        if inventory::scan(&preview.imported.source.game_directory, &cancel)? != preview.source {
            return Err(Error::SourceChanged);
        }
        verify_import(&preview.imported)?;
        sync_tree(&stage)?;
        write_large(
            &state.directory.root,
            &name(id),
            &Record {
                plan: plan.clone(),
                phase: Phase::Staged,
            },
        )?;
        hook(Point::Staged)?;
        inventory::check_cancel(&cancel)?;
        finish(&mut state, &plan, &mut hook)?;
        Ok(provenance)
    })();
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
    #[cfg(not(target_os = "macos"))]
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
    let result = finish(state, &record.plan, &mut |_| Ok(()));
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
    if std::fs::symlink_metadata(plan.installation.destination.join("game")).is_ok() {
        return Err(ContractError::InvalidTransition.into());
    }
    if op.state != OperationState::ReconciliationRequired {
        return Err(ContractError::InvalidTransition.into());
    }
    state
        .operations_mut()?
        .reconcile(id, OperationState::Cancelled)?;
    Ok(())
}

pub(super) fn verify_provenance(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<(), Error> {
    let record: Record = read_large(&state.directory.root.join(name(provenance.work_id)))?;
    if record.plan.schema_version != 1 || provenance.setup_policy_version != 1 {
        return Err(StorageError::UnsupportedSchema.into());
    }
    if record.plan.imported.confirmation != provenance.import_digest
        || record.plan.imported.identity.install_id != provenance.legacy_install_id
    {
        return Err(StorageError::Corrupt.into());
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
    Ok(())
}
