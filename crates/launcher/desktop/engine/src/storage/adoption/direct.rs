//! Schema-2 adoption applies reviewed content at the selected root, without
//! relocating the installation. Schema-1 journals are never resumed as movers.
use super::in_place::{present, Way, OWNER};
use super::in_place_replace::{self, keeps_login, replaced_kind};
use super::publication::Point;
use super::*;
use crate::install_progress::{ProgressReporter, ProgressSink};

pub(super) fn apply(
    plan: &Plan,
    progress: &ProgressSink,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    crate::phase_timing::Context::for_operation(plan.provenance.work_id)
        .measure(crate::phase_timing::Phase::AdoptionApply, || {
            apply_timed(plan, progress, hook)
        })
}

fn apply_timed(
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

pub(super) fn abandon(state: &mut DesktopState, plan: &Plan, phase: Phase) -> Result<(), Error> {
    super::direct_abandon::abandon(state, plan, phase)
}
