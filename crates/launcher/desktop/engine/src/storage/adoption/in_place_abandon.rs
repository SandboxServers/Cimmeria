//! Abandoning an in-place adoption that has only renamed files so far.
use super::in_place::{move_files, release_files, Way, OWNER};
use super::in_place_check;
use super::in_place_replace::clear_abandoned;
use super::publication::{name, read_large};
use super::*;
use crate::install_progress::ProgressSink;

fn phase(state: &DesktopState, plan: &Plan) -> Option<Phase> {
    let record =
        read_large::<Record>(&state.directory.root.join(name(plan.provenance.work_id))).ok()?;
    (record.plan == *plan).then_some(record.phase)
}

/// While only renames have happened, abandonment can put every file back. A light
/// adoption can also be abandoned after setup started, because it copied the setup
/// files aside before the checkpoint (D-LF2); one without that copy is recover-only.
pub(super) fn abandonable(state: &DesktopState, plan: &Plan) -> bool {
    match phase(state, plan) {
        Some(Phase::Staged) => true,
        Some(Phase::Promoted) => {
            matches!(plan.provenance.check, FileCheck::Light)
                && in_place_check::has_setup_originals(&plan.stage())
        }
        _ => false,
    }
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
    let mut restored_setup = 0;
    match std::fs::symlink_metadata(&game) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io.into()),
        Ok(_) => {
            inventory::same_directory(&game, &plan.stage_identity)?;
            // Setup rewrote some of these bytes: put them back before the release files
            // go home, so the folder is what the review saw.
            if phase(state, plan) == Some(Phase::Promoted) {
                restored_setup = in_place_check::restore_setup_originals(&plan.stage(), &game)?;
            }
            let files = release_files(&plan.report);
            move_files(
                &files,
                &game,
                root,
                Way::Abandon { plan },
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
            // Only after the files are home: until then the copies are what makes a
            // rerun of this abandon possible.
            in_place_check::remove_setup_originals(&plan.stage());
            clear_abandoned(plan, &files)?;
        }
    }
    // Written after the setup files are restored, so it exists only on a Promoted record.
    match std::fs::remove_file(root.join("content-ready.json")) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io.into()),
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
    tracing::info!(target: "adoption", directory = %root.display(), restored_setup, "abandoned in-place adoption; release files are back in place");
    Ok(())
}
