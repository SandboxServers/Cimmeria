//! Retained replacement with a durable checkpoint before either rename.
//! Backup cleanup and restart reconciliation are separate, not implicit retries.
use super::*;
use crate::OperationState;
use preparation::{Failure, Prepared};
use std::{
    io::{Seek, SeekFrom},
    panic::AssertUnwindSafe,
    sync::Mutex,
};
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Planned,
    OriginalMoved,
    Promoted,
    Published,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub(super) schema_version: u32,
    pub(super) plan: Plan,
    pub(super) phase: Phase,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    OriginalMarked,
    ReplacementMarked,
    Planned,
    BeforeOriginalRename,
    AfterOriginalRename,
    OriginalMoved,
    BeforePromotion,
    AfterPromotion,
    Promoted,
    Published,
}

/// Takes ownership of the staged reconstruction. Closing the observer does not
/// abort replacement. Success retains the old backup for future explicit cleanup.
/// This internal native-Windows milestone is not yet exposed through IPC.
pub fn commit_native(
    state: Arc<Mutex<DesktopState>>,
    prepared: Prepared,
) -> Result<oneshot::Receiver<Result<(), Failure>>, IntentError> {
    if !cfg!(windows) || !prepared.plan.installation.backend.is_native() {
        return Err(ContractError::InvalidTransition.into());
    }
    start(state, prepared)
}
fn start(
    state: Arc<Mutex<DesktopState>>,
    prepared: Prepared,
) -> Result<oneshot::Receiver<Result<(), Failure>>, IntentError> {
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (send, observed) = oneshot::channel();
    runtime.spawn_blocking(move || {
        let id = prepared.plan.id;
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| replace(&state, &prepared, |_| Ok(()))))
                .unwrap_or(Err(Failure::ReconciliationRequired));
        let result = result.map_err(|error| preparation::finish_failure(&state, id, error));
        // Prepared's observer is harmless once a durable terminal result exists.
        drop(prepared);
        let _ = send.send(result);
    });
    Ok(observed)
}
pub(super) fn replace(
    state: &Mutex<DesktopState>,
    prepared: &Prepared,
    mut hook: impl FnMut(Point) -> Result<(), StorageError>,
) -> Result<(), Failure> {
    let mut state = state.lock().map_err(|_| Failure::ReconciliationRequired)?;
    // Serializes cancellation with entry into commit. After this point a queued
    // cancel cannot abandon the two-rename sequence.
    let plan = &prepared.plan;
    let checked = || -> Result<(), IntentError> {
        // Reuse the locked handles on Windows; both have already been read or
        // written during preparation and therefore need an explicit rewind.
        (&prepared._root_owner)
            .seek(SeekFrom::Start(0))
            .map_err(|_| StorageError::Io)?;
        (&prepared._work_owner)
            .seek(SeekFrom::Start(0))
            .map_err(|_| StorageError::Io)?;
        if state.repair_plan()?.as_ref() != Some(plan)
            || read_open::<InstallIntent>(&prepared._root_owner)? != plan.installation
            || read_open::<Plan>(&prepared._work_owner)? != *plan
        {
            return Err(StorageError::Corrupt.into());
        }
        let staged: Plan = read(
            &state
                .directory
                .root
                .join(format!("repair-prepared-{}.json", plan.id)),
        )?
        .ok_or(StorageError::Corrupt)?;
        if staged != *plan {
            return Err(StorageError::Corrupt.into());
        }
        Ok(())
    };
    checked().map_err(|_| Failure::ReconciliationRequired)?;
    let operation = state
        .operations()
        .snapshot()
        .operation
        .as_ref()
        .ok_or(Failure::ReconciliationRequired)?;
    match operation.state {
        OperationState::CancelRequested => return Err(Failure::Cancelled),
        OperationState::Running => (),
        _ => return Err(Failure::ReconciliationRequired),
    }
    let result = (|| -> Result<(), IntentError> {
        let root = &plan.installation.destination;
        let game = root.join("game");
        if root.canonicalize().map_err(|_| StorageError::Io)? != *root
            || directory_or_absent(&game)? != plan.original_present
            || directory_or_absent(&plan.backup())?
            || !directory_or_absent(&plan.work_directory())?
            || !directory_or_absent(&plan.stage())?
        {
            return Err(StorageError::UnsafeFile.into());
        }
        // No links, reparse points or special files in either tree, including
        // entries outside the minimal content_valid layout checks.
        failed_cleanup::validate_tree(&plan.work_directory())?;
        if plan.original_present {
            failed_cleanup::validate_tree(&game)?;
        }
        let release = state
            .release_for_intent(&plan.installation)
            .map_err(|_| StorageError::Corrupt)?;
        if !install_worker::content_valid(&plan.stage(), &release) {
            return Err(StorageError::Corrupt.into());
        }
        let checkpoint = format!("repair-commit-{}.json", plan.id);
        if read::<Record>(&state.directory.root.join(&checkpoint))?.is_some() {
            return Err(StorageError::Corrupt.into()); // explicit recovery only
        }
        // Markers move with the trees and distinguish interrupted renames from
        // unrelated replacements during explicit restart recovery.
        tree_identity::absent(&plan.stage(), plan)?;
        if plan.original_present {
            tree_identity::absent(&game, plan)?;
            tree_identity::write(&game, plan, tree_identity::Role::Original)?;
            hook(Point::OriginalMarked)?;
        }
        tree_identity::write(&plan.stage(), plan, tree_identity::Role::Replacement)?;
        hook(Point::ReplacementMarked)?;
        record(&mut state, plan, Phase::Planned)?;
        hook(Point::Planned)?;
        if plan.original_present {
            hook(Point::BeforeOriginalRename)?;
            std::fs::rename(&game, plan.backup()).map_err(|_| StorageError::Io)?;
            hook(Point::AfterOriginalRename)?;
            sync(root)?;
        }
        record(&mut state, plan, Phase::OriginalMoved)?;
        hook(Point::OriginalMoved)?;
        hook(Point::BeforePromotion)?;
        std::fs::rename(plan.stage(), &game).map_err(|_| StorageError::Io)?;
        hook(Point::AfterPromotion)?;
        sync(&plan.work_directory())?;
        sync(root)?;
        record(&mut state, plan, Phase::Promoted)?;
        hook(Point::Promoted)?;
        if !install_worker::content_valid(&game, &release) {
            return Err(StorageError::Corrupt.into());
        }
        atomic::write(root, "content-ready.json", &plan.installation)?;
        record(&mut state, plan, Phase::Published)?;
        hook(Point::Published)?;
        state
            .operations_mut()?
            .observe(plan.id, OperationState::Succeeded)?;
        Ok(())
    })();
    result.map_err(|_| Failure::ReconciliationRequired)
}
pub(super) fn record(
    state: &mut DesktopState,
    plan: &Plan,
    phase: Phase,
) -> Result<(), StorageError> {
    let result = atomic::write(
        &state.directory.root,
        &format!("repair-commit-{}.json", plan.id),
        &Record {
            schema_version: 2,
            plan: plan.clone(),
            phase,
        },
    );
    state.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
    result
}
pub(super) fn sync(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| StorageError::PersistenceUncertain)?;
    #[cfg(not(unix))]
    let _ = path; // Native Windows power-loss validation remains a release gate.
    Ok(())
}
#[cfg(test)]
pub(super) mod tests;
