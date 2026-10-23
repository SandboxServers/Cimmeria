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
pub(super) fn full_layout(folder: &Path) -> bool {
    let working = folder.join("Working");
    working.is_dir() && crate::install_layout::binaries_dir(folder).parent() == Some(&working)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: ProgressSink,
    backend: Backend,
    transport: Option<artifacts::Transport>,
    light: bool,
) -> Result<Preview, Error> {
    {
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
    }
    // Resolved and scanned with no state guard held.
    let (adopted, scanned) = if light {
        // Light hashes the whole folder, so the scan reports progress (LA-11).
        let scan = light::scan_reporting(&progress, &cancel);
        source::request_source_with(&state, &request.source, AdoptionMode::Light, &scan)?
    } else {
        source::request_source(&state, &request.source, AdoptionMode::InPlace)?
    };
    let (folder, before) = {
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        let folder = match &adopted {
            AdoptionSource::Legacy(imported) => {
                if let Some(blocker) = source::refuse_blocker(imported) {
                    return Err(refused(blocker, "imported configuration"));
                }
                let folder = &imported.source.game_directory;
                if request.destination != *folder {
                    return Err(rejection::reject(
                        StorageError::InvalidDirectory.into(),
                        Rejection::new("invalid_directory")
                            .expected("the folder is not the imported game folder"),
                    ));
                }
                if folder.starts_with(&owner.directory.root)
                    || owner.directory.root.starts_with(folder)
                {
                    return Err(rejection::reject(
                        StorageError::InvalidDirectory.into(),
                        Rejection::new("invalid_directory")
                            .expected("the folder overlaps the launcher state"),
                    ));
                }
                // Its lock is held for the whole adoption, and a locked file cannot be
                // read for verification.
                if imported.source.launcher_directory.starts_with(folder) {
                    return Err(rejection::reject(
                        StorageError::InvalidDirectory.into(),
                        Rejection::new("invalid_directory")
                            .expected("the old launcher lives inside the game folder"),
                    ));
                }
                folder.clone()
            }
            AdoptionSource::Folder(picked) => {
                if request.destination != picked.root {
                    return Err(rejection::reject(
                        StorageError::InvalidDirectory.into(),
                        Rejection::new("invalid_directory")
                            .expected("the folder is not the picked game folder"),
                    ));
                }
                picked.root.clone()
            }
        };
        (folder, owner.preferences.clone())
    };
    if matches!(adopted, AdoptionSource::Legacy(_)) {
        source::canonical_paths(&adopted)
            .map_err(|error| refused(error, "a folder is not canonical"))?;
    }
    let cleared = clear_unbegun_admission(&*state.lock().map_err(|_| StorageError::Io)?, &folder)?;
    // The scan above saw any launcher-owned names this cleanup just took back;
    // only then is the folder hashed again.
    let scanned = if cleared {
        survey::explained(&folder, inventory::scan(&folder, &cancel))?
    } else {
        scanned
    };
    // The digest pins the scan the player reviews, so a rescan refreshes it.
    let adopted = match adopted {
        AdoptionSource::Folder(picked) if cleared => AdoptionSource::Folder(FolderSource {
            scan_digest: digest(&scanned)?,
            ..picked
        }),
        other => other,
    };
    if !full_layout(&folder) {
        return Err(rejection::reject(
            Error::UnsupportedLayout,
            Rejection::new("unsupported_layout")
                .expected("the installer's Working\\Binaries layout")
                .found("another layout"),
        ));
    }
    let (adopted, legacy_lock) = source::resolve(adopted)?;
    let servers = source::servers_of(&adopted, &before);
    let identity = match &backend {
        Backend::Native => ExtractionBackend::Native,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => helper.backend(),
    };
    let mut preparation =
        preparation::Ownership::claim(state.clone(), &request, identity, servers.clone())?;
    let prepared = if light {
        light::light_comparison(
            &state,
            &request,
            transport,
            &preparation,
            &folder,
            &scanned,
            &servers,
            &cancel,
            progress,
        )
        .map(|(compared, light)| (compared, Some(light)))
    } else {
        verification::reference_and_comparison(
            &state,
            &request,
            backend,
            transport,
            &mut preparation,
            source::borrowed(&adopted),
            AdoptionMode::InPlace,
            &scanned,
            &servers,
            &cancel,
            progress,
        )
        .map(|compared| (compared, None))
    };
    let (
        verification::Compared {
            reference,
            expected_digest,
            expected_paths,
            expected_tree,
            files,
            download,
            replacements,
            login_replaced,
            file_index_sha256,
            index_fallback,
        },
        light,
    ) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            preparation.release();
            let check = Rejection::new("comparison_failed").found(rejection::code(&error));
            return Err(rejection::reject(error, check));
        }
    };
    let (imported_identity, requested_config) = match &adopted {
        AdoptionSource::Legacy(imported) => (
            Some(imported.identity.clone()),
            Some(imported.config.clone()),
        ),
        AdoptionSource::Folder(_) => (None, None),
    };
    // The backup holds each original a replacement overwrites (D-AI1, D-AI13). It is
    // named by the preview handle, which admission uses too.
    let backup = (files
        .iter()
        .any(|d| d.classification == Classification::Modified)
        || login_replaced)
        .then(|| {
            folder.join(format!(
                "cimmeria-adoption-backup-{}",
                preparation.record.id
            ))
        });
    let report = Report {
        preview_handle: preparation.record.id,
        source: folder.clone(),
        destination: folder,
        release_digest: request.release.digest(),
        imported_identity,
        requested_config,
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
        index_fallback,
        download: Some(download),
        backup,
        login_replaced,
    };
    for file in report.blocking(&replacements) {
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
        expected_digest,
        expected_paths,
        expected_tree,
        replacements,
        file_index_sha256,
        light,
        _legacy_lock: legacy_lock,
    })
}

/// Launcher-owned names already in `folder`. The only ones taken back are what
/// an admission leaves when it dies before its operation begins, or when
/// Abandon could not remove `game`: an owner marker that no journal operation or
/// saved plan names, and an empty `game`. Anything else is refused.
fn clear_unbegun_admission(state: &DesktopState, folder: &Path) -> Result<bool, Error> {
    let marker = folder.join(OWNER);
    let game = folder.join("game");
    let has = |path: &Path| std::fs::symlink_metadata(path).is_ok();
    let stages = work_folders(folder)?;
    if !MANAGED.iter().any(|entry| has(&folder.join(entry))) && stages.is_empty() {
        return Ok(false);
    }
    let leftover = |entry: &str, found: &str| {
        Err(rejection::reject(
            Error::LeftoverLauncherFiles,
            Rejection::new("leftover_launcher_files")
                .entry(entry)
                .found(found),
        ))
    };
    if has(&folder.join("content-ready.json")) {
        return leftover("content-ready.json", "content_ready");
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
        return leftover("game", "game_not_empty");
    }
    if has(&marker) {
        let Ok(intent) = inventory::open(&marker)
            .and_then(|file| super::super::read_open::<InstallIntent>(&file).map_err(Error::from))
        else {
            return leftover(OWNER, "marker");
        };
        let journaled = state
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| op.id == intent.operation_id);
        if journaled || intent.destination != folder || planned(state, &intent)? {
            return leftover(OWNER, "marker");
        }
    }
    for (id, stage) in &stages {
        if !unbegun_work_folder(state, *id, stage)? {
            return leftover(&rejection::relative(folder, stage), "work_folder");
        }
    }
    if has(&marker) {
        std::fs::remove_file(&marker)?;
    }
    if empty_game {
        std::fs::remove_dir(&game)?;
    }
    for (_, stage) in &stages {
        std::fs::remove_dir_all(stage)?;
    }
    tracing::warn!(target: "adoption", folder = %folder.display(), "in-place adoption took back an owner marker, empty game folder or work folder left by an admission that never began");
    Ok(true)
}

/// The `.cimmeria-adopt-<work id>` folders in `folder`, by work id.
fn work_folders(folder: &Path) -> Result<Vec<(Uuid, PathBuf)>, Error> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name();
        let id = name
            .to_string_lossy()
            .strip_prefix(".cimmeria-adopt-")
            .and_then(|id| Uuid::parse_str(id).ok());
        if let Some(id) = id {
            found.push((id, entry.path()));
        }
    }
    Ok(found)
}

/// Whether a work folder holds only what admission copies in: `replacements` (any
/// content), the stored `expected-tree.json` of a light admission, and optionally an
/// empty `originals`, and nothing else at its top level. Anything else is not the
/// launcher's to delete, so the folder is kept.
fn only_copies(stage: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(stage) else {
        return false;
    };
    let mut replacements = false;
    let mut stored = false;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let path = entry.path();
        let plain_dir = std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir());
        match entry.file_name().to_str() {
            Some("replacements") if plain_dir => replacements = true,
            Some(super::in_place_check::EXPECTED_TREE)
                if std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file()) =>
            {
                stored = true
            }
            Some("originals") if plain_dir => {
                if std::fs::read_dir(&path).is_ok_and(|mut d| d.next().is_some()) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    replacements || stored
}

/// Whether an admission for `id` never began. Moves follow `begin`, so such a
/// work folder holds only copies. It is not begun when no journal operation names
/// it, nothing was set aside, no backup was taken, and its record is absent or
/// still Staged. A record that cannot be read counts as begun.
fn unbegun_work_folder(state: &DesktopState, id: Uuid, stage: &Path) -> Result<bool, Error> {
    if !only_copies(stage) {
        return Ok(false);
    }
    let journaled = state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| op.id == id);
    let parked = std::fs::read_dir(stage.join("originals")).is_ok_and(|mut d| d.next().is_some());
    if journaled || parked {
        return Ok(false);
    }
    let record = state.directory.root.join(super::publication::name(id));
    if std::fs::symlink_metadata(&record).is_err() {
        return Ok(true);
    }
    let Ok(record) = read_large::<Record>(&record) else {
        return Ok(false);
    };
    let backed_up = record
        .plan
        .report
        .backup
        .as_ref()
        .is_some_and(|backup| std::fs::symlink_metadata(backup).is_ok());
    Ok(record.phase == Phase::Staged && !backed_up)
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
