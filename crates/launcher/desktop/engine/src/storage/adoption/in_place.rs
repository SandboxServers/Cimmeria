//! In-place adoption (D-LX12). The imported folder is verified against the signed
//! release and becomes the owner root. Its release files move by rename into the
//! owned `game` tree, so nothing is copied; every other file stays where it was,
//! outside that tree, and no later Repair, Update or Uninstall touches it.
use super::publication::{name, plan_name, read_large, write_large, Point};
use super::*;
use crate::install_progress::{ProgressReporter, ProgressSink};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

pub(super) const OWNER: &str = ".cimmeria-install.json";
/// What an owner root holds. A folder that already has any of these is refused.
pub(super) const MANAGED: [&str; 3] = ["game", OWNER, "content-ready.json"];

pub(super) fn refused(error: Error, why: &str) -> Error {
    tracing::warn!(target: "adoption", ?error, "in-place adoption refused: {why}");
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
    verify_import(&preview.imported)?;
    let root = preview.report.destination.clone();
    if inventory::scan(&root, cancel)? != preview.source {
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
    let store = state.directory.root.clone();
    let game = root.join("game");
    std::fs::create_dir(&game).map_err(|_| StorageError::InUse)?;
    let admitted = admit(preview, &mut state, id, choices, &root, &game);
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
                let _ = std::fs::remove_file(root.join(OWNER));
                let _ = std::fs::remove_dir(&game);
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
) -> Result<(Plan, Provenance, File), Error> {
    let owner_id = Uuid::new_v4();
    let installation = InstallIntent {
        schema_version: 1,
        operation_id: owner_id,
        preferences_revision: preview.before.revision,
        destination: root.to_path_buf(),
        manifest_digest: preview.release.digest(),
        login_servers: servers(&preview.imported),
        backend: preview.preparation.record.descriptor.backend.clone(),
    };
    let mut owner = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(root.join(OWNER))?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    owner.write_all(&serde_json::to_vec(&installation).map_err(|_| StorageError::Corrupt)?)?;
    owner.sync_all()?;
    let provenance = Provenance {
        work_id: id,
        legacy_install_id: preview.imported.identity.install_id,
        import_digest: preview.imported.confirmation.clone(),
        reference_digest: inventory::content_digest(&preview.reference.index)?,
        setup_policy_version: 1,
    };
    let mut after = preview.before.clone();
    after.revision = after.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
    after.install_directory = Some(root.to_path_buf());
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
        stage_identity: inventory::identity(game)?,
        destination_identity: inventory::identity(root)?,
    };
    state.save_release_evidence(owner_id, &preview.release)?;
    atomic::write(
        &state.directory.root,
        &format!("install-intent-{owner_id}.json"),
        &plan.installation,
    )?;
    // What each release file held at review: recovery tells the reviewed copy
    // from one something wrote at the old path afterwards (see `move_files`).
    let reviewed: Reviewed = release_files(&preview.report)
        .into_iter()
        .map(|path| {
            let entry = preview.source.get(path).ok_or(StorageError::Corrupt)?;
            Ok((
                path.to_string(),
                Hashed {
                    sha256: entry.sha256,
                    size: entry.size,
                },
            ))
        })
        .collect::<Result<_, Error>>()?;
    write_large(&state.directory.root, &reviewed_name(id), &reviewed)?;
    write_large(&state.directory.root, &plan_name(id), &plan)?;
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
fn release_files(report: &Report) -> Vec<&str> {
    report
        .files
        .iter()
        .filter(|d| d.classification != Classification::Extra)
        .map(|d| d.path.as_str())
        .collect()
}

fn present(path: &Path) -> Result<bool, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if super::super::failed_cleanup::plain(&meta) && meta.is_file() => Ok(true),
        Ok(_) => Err(StorageError::UnsafeFile.into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Io.into()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Hashed {
    sha256: [u8; 32],
    size: u64,
}
/// Release files by index name, as the review hashed them.
type Reviewed = BTreeMap<String, Hashed>;
fn reviewed_name(id: Uuid) -> String {
    format!("adoption-reviewed-{id}.json")
}
fn holds(path: &Path, expected: &Hashed) -> Result<bool, Error> {
    let entry = inventory::hash(path, &CancellationToken::new())?;
    Ok(entry.sha256 == expected.sha256 && entry.size == expected.size)
}

/// Which way the release files go, and what a file found in both places means.
#[derive(Clone, Copy)]
enum Way<'a> {
    /// Into `game`. The old launcher or a running game can recreate a file at
    /// its old path after the review, so a duplicate is settled by content.
    Adopt { plan: &'a Plan, store: &'a Path },
    /// Back out of `game`. The folder's copy is kept and `game`'s copy is left
    /// in `game`: nothing the player may want is deleted.
    Abandon,
}

/// A release file in both places while adopting. The copy the review verified
/// goes into `game`; the other stays outside it as a file the release does not
/// list, so the owned tree still matches the signed reference.
fn settle_duplicate(
    path: &str,
    source: &Path,
    target: &Path,
    plan: &Plan,
    store: &Path,
    reviewed: &mut Option<Reviewed>,
) -> Result<(), Error> {
    let owner = plan.installation.operation_id;
    let expected = match reviewed {
        Some(reviewed) => reviewed,
        None => reviewed.insert(read_large(
            &store.join(reviewed_name(plan.provenance.work_id)),
        )?),
    }
    .get(path)
    .copied()
    .ok_or(StorageError::Corrupt)?;
    if holds(target, &expected)? {
        tracing::warn!(target: "adoption", path, "in-place adoption: a release file reappeared at its old path; the verified copy is already in game, the other is left where it is");
        return Ok(());
    }
    if holds(source, &expected)? {
        let mut aside = source.as_os_str().to_owned();
        aside.push(format!(".adopt-conflict-{owner}"));
        let aside = PathBuf::from(aside);
        if present(&aside)? {
            tracing::error!(target: "adoption", path, "in-place adoption: a duplicate release file's conflict name is taken");
            return Err(Error::DuplicateReleaseFile);
        }
        std::fs::rename(target, &aside)?;
        std::fs::rename(source, target)?;
        tracing::warn!(target: "adoption", path, aside = %aside.display(), "in-place adoption: the copy in game did not verify; the reviewed copy replaced it and the other was kept beside the old path");
        return Ok(());
    }
    tracing::error!(target: "adoption", path, "in-place adoption: a release file is in game and at its old path, and neither copy is the one the review verified");
    Err(Error::DuplicateReleaseFile)
}

/// Move each file from `from` to `to`. Idempotent: a file already at `to` is
/// one an interrupted run moved. A file in neither place is refused; one in
/// both is settled as `way` says.
fn move_files(
    files: &[&str],
    from: &Path,
    to: &Path,
    way: Way<'_>,
    progress: &ProgressSink,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut reviewed = None;
    for (current, path) in files.iter().enumerate() {
        progress.report(crate::install::Progress::Extracting {
            label: "Adopting in place".into(),
            current,
            total: files.len(),
            filename: path.to_string(),
        });
        let source = inventory::native(from, path);
        let target = inventory::native(to, path);
        match (present(&source)?, present(&target)?) {
            (true, false) => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::rename(&source, &target)?;
                hook(Point::Moved)?;
            }
            (false, true) => (),
            (true, true) => match way {
                Way::Adopt { plan, store } => {
                    settle_duplicate(path, &source, &target, plan, store, &mut reviewed)?
                }
                Way::Abandon => {
                    tracing::warn!(target: "adoption", path, "abandoning in-place adoption: a release file is in both places; the folder's copy stays and game keeps the other");
                }
            },
            (false, false) => {
                tracing::error!(target: "adoption", path, "in-place adoption: a release file is in neither place");
                return Err(StorageError::Corrupt.into());
            }
        }
    }
    progress.report(crate::install::Progress::Extracting {
        label: "Adopting in place".into(),
        current: files.len(),
        total: files.len(),
        filename: String::new(),
    });
    remove_emptied(from, files)
}

fn move_release_files(
    store: &Path,
    plan: &Plan,
    progress: &ProgressSink,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let root = &plan.installation.destination;
    hook(Point::BeforePromotion)?;
    move_files(
        &release_files(&plan.report),
        root,
        &root.join("game"),
        Way::Adopt { plan, store },
        progress,
        hook,
    )?;
    hook(Point::AfterPromotion)
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

/// Remove the directories the moves emptied, deepest first. A directory that
/// still holds anything (the player's own files) stays.
fn remove_emptied(root: &Path, files: &[&str]) -> Result<(), Error> {
    let mut directories = BTreeSet::new();
    for file in files {
        let mut parts: Vec<&str> = file.split('/').collect();
        parts.pop();
        while !parts.is_empty() {
            directories.insert(parts.join("/"));
            parts.pop();
        }
    }
    let mut directories: Vec<String> = directories.into_iter().collect();
    directories.sort_by_key(|d| std::cmp::Reverse(d.matches('/').count()));
    for directory in directories {
        let path = inventory::native(root, &directory);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !super::super::failed_cleanup::plain(&meta) || !meta.is_dir() {
            continue;
        }
        if std::fs::read_dir(&path)?.next().is_none() {
            std::fs::remove_dir(&path)?;
        }
    }
    Ok(())
}

fn validate(game: &Path, plan: &Plan) -> Result<(), Error> {
    inventory::same_directory(game, &plan.stage_identity)?;
    if inventory::content_digest(&inventory::scan(game, &CancellationToken::new())?)?
        != plan.provenance.reference_digest
    {
        tracing::error!(target: "adoption", game = %game.display(), "in-place adoption: the owned tree does not match the signed reference");
        return Err(StorageError::Corrupt.into());
    }
    Ok(())
}

/// Shared by publication and explicit recovery. A Staged checkpoint finishes its
/// moves; the release files are then exactly the signed reference.
pub(super) fn finish(
    state: &mut DesktopState,
    plan: &Plan,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let id = plan.provenance.work_id;
    let root = &plan.installation.destination;
    let game = root.join("game");
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
        move_files(
            &release_files(&plan.report),
            root,
            &game,
            Way::Adopt {
                plan,
                store: &state.directory.root,
            },
            &ProgressSink::latest().0,
            &mut |_| Ok(()),
        )?;
        // Checkpointed before setup rewrites any release bytes: from here only
        // recovery is offered, because moving the files back would not undo
        // the setup and Abandon would not put the folder back as reviewed.
        record.phase = Phase::Promoted;
        write_large(&state.directory.root, &name(id), &record)?;
    }
    if record.phase == Phase::Promoted {
        // Idempotent, so a recovery after a crash here applies it again.
        crate::client_setup::prepare(&game, &plan.installation.login_servers)?;
        reference::ledger(&release)
            .save(&game)
            .map_err(|_| StorageError::Io)?;
        hook(Point::SetUp)?;
    }
    validate(&game, plan)?;
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
    tracing::info!(target: "adoption", directory = %root.display(), files = plan.report.files.len(), "adopted the installation in place");
    Ok(())
}

/// While only renames have happened, abandonment can put every file back.
pub(super) fn abandonable(state: &DesktopState, plan: &Plan) -> bool {
    read_large::<Record>(&state.directory.root.join(name(plan.provenance.work_id)))
        .is_ok_and(|record| record.plan == *plan && record.phase == Phase::Staged)
}

/// Move the release files back to where the review found them, then release the
/// folder. Published content is recovered, never unwound.
pub(super) fn abandon(state: &mut DesktopState, plan: &Plan) -> Result<(), Error> {
    let id = plan.provenance.work_id;
    if !abandonable(state, plan) {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.installation.destination;
    let game = root.join("game");
    inventory::same_directory(root, &plan.destination_identity)?;
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(OWNER))
        .map_err(|_| StorageError::Corrupt)?;
    marker.try_lock().map_err(|_| StorageError::InUse)?;
    let saved: InstallIntent = super::super::read_open(&marker)?;
    if saved != plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    match std::fs::symlink_metadata(&game) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io.into()),
        Ok(_) => {
            inventory::same_directory(&game, &plan.stage_identity)?;
            let files = release_files(&plan.report);
            move_files(
                &files,
                &game,
                root,
                Way::Abandon,
                &ProgressSink::latest().0,
                &mut |_| Ok(()),
            )?;
            // The synthesized ledger is the launcher's own file.
            match std::fs::remove_file(game.join("launcher-installed.json")) {
                Ok(()) => (),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(_) => return Err(StorageError::Io.into()),
            }
            if std::fs::remove_dir(&game).is_err() {
                tracing::warn!(target: "adoption", game = %game.display(), "abandoned in-place adoption left a non-empty game folder in place");
            }
        }
    }
    // Windows cannot unlink a locked file; admission is still serialized here.
    drop(marker);
    match std::fs::remove_file(root.join(OWNER)) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io.into()),
    }
    state
        .operations_mut()?
        .reconcile(id, OperationState::Cancelled)?;
    tracing::info!(target: "adoption", directory = %root.display(), "abandoned in-place adoption; release files are back in place");
    Ok(())
}
