//! Schema-2 adoption applies reviewed content at the selected root, without
//! relocating the installation. Schema-1 journals are never resumed as movers.
use super::in_place::{holds, present, Hashed, Way, OWNER};
use super::in_place_replace::{self, keeps_login, replaced_kind};
use super::publication::Point;
use super::*;
use crate::install_progress::{ProgressReporter, ProgressSink};

pub(super) fn apply(
    plan: &Plan,
    progress: &ProgressSink,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    if plan.schema_version != 2 || plan.installation.schema_version != 2 {
        return Err(ContractError::InvalidTransition.into());
    }
    let root = &plan.installation.destination;
    inventory::same_directory(root, &plan.destination_identity)?;
    hook(Point::BeforePromotion)?;
    in_place_replace::place_cooked_tier(Way::Adopt { plan }, root, root)?;
    let files = super::in_place::release_files(&plan.report);
    for (current, path) in files.iter().enumerate() {
        progress.report(crate::install::Progress::Extracting {
            label: "Adopting in place".into(),
            current,
            total: files.len(),
            filename: path.to_string(),
        });
        let difference = plan
            .report
            .files
            .iter()
            .find(|d| d.path == *path)
            .ok_or(StorageError::Corrupt)?;
        let original = inventory::native(root, difference.source_path.as_deref().unwrap_or(path));
        let canonical = inventory::native(root, path);
        // Keep existing case spellings; intentional cooked-tier conversions have
        // already run above and use the reviewed target spelling.
        let target = if original.exists()
            && difference
                .source_path
                .as_deref()
                .is_some_and(|source| source.eq_ignore_ascii_case(path))
        {
            original.clone()
        } else {
            canonical
        };
        // A placed alias can be added beside a stock tier. Replace only the
        // target; never park or delete its separate source file.
        let source = target.clone();
        if let Some(replacement) = plan.replacements.iter().find(|r| r.path == *path) {
            if let Some(kind) = replaced_kind(path, replacement.reason) {
                in_place_replace::replace_file(
                    path,
                    &source,
                    &target,
                    plan,
                    replacement,
                    kind,
                    hook,
                )?;
                hook(Point::Moved)?;
                continue;
            }
        }
        if !present(&target)? {
            return Err(StorageError::Corrupt.into());
        }
        if keeps_login(plan, path) {
            in_place_replace::keep_login(path, &target, plan)?;
        }
        // No rename or copy for a file that is already at its reviewed path.
        hook(Point::Moved)?;
    }
    hook(Point::AfterPromotion)
}

pub(super) fn resume_files(_state: &DesktopState, plan: &Plan, _game: &Path) -> Result<(), Error> {
    apply(plan, &ProgressSink::latest().0, &mut |_| Ok(()))
}

/// Exclude only this authenticated operation's own metadata/work and backup.
/// Original player extras remain part of the light comparison.
pub(super) fn scan(root: &Path, plan: &Plan) -> Result<inventory::Index, Error> {
    let index =
        super::inventory_walk::scan(root, &CancellationToken::new(), &mut |_| {}, &|path| {
            path == root.join(OWNER)
                || path == root.join("content-ready.json")
                || path == plan.stage()
                || plan.report.backup.as_deref() == Some(path)
        })?;
    if matches!(plan.provenance.check, FileCheck::Light) {
        return Ok(index);
    }
    let mut result = inventory::Index::new();
    for difference in &plan.report.files {
        if difference.classification == Classification::Extra {
            continue;
        }
        let (_, entry) = index
            .iter()
            .find(|(path, _)| path.eq_ignore_ascii_case(&difference.path))
            .ok_or(StorageError::Corrupt)?;
        result.insert(difference.path.clone(), entry.clone());
    }
    // The full signed reference includes the synthesized ledger even though
    // the source ledger is classified as an extra during the review.
    let ledger = index
        .get("launcher-installed.json")
        .ok_or(StorageError::Corrupt)?;
    result.insert("launcher-installed.json".into(), ledger.clone());
    Ok(result)
}

/// Roll back reviewed per-file edits without ever moving the installation tree.
pub(super) fn restore(plan: &Plan) -> Result<(), Error> {
    let root = &plan.installation.destination;
    for replacement in plan.replacements.iter().rev() {
        let Some(kind) = replaced_kind(&replacement.path, replacement.reason) else {
            continue;
        };
        let target = inventory::native(root, &replacement.path);
        let home = in_place_replace::original_home(plan, &replacement.path, kind)?;
        let expected = Hashed {
            sha256: replacement.sha256,
            size: replacement.size,
        };
        let has_original = home.as_ref().is_some_and(|path| path.exists());
        let had_target = plan
            .report
            .files
            .iter()
            .find(|d| d.path == replacement.path)
            .and_then(|d| d.source_path.as_deref())
            .is_some_and(|source| {
                source.eq_ignore_ascii_case(&replacement.path)
                    || in_place_replace::cooked_tier(plan).is_some()
            });
        let mut partial = target.as_os_str().to_os_string();
        partial.push(".adopt-partial");
        let partial = PathBuf::from(partial);
        if present(&partial)? {
            std::fs::remove_file(&partial)?;
        }
        if present(&target)? && holds(&target, &expected)? {
            if home.is_some() && !has_original && had_target {
                return Err(StorageError::Corrupt.into());
            }
            std::fs::remove_file(&target)?;
        } else if present(&target)? {
            // An unrelated edit after interruption is never overwritten.
            if has_original {
                return Err(Error::SourceChanged);
            }
            continue;
        }
        if let Some(home) = home.filter(|home| home.exists()) {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(home, target)?;
        }
    }
    in_place_replace::restore_cooked_tier(Way::Abandon { plan }, root)
}

pub(super) fn abandon(state: &mut DesktopState, plan: &Plan, phase: Phase) -> Result<(), Error> {
    let root = &plan.installation.destination;
    inventory::same_directory(root, &plan.destination_identity)?;
    let marker = OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join(OWNER))?;
    marker.try_lock().map_err(|_| StorageError::InUse)?;
    let saved: InstallIntent = super::super::read_open(&marker)?;
    if saved != plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    let mut restored_setup = 0;
    if plan.schema_version == 2 {
        if phase == Phase::Promoted {
            restored_setup = super::in_place_check::restore_setup_originals(&plan.stage(), root)?;
        }
        restore(plan)?;
        for path in super::in_place::release_files(&plan.report) {
            if keeps_login(plan, path) {
                in_place_replace::drop_login_copy(path, &inventory::native(root, path), plan)?;
            }
        }
        super::in_place_replace::clear_abandoned(
            plan,
            &super::in_place::release_files(&plan.report),
        )?;
        super::in_place_check::remove_setup_originals(&plan.stage());
        let _ = std::fs::remove_dir(plan.stage());
    }
    // Schema-1 recovery releases only ownership. Its partially moved content,
    // staged originals and backups remain available for manual inspection.
    state.forget_installed_content(&plan.installation)?;
    match std::fs::remove_file(root.join("content-ready.json")) {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    drop(marker);
    std::fs::remove_file(root.join(OWNER))?;
    state
        .operations_mut()?
        .reconcile(plan.provenance.work_id, OperationState::Cancelled)?;
    tracing::info!(target: "adoption", restored_setup, "abandoned adoption ownership; installation root retained");
    Ok(())
}
