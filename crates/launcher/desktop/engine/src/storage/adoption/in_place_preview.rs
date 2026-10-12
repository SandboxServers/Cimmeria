//! Verification for in-place adoption (D-LX12): the imported folder is compared
//! with a private signed reference before anything in it is touched.
use super::in_place::{refused, MANAGED, OWNER};
use super::publication::read_large;
use super::*;
use crate::install_progress::ProgressSink;

/// The layout the 2009 installer and a fresh launcher install write:
/// `SGW.exe` in `Working\Binaries`. The old launcher also manages folders that
/// point straight at `Working` or `Binaries`; every release file there sits at
/// another path than the release keeps it, so in place they cannot verify.
fn full_layout(folder: &Path) -> bool {
    let working = folder.join("Working");
    working.is_dir() && crate::install_layout::binaries_dir(folder).parent() == Some(&working)
}

/// What the old launcher's ledger says is still missing from the signed release.
/// Checked before anything is downloaded: such a folder cannot verify.
fn outdated(ledger: &migration::LegacyLedger, release: &VerifiedRelease) -> Vec<String> {
    let manifest = release.manifest();
    let mut missing = Vec::new();
    if !ledger
        .seed_sha256
        .as_deref()
        .is_some_and(|seed| seed.eq_ignore_ascii_case(&manifest.seed.sha256))
    {
        missing.push("seed".to_string());
    }
    for patch in &manifest.patches {
        if !ledger.applied_patches.contains(&patch.state_key()) {
            missing.push(patch.id.clone());
        }
    }
    missing
}

pub(super) fn preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: ProgressSink,
    transport: Option<artifacts::Transport>,
) -> Result<Preview, Error> {
    let (imported, before) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        idle(
            &owner,
            request.operation_revision,
            request.preferences_revision,
        )?;
        if owner.compatibility.for_release(&request.release).blocks() {
            return Err(Error::LauncherTooOld);
        }
        if owner.installed_content()?.is_some() {
            return Err(ContractError::IdentityConflict.into());
        }
        let imported = source::saved_import(&owner, &request.source)?;
        if let Some(blocker) = source::import_blocker(&imported) {
            return Err(refused(blocker, "imported configuration"));
        }
        let folder = &imported.source.game_directory;
        if request.destination != *folder {
            return Err(refused(
                StorageError::InvalidDirectory.into(),
                "the folder is not the imported game folder",
            ));
        }
        if folder.starts_with(&owner.directory.root) || owner.directory.root.starts_with(folder) {
            return Err(refused(
                StorageError::InvalidDirectory.into(),
                "the folder overlaps the launcher state",
            ));
        }
        // Its lock is held for the whole adoption, and a locked file cannot be
        // read for verification.
        if imported.source.launcher_directory.starts_with(folder) {
            return Err(refused(
                StorageError::InvalidDirectory.into(),
                "the old launcher lives inside the game folder",
            ));
        }
        (imported, owner.preferences.clone())
    };
    let folder = imported.source.game_directory.clone();
    source::canonical_paths(&imported)
        .map_err(|error| refused(error, "a folder is not canonical"))?;
    clear_unbegun_admission(&*state.lock().map_err(|_| StorageError::Io)?, &folder)?;
    if !full_layout(&folder) {
        tracing::warn!(target: "adoption", folder = %folder.display(), binaries = %crate::install_layout::binaries_dir(&folder).display(), "in-place adoption refused: the folder is not in the installer's Working\\Binaries layout");
        return Err(Error::UnsupportedLayout);
    }
    let missing = outdated(&imported.ledger, &request.release);
    if !missing.is_empty() {
        tracing::warn!(target: "adoption", ?missing, "in-place adoption refused: the old launcher has not applied the signed release");
        return Err(Error::OutdatedInstall);
    }
    let (adopted, legacy_lock) = source::resolve(imported)?;
    let imported = source::legacy(&adopted)?;
    let scanned = inventory::scan(&folder, &cancel)?;
    let servers = source::servers(imported);
    let mut preparation = preparation::Ownership::claim(
        state.clone(),
        &request,
        ExtractionBackend::Native,
        servers.clone(),
    )?;
    let prepared = verification::reference_and_comparison(
        &state,
        &request,
        Backend::Native,
        transport,
        &mut preparation,
        source::borrowed(&adopted),
        &scanned,
        &servers,
        &cancel,
        progress,
    );
    let (reference, files) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            preparation.release();
            return Err(refused(error, "the signed reference could not be compared"));
        }
    };
    let report = Report {
        preview_handle: preparation.record.id,
        source: folder.clone(),
        destination: folder,
        release_digest: request.release.digest(),
        imported_identity: Some(imported.identity.clone()),
        requested_config: Some(imported.config.clone()),
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
        download: None,
        backup: None,
    };
    for file in report.blocking() {
        tracing::warn!(target: "adoption", path = %file.path, classification = ?file.classification, "in-place adoption: release file does not verify");
    }
    Ok(Preview {
        state,
        report,
        source: adopted,
        before,
        preparation,
        release: request.release,
        scanned,
        reference,
        _legacy_lock: legacy_lock,
    })
}

/// Launcher-owned names already in `folder`. The only ones taken back are what
/// an admission leaves when it dies before its operation begins, or when
/// Abandon could not remove `game`: an owner marker that no journal operation or
/// saved plan names, and an empty `game`. Anything else is refused.
fn clear_unbegun_admission(state: &DesktopState, folder: &Path) -> Result<(), Error> {
    let marker = folder.join(OWNER);
    let game = folder.join("game");
    let has = |path: &Path| std::fs::symlink_metadata(path).is_ok();
    if !MANAGED.iter().any(|entry| has(&folder.join(entry))) {
        return Ok(());
    }
    let leftover = |why: &str| {
        tracing::warn!(target: "adoption", folder = %folder.display(), "in-place adoption refused: launcher-owned names are already in the folder ({why})");
        Err(Error::LeftoverLauncherFiles)
    };
    if has(&folder.join("content-ready.json")) {
        return leftover("content-ready.json");
    }
    let empty_game = match std::fs::symlink_metadata(&game) {
        Err(_) => false,
        Ok(meta) => {
            super::super::failed_cleanup::plain(&meta)
                && meta.is_dir()
                && std::fs::read_dir(&game)?.next().is_none()
        }
    };
    if has(&game) && !empty_game {
        return leftover("game is not an empty folder");
    }
    if has(&marker) {
        let Ok(intent) = inventory::open(&marker)
            .and_then(|file| super::super::read_open::<InstallIntent>(&file).map_err(Error::from))
        else {
            return leftover("the owner marker is not readable");
        };
        let journaled = state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == intent.operation_id);
        if journaled || intent.destination != folder || planned(state, &intent)? {
            return leftover("the owner marker belongs to known work");
        }
    }
    if has(&marker) {
        std::fs::remove_file(&marker)?;
    }
    if empty_game {
        std::fs::remove_dir(&game)?;
    }
    tracing::warn!(target: "adoption", folder = %folder.display(), "in-place adoption took back an owner marker or empty game folder left by an admission that never began");
    Ok(())
}
/// Whether a saved adoption plan admits this owner and the journal began it.
/// A plan whose operation never began moved nothing: moves follow `begin`.
fn planned(state: &DesktopState, intent: &InstallIntent) -> Result<bool, Error> {
    let current = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .map(|op| op.id);
    for entry in std::fs::read_dir(&state.directory.root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("adoption-plan-") && name.ends_with(".json")) {
            continue;
        }
        match read_large::<Plan>(&entry.path()) {
            Ok(plan) if plan.installation == *intent => {
                if current == Some(plan.provenance.work_id) {
                    return Ok(true);
                }
            }
            Ok(_) => (),
            // An unreadable plan might be this one: never guess.
            Err(_) => return Ok(true),
        }
    }
    Ok(false)
}
