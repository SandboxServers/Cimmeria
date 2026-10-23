//! Durable, plan-bound rollback progress for a shared content root.
use super::in_place::{Hashed, OWNER};
use super::in_place_replace::{original_home, replaced_kind};
use super::publication::{read_large, write_large};
use super::*;
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    version: u32,
    digest: [u8; 32],
    setup_done: bool,
    restored_setup: u32,
    restored: BTreeSet<String>,
    files_done: bool,
    cleanup: u8,
}
fn name(plan: &Plan) -> String {
    format!("adoption-abandon-{}.json", plan.provenance.work_id)
}
fn load(state: &DesktopState, plan: &Plan) -> Result<Option<Progress>, Error> {
    let path = state.directory.root.join(name(plan));
    if !path.exists() {
        return Ok(None);
    }
    let progress: Progress = read_large(&path)?;
    if progress.version != 1 || progress.digest != plan.digest()? || progress.cleanup > 4 {
        return Err(StorageError::Corrupt.into());
    }
    Ok(Some(progress))
}
pub(super) fn started(state: &DesktopState, plan: &Plan) -> bool {
    matches!(load(state, plan), Ok(Some(_)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Point {
    Setup,
    File,
    Manifest,
    Expected,
    Work,
    Identity,
    Receipt,
    Owner,
}

pub(super) fn abandon(state: &mut DesktopState, plan: &Plan, phase: Phase) -> Result<(), Error> {
    abandon_with(state, plan, phase, &mut |_| Ok(()))
}
pub(super) fn abandon_with(
    state: &mut DesktopState,
    plan: &Plan,
    phase: Phase,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    let root = &plan.installation.destination;
    inventory::same_directory(root, &plan.destination_identity)?;
    super::direct_paths::validate_plan(plan)?;
    let mut progress = load(state, plan)?;
    let marker_path = root.join(OWNER);
    let guard = super::direct_paths::Parents::new(plan, &[&marker_path], false)?;
    let marker = match guard.open(&marker_path, false, false) {
        Ok(marker) => {
            marker.try_lock().map_err(|_| StorageError::InUse)?;
            let saved: InstallIntent = super::super::read_open(&marker)?;
            if saved != plan.installation {
                return Err(StorageError::Corrupt.into());
            }
            Some(marker)
        }
        Err(_)
            if progress
                .as_ref()
                .is_some_and(|p| p.files_done && p.cleanup >= 3)
                && matches!(std::fs::symlink_metadata(&marker_path), Err(error) if error.kind() == std::io::ErrorKind::NotFound) =>
        {
            None
        }
        Err(error) => return Err(error),
    };
    let mut progress = progress.take().unwrap_or(Progress {
        version: 1,
        digest: plan.digest()?,
        setup_done: false,
        restored_setup: 0,
        restored: BTreeSet::new(),
        files_done: plan.schema_version == 1,
        cleanup: 0,
    });
    write_large(&state.directory.root, &name(plan), &progress)?;
    if !progress.files_done {
        if !progress.setup_done {
            if phase == Phase::Promoted {
                progress.restored_setup = super::in_place_check::restore_setup_guarded(plan)?;
            }
            progress.setup_done = true;
            write_large(&state.directory.root, &name(plan), &progress)?;
            hook(Point::Setup)?;
        }
        for replacement in plan.replacements.iter().rev() {
            if progress.restored.contains(&replacement.path) {
                continue;
            }
            restore_one(plan, replacement)?;
            progress.restored.insert(replacement.path.clone());
            write_large(&state.directory.root, &name(plan), &progress)?;
            hook(Point::File)?;
        }
        super::in_place_replace::restore_cooked_tier(super::in_place::Way::Abandon { plan }, root)?;
        progress.files_done = true;
        write_large(&state.directory.root, &name(plan), &progress)?;
    }
    if progress.cleanup == 0 {
        if plan.schema_version == 2 {
            cleanup_work(plan, hook)?;
        }
        hook(Point::Work)?;
        progress.cleanup = 1;
        write_large(&state.directory.root, &name(plan), &progress)?;
    }
    if progress.cleanup == 1 {
        state.forget_installed_content(&plan.installation)?;
        hook(Point::Identity)?;
        progress.cleanup = 2;
        write_large(&state.directory.root, &name(plan), &progress)?;
    }
    if progress.cleanup == 2 {
        super::direct_paths::remove(plan, &root.join("content-ready.json"))?;
        hook(Point::Receipt)?;
        progress.cleanup = 3;
        write_large(&state.directory.root, &name(plan), &progress)?;
    }
    drop(marker);
    if progress.cleanup == 3 {
        super::direct_paths::remove(plan, &marker_path)?;
        hook(Point::Owner)?;
        progress.cleanup = 4;
        write_large(&state.directory.root, &name(plan), &progress)?;
    }
    state
        .operations_mut()?
        .reconcile(plan.provenance.work_id, OperationState::Cancelled)?;
    tracing::info!(target: "adoption", restored_setup = progress.restored_setup, "abandoned adoption ownership; installation root retained");
    Ok(())
}
fn restore_one(plan: &Plan, replacement: &Replacement) -> Result<(), Error> {
    let Some(kind) = replaced_kind(&replacement.path, replacement.reason) else {
        return Ok(());
    };
    let root = &plan.installation.destination;
    let target = inventory::native(root, &replacement.path);
    let home = original_home(plan, &replacement.path, kind)?;
    let mut partial = target.as_os_str().to_owned();
    partial.push(".adopt-partial");
    let partial = PathBuf::from(partial);
    let mut paths = vec![target.as_path(), partial.as_path()];
    paths.extend(home.as_deref());
    let guard = super::direct_paths::Parents::new(plan, &paths, true)?;
    let live = &target;
    let parked = home.as_ref();
    let expected = Hashed {
        sha256: replacement.sha256,
        size: replacement.size,
    };
    let original = match parked {
        Some(home) if guard.present(home)? => Some(home),
        _ => None,
    };
    if guard.present(live)? {
        let already_restored = original
            .map(|home| {
                let hash = guard.hash(home)?;
                guard.holds(
                    live,
                    &Hashed {
                        sha256: hash.sha256,
                        size: hash.size,
                    },
                )
            })
            .transpose()?
            .unwrap_or(false);
        if already_restored {
            return Ok(());
        }
        if !guard.holds(live, &expected)? {
            if original.is_some() || kind == Classification::Missing {
                return Err(super::in_place::refused(
                    Error::SourceChanged,
                    "an abandon target changed",
                ));
            }
            // A replacement not started yet still has its reviewed original.
            return Ok(());
        }
        if original.is_none() && home.is_some() {
            return Err(StorageError::Corrupt.into());
        }
    }
    guard.remove(&partial, false)?;
    if let Some(home) = home.as_ref().filter(|_| original.is_some()) {
        // Keep the only original until the completion checkpoint is durable.
        guard.copy_atomic(home, &target)?;
    } else if guard.present(live)? {
        guard.remove(&target, false)?;
    }
    Ok(())
}
fn cleanup_work(
    plan: &Plan,
    hook: &mut impl FnMut(Point) -> Result<(), Error>,
) -> Result<(), Error> {
    super::direct_paths::remove(
        plan,
        &plan.stage().join(super::in_place_check::SETUP_MANIFEST),
    )?;
    hook(Point::Manifest)?;
    super::direct_paths::remove(
        plan,
        &plan.stage().join(super::in_place_check::EXPECTED_TREE),
    )?;
    hook(Point::Expected)?;
    for replacement in &plan.replacements {
        if let Some(kind) = replaced_kind(&replacement.path, replacement.reason) {
            if let Some(home) = original_home(plan, &replacement.path, kind)? {
                super::direct_paths::remove(plan, &home)?;
            }
        }
    }
    for path in super::in_place::release_files(&plan.report) {
        if super::in_place_replace::keeps_login(plan, path) {
            super::in_place_replace::drop_login_copy(
                path,
                &inventory::native(&plan.installation.destination, path),
                plan,
            )?;
        }
    }
    if plan.stage().exists() {
        super::in_place_replace::clear_abandoned(
            plan,
            &super::in_place::release_files(&plan.report),
        )?;
    }
    let stage = plan.stage();
    let guard = super::direct_paths::Parents::new(plan, &[&stage], false)?;
    guard.remove_tree(&stage)?;
    Ok(())
}
