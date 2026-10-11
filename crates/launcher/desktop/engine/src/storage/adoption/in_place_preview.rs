//! Verification for in-place adoption (D-LX12): the imported folder is compared
//! with a private signed reference before anything in it is touched.
use super::in_place::{refused, MANAGED};
use super::*;
use crate::install_progress::ProgressSink;

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
        let imported = owner
            .legacy_import()
            .map_err(|_| StorageError::Corrupt)?
            .ok_or(StorageError::Corrupt)?;
        if imported.confirmation != request.import_digest {
            return Err(Error::SourceChanged);
        }
        if let Some(blocker) = import_blocker(&imported) {
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
    for path in [&folder, &imported.source.launcher_directory] {
        if path.canonicalize()? != *path {
            return Err(refused(
                StorageError::UnsafeFile.into(),
                "a folder is not canonical",
            ));
        }
    }
    for entry in MANAGED {
        if std::fs::symlink_metadata(folder.join(entry)).is_ok() {
            tracing::warn!(target: "adoption", entry, "in-place adoption refused: the folder already has a launcher-owned name");
            return Err(StorageError::InvalidDirectory.into());
        }
    }
    let missing = outdated(&imported.ledger, &request.release);
    if !missing.is_empty() {
        tracing::warn!(target: "adoption", ?missing, "in-place adoption refused: the old launcher has not applied the signed release");
        return Err(Error::OutdatedInstall);
    }
    let legacy_lock = lock_source(&imported)?;
    verify_import(&imported)?;
    let source = inventory::scan(&folder, &cancel)?;
    let servers = servers(&imported);
    let mut preparation = preparation::Ownership::claim(
        state.clone(),
        &request,
        ExtractionBackend::Native,
        servers.clone(),
    )?;
    let prepared = reference_and_comparison(
        &state,
        &request,
        Backend::Native,
        transport,
        &mut preparation,
        &imported,
        &source,
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
        imported_identity: imported.identity.clone(),
        requested_config: imported.config.clone(),
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
    };
    for file in report.blocking() {
        tracing::warn!(target: "adoption", path = %file.path, classification = ?file.classification, "in-place adoption: release file does not verify");
    }
    Ok(Preview {
        state,
        report,
        imported,
        before,
        preparation,
        release: request.release,
        source,
        reference,
        _legacy_lock: legacy_lock,
    })
}
