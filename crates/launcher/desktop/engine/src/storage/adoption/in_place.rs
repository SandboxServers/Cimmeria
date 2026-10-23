//! In-place adoption (D-LX12). The imported folder is verified against the signed
//! release and becomes the content root. Existing release files stay at their
//! original paths; only explicitly reviewed content replacements are applied.
pub(super) use super::in_place_abandon::{abandon, abandonable};
use super::in_place_check;
use super::in_place_replace::remove_work_folder;
use super::publication::{self, name, plan_name, read_large, write_large, Point};
use super::*;
use crate::install_progress::ProgressSink;
use std::{collections::BTreeMap, io::Write};

pub(super) const OWNER: &str = ".cimmeria-install.json";
/// What an owner root holds. A folder that already has any of these is refused.
pub(super) const MANAGED: [&str; 3] = ["game", OWNER, "content-ready.json"];
pub(super) fn refused(error: Error, why: &str) -> Error {
    let code = rejection::code(&error);
    tracing::warn!(target: "adoption", code, "in-place adoption refused: {why}");
    error
}

/// Admission writes the owner marker, plan and checkpoint before the operation
/// begins, so every interruption after it can be recovered or abandoned. Once
/// admitted it is not cancelled: the moves are renames and finish in moments.
pub(super) fn publish(
    preview: &Preview,
    id: Uuid,
    handle: Uuid,
    choices: Choices,
    cancel: &CancellationToken,
    progress: &ProgressSink,
    mut hook: impl FnMut(Point) -> Result<(), Error>,
) -> Result<Provenance, Error> {
    if id.is_nil() || handle != preview.report.preview_handle {
        return Err(ContractError::IdentityConflict.into());
    }
    preview
        .consent(&choices)
        .map_err(|error| refused(error, "confirmation"))?;
    inventory::check_cancel(cancel)?;
    if let AdoptionSource::Legacy(imported) = &preview.source {
        source::verify_import(imported)?;
    }
    let root = preview.report.destination.clone();
    if inventory::scan(&root, cancel)? != preview.scanned {
        return Err(refused(
            Error::SourceChanged,
            "the folder changed after review",
        ));
    }
    let mut state = preview.state.lock().map_err(|_| StorageError::Io)?;
    preview.preparation.validate(&state)?;
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
    if root.canonicalize()? != root {
        return Err(StorageError::InvalidDirectory.into());
    }
    for entry in MANAGED {
        if std::fs::symlink_metadata(root.join(entry)).is_ok() {
            return Err(refused(
                StorageError::InUse.into(),
                "a launcher-owned name appeared",
            ));
        }
    }
    // The review's checkbox is a promise, not a check: a running game would
    // hold SGW.exe open while its folder is moved out from under it.
    if game_running(&root)? {
        return Err(refused(Error::GameRunning, "SGW.exe is open"));
    }
    // The backup is named by the preview handle at review; admission keeps that name.
    if let Some(backup) = &preview.report.backup {
        if std::fs::symlink_metadata(backup).is_ok() {
            return Err(refused(Error::BackupConflict, "the backup folder exists"));
        }
    }
    let store = state.directory.root.clone();
    let game = root.clone();
    let admission_root = super::direct_paths::Parents::retain_root(&root)?;
    let admitted = admit(preview, &mut state, id, choices, &root, &game, cancel);
    let (plan, provenance, owner) = match admitted {
        Ok(admitted) => admitted,
        Err(error) => {
            let begun = state
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| op.id == id);
            if begun {
                let _ = state.operations_mut().and_then(|op| {
                    op.mark_uncertain(id)
                        .map(|_| ())
                        .map_err(|_| StorageError::PersistenceUncertain)
                });
            } else {
                // No operation names this folder yet, so nothing could recover
                // it: take back the marker and the empty tree, never strand them.
                let _ = admission_root.remove(&root.join(OWNER), false);
            }
            return Err(refused(error, "admission failed"));
        }
    };
    drop(state);
    let result = (|| {
        hook(Point::Plan)?;
        // Renames run without the state guard, so status reads stay responsive.
        move_release_files(&store, &plan, progress, &mut hook)?;
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
        finish(&mut state, &plan, &mut hook)?;
        Ok(provenance)
    })();
    if let Err(error) = &result {
        tracing::error!(target: "adoption", ?error, directory = %root.display(), "in-place adoption interrupted; recover or abandon it");
        let mut state = preview.state.lock().map_err(|_| StorageError::Io)?;
        let _ = state.operations_mut().and_then(|op| {
            op.mark_uncertain(id)
                .map(|_| ())
                .map_err(|_| StorageError::PersistenceUncertain)
        });
    }
    drop(owner);
    if result.is_ok() {
        if let Ok(state) = preview.state.lock() {
            state.log_applied_content("applied");
        }
    }
    result
}

/// The marker, plan and Staged checkpoint, then the journal operation. The
/// returned handle keeps the marker locked until publication ends.
fn admit(
    preview: &Preview,
    state: &mut DesktopState,
    id: Uuid,
    choices: Choices,
    root: &Path,
    game: &Path,
    cancel: &CancellationToken,
) -> Result<(Plan, Provenance, File), Error> {
    let owner_id = Uuid::new_v4();
    let installation = InstallIntent {
        schema_version: 2,
        operation_id: owner_id,
        preferences_revision: preview.before.revision,
        destination: root.to_path_buf(),
        manifest_digest: preview.release.digest(),
        login_servers: source::servers_of(&preview.source, &preview.before),
        backend: preview.preparation.record.descriptor.backend.clone(),
    };
    let (legacy_install_id, import_digest, folder_scan_digest) = publication::pins(&preview.source);
    let provenance = Provenance {
        work_id: id,
        legacy_install_id,
        import_digest,
        reference_digest: preview.expected_digest,
        setup_policy_version: 1,
        folder_scan_digest,
        file_index_sha256: preview.file_index_sha256.clone(),
        check: preview.file_check(),
    };
    let mut after = preview.before.clone();
    after.revision = after.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    after.install_directory = Some(root.to_path_buf());
    let (imported, folder) = publication::planned(&preview.source);
    let mut plan = Plan {
        schema_version: 2,
        installation,
        provenance: provenance.clone(),
        imported,
        folder,
        before: preview.before.clone(),
        after,
        report: preview.report.clone(),
        choices,
        source_digest: digest(&preview.scanned)?,
        stage_identity: inventory::identity(game)?,
        destination_identity: inventory::identity(root)?,
        replacements: Vec::new(),
        incomplete_patches: preview.incomplete_patches(),
    };
    let owner_path = root.join(OWNER);
    let root_guard = super::direct_paths::Parents::new(&plan, &[&owner_path], false)?;
    let mut owner = root_guard.open(&owner_path, true, true)?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    owner.write_all(&serde_json::to_vec(&plan.installation).map_err(|_| StorageError::Corrupt)?)?;
    owner.sync_all()?;
    plan.replacements = stage_replacements(preview, &plan, cancel)?;
    state.save_release_evidence(owner_id, &preview.release)?;
    atomic::write(
        &state.directory.root,
        &format!("install-intent-{owner_id}.json"),
        &plan.installation,
    )?;
    // What each release file held at review: recovery tells the reviewed copy
    // from one something wrote at the old path afterwards (see `move_files`).
    // A missing file held nothing at review, so it has no reviewed copy. Keyed by
    // the release path, and read from the folder's spelling of it: a file a patch
    // adds has no folder copy, and a folder may spell the path in another case.
    let reviewed: Reviewed = preview
        .report
        .files
        .iter()
        .filter(|d| d.classification != Classification::Missing)
        .filter(|d| d.classification != Classification::Extra)
        .filter_map(|d| {
            let entry = preview
                .scanned
                .get(d.source_path.as_deref().unwrap_or(d.path.as_str()))?;
            Some((
                d.path.clone(),
                Hashed {
                    sha256: entry.sha256,
                    size: entry.size,
                },
            ))
        })
        .collect();
    write_large(&state.directory.root, &reviewed_name(id), &reviewed)?;
    write_large(&state.directory.root, &plan_name(id), &plan)?;
    if let Some(tree) = &preview.expected_tree {
        in_place_check::write_expected_guarded(&plan, tree)?;
    }
    // Staged here means the moves are authorized, not that they are done.
    write_large(
        &state.directory.root,
        &name(id),
        &Record {
            plan: plan.clone(),
            phase: Phase::Staged,
        },
    )?;
    let operation_revision = preview.preparation.handoff(state)?;
    state
        .operations_mut()?
        .begin(id, OperationKind::Adopt, plan.digest()?, operation_revision)?;
    state
        .operations_mut()?
        .observe(id, OperationState::Running)?;
    Ok((plan, provenance, owner))
}

/// The verified release files, by index name. The old launcher's ledger is not
/// one of them: the adopted tree gets a synthesized ledger instead.
pub(super) fn release_files(report: &Report) -> Vec<&str> {
    report
        .files
        .iter()
        .filter(|d| d.classification != Classification::Extra)
        .map(|d| d.path.as_str())
        .collect()
}

/// Copies each replacement from the preparation store into `stage/replacements`.
/// Copying, not renaming: the state root may sit on another volume. Each copy is
/// checked against the size and hash the review recorded.
fn stage_replacements(
    preview: &Preview,
    plan: &Plan,
    cancel: &CancellationToken,
) -> Result<Vec<Replacement>, Error> {
    let stage = plan.stage();
    let store = preview.preparation.record.directory.join("replacements");
    let mut staged = Vec::new();
    for replacement in preview.replacements() {
        let source = inventory::native(&store, &replacement.path);
        let entry = inventory::hash(&source, cancel)?;
        if entry.size != replacement.size || entry.sha256 != replacement.sha256 {
            return Err(StorageError::Corrupt.into());
        }
        let dest = inventory::native(&stage.join("replacements"), &replacement.path);
        let guard = super::direct_paths::Parents::new(plan, &[&dest], true)?;
        guard.copy_from(&source, &dest, &entry, cancel)?;
        staged.push(replacement.clone());
    }
    Ok(staged)
}

pub(super) fn present(path: &Path) -> Result<bool, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if super::super::failed_cleanup::plain(&meta) && meta.is_file() => Ok(true),
        Ok(_) => Err(StorageError::UnsafeFile.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Io.into()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Hashed {
    pub(crate) sha256: [u8; 32],
    pub(crate) size: u64,
}
/// Release files by index name, as the review hashed them.
type Reviewed = BTreeMap<String, Hashed>;
fn reviewed_name(id: Uuid) -> String {
    format!("adoption-reviewed-{id}.json")
}
/// The direction of an explicitly reviewed cooked-data normalization.
#[derive(Clone, Copy)]
pub(super) enum Way<'a> {
    Adopt { plan: &'a Plan },
    Abandon { plan: &'a Plan },
}

fn move_release_files(
    _store: &Path,
    plan: &Plan,
    progress: &ProgressSink,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    super::direct::apply(plan, progress, hook)
}

/// Whether the folder's `SGW.exe` is open. Windows refuses an exclusive open
/// of a file another process holds, which a running game always does.
#[cfg(windows)]
fn game_running(root: &Path) -> Result<bool, Error> {
    use std::os::windows::fs::OpenOptionsExt;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    match OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(crate::install_layout::sgw_exe(root))
    {
        Ok(_) => Ok(false),
        Err(error) if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(true),
        // A missing executable fails verification, not this check.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Io.into()),
    }
}
/// Other hosts have no mandatory share modes; the review's confirmation stands.
#[cfg(not(windows))]
fn game_running(_root: &Path) -> Result<bool, Error> {
    Ok(false)
}

/// Shared by publication and explicit recovery. A Staged checkpoint finishes its
/// moves; the release files are then exactly the signed reference.
/// The files the launcher's setup writes in the game folder, relative to it. Both
/// adoption paths flush them before the receipt is written.
pub(super) const SETUP_FILES: [&str; 3] = [
    "Working/Binaries/SGW.exe",
    "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua",
    "launcher-installed.json",
];

/// Flushes the listed files under `root`; a file that is not there is skipped.
pub(super) fn sync_files(root: &Path, paths: &[&str]) -> Result<(), Error> {
    for path in paths {
        match std::fs::OpenOptions::new()
            .write(true)
            .open(root.join(path))
        {
            Ok(file) => file.sync_all()?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn finish(
    state: &mut DesktopState,
    plan: &Plan,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let id = plan.provenance.work_id;
    let root = &plan.installation.destination;
    let game = plan.installation.content_root();
    inventory::same_directory(root, &plan.destination_identity)?;
    inventory::same_directory(&game, &plan.stage_identity)?;
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
    if record.phase == Phase::Staged {
        // A first run finds every file moved; recovery finishes what it left.
        super::direct::resume_files(state, plan, &game)?;

        // A light adoption copies the setup files aside first, so Abandon can still
        // put them back once setup has rewritten them (D-LF2).
        in_place_check::save_setup_originals(plan, &game)?;
        // Checkpointed before setup rewrites any release bytes. A verified adoption
        // is only recoverable from here; moving its files back would not undo the setup.
        record.phase = Phase::Promoted;
        write_large(&state.directory.root, &name(id), &record)?;
    }
    if record.phase == Phase::Promoted {
        // Idempotent, so a recovery after a crash here applies it again.
        super::direct_setup::prepare(plan)?;
        let ledger_path = game.join("launcher-installed.json");
        let ledger_guard = super::direct_paths::Parents::new(plan, &[&ledger_path], false)?;
        ledger_guard.write(
            &ledger_path,
            &serde_json::to_vec_pretty(&reference::ledger(&release, &plan.incomplete_patches))
                .map_err(|_| StorageError::Corrupt)?,
        )?;
        sync_files(&game, &SETUP_FILES)?;
        hook(Point::SetUp)?;
    }
    in_place_check::validate(&game, plan)?;
    let receipt = root.join("content-ready.json");
    let guard = super::direct_paths::Parents::new(plan, &[&receipt], false)?;
    guard.write(
        &receipt,
        &serde_json::to_vec(&plan.installation).map_err(|_| StorageError::Corrupt)?,
    )?;
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
    remove_work_folder(plan);
    if let Some(backup) = &plan.report.backup {
        tracing::info!(target: "adoption", backup = %backup.display(), "in-place adoption: the replaced files are kept in the backup folder");
    }
    tracing::info!(target: "adoption", directory = %root.display(), files = plan.report.files.len(), "adopted the installation in place");
    Ok(())
}
