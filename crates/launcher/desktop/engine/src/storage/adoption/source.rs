//! Where an adoption takes its files from. A legacy source is the saved import:
//! its folders are checked, locked and verified before anything is read. A
//! folder source is a natively picked folder: it is resolved, checked and
//! scanned, and it has no lock or saved import to verify. It is copied read only.
use super::*;

/// The saved import a legacy request names, copied under the state guard: it
/// must be the import the player confirmed. The caller refuses an import this
/// build cannot honour (see `import_blocker`).
pub(super) fn saved_import(
    saved: Option<migration::LegacyImport>,
    request: &SourceRequest,
) -> Result<migration::LegacyImport, Error> {
    let SourceRequest::Legacy { import_digest } = request else {
        return Err(Error::UnsupportedMode);
    };
    let imported = saved.ok_or(StorageError::Corrupt)?;
    if imported.confirmation != *import_digest {
        let check = rejection::Rejection::new("import_changed")
            .expected(digest_prefix(&imported.confirmation))
            .found(digest_prefix(import_digest));
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    Ok(imported)
}

/// The first 12 characters of a confirmation digest, the closed form C14 keeps.
fn digest_prefix(confirmation: &str) -> String {
    confirmation.chars().take(12).collect()
}

/// `check` with `found` set to the I/O error kind, as a closed snake_case code (C6).
fn with_io_kind(check: rejection::Rejection, error: &std::io::Error) -> rejection::Rejection {
    check.found(crate::storage::game_telemetry::launcher_event::kind(error))
}

/// The source a request names, and the scan of its folder. The state guard is
/// held only to copy the saved import and the state root; nothing below holds
/// it, so the folder is never read under the guard. A folder request resolves
/// to its client root first: when the saved import names that root it is the
/// legacy source, otherwise it is a folder source. In place, the folder must be
/// writable; either way it must be outside the launcher state.
pub(super) fn request_source(
    state: &Mutex<DesktopState>,
    request: &SourceRequest,
    mode: AdoptionMode,
) -> Result<(AdoptionSource, inventory::Index), Error> {
    let (saved, state_root) = {
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        let saved = owner.legacy_import().map_err(|_| StorageError::Corrupt)?;
        (saved, owner.directory.root.clone())
    };
    let SourceRequest::Folder { selected } = request else {
        let imported = saved_import(saved, request)?;
        let scanned = survey::explained(
            &imported.source.game_directory,
            inventory::scan(&imported.source.game_directory, &CancellationToken::new()),
        )?;
        return Ok((AdoptionSource::Legacy(imported), scanned));
    };
    let root = resolve_folder(selected)?;
    if let Some(imported) = saved.filter(|imported| imported.source.game_directory == root) {
        let scanned = survey::explained(&root, inventory::scan(&root, &CancellationToken::new()))?;
        return Ok((AdoptionSource::Legacy(imported), scanned));
    }
    // A copy only reads the folder, so it needs no write access to it.
    if mode == AdoptionMode::InPlace {
        survey::explained(&root, refuse_links(&root))?;
        check_writable(&root)?;
    }
    if root.starts_with(&state_root) || state_root.starts_with(&root) {
        let check = rejection::Rejection::new("folder_overlaps_launcher")
            .entry(".")
            .expected("outside the launcher folder")
            .found("overlaps the launcher folder");
        return Err(rejection::reject(
            StorageError::InvalidDirectory.into(),
            check,
        ));
    }
    let scanned = survey::explained(&root, inventory::scan(&root, &CancellationToken::new()))?;
    let source = AdoptionSource::Folder(FolderSource {
        selected: selected.clone(),
        root,
        scan_digest: digest(&scanned)?,
    });
    Ok((source, scanned))
}

/// The client root a picked folder names: the folder itself, its `Working`, or
/// its `Working\Binaries`. A flat folder (`SGW.exe` beside `SGWGame`) is not a
/// root in either mode: copy cannot map its layout. Names match case-blind.
pub fn resolve_folder(selected: &Path) -> Result<PathBuf, Error> {
    let selected = selected.canonicalize().map_err(|error| {
        let check = with_io_kind(
            rejection::Rejection::new("folder_unreadable").entry("."),
            &error,
        );
        rejection::reject(error.into(), check)
    })?;
    if working_binaries(&selected).is_some() {
        return Ok(selected);
    }
    if is_named(&selected, "Working") && holds_binaries(&selected) {
        return selected
            .parent()
            .map(Path::to_path_buf)
            .ok_or(Error::NotAGameFolder);
    }
    let working = selected
        .parent()
        .filter(|parent| is_named(parent, "Working"));
    if let Some(root) = working.and_then(Path::parent) {
        if is_named(&selected, "Binaries") && holds_sgw(&selected) {
            return Ok(root.to_path_buf());
        }
    }
    tracing::warn!(target: "adoption", "adoption refused: the folder is not a game folder");
    let check = rejection::Rejection::new("not_a_game_folder")
        .entry(".")
        .expected("SGW.exe and the game folders")
        .found("no game layout");
    Err(rejection::reject(Error::NotAGameFolder, check))
}

/// Whether the last component of `path` is `name`, case-blind.
fn is_named(path: &Path, name: &str) -> bool {
    path.file_name()
        .is_some_and(|found| found.to_string_lossy().eq_ignore_ascii_case(name))
}

/// The entry of `dir` whose name matches `name` case-blind, if there is one.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| is_named(path, name))
}

/// `dir\SGW.exe`, matched case-blind, is a file.
fn holds_sgw(dir: &Path) -> bool {
    child_ci(dir, "SGW.exe").is_some_and(|exe| exe.is_file())
}

/// Whether `dir` holds `Binaries\SGW.exe`, case-blind.
fn holds_binaries(dir: &Path) -> bool {
    child_ci(dir, "Binaries").is_some_and(|binaries| holds_sgw(&binaries))
}

/// The `Working\Binaries` folder of a client root, when it holds `SGW.exe`.
fn working_binaries(root: &Path) -> Option<PathBuf> {
    let working = child_ci(root, "Working")?;
    let binaries = child_ci(&working, "Binaries")?;
    holds_sgw(&binaries).then_some(binaries)
}

/// Refuses a folder whose root, `Working` or `Binaries` is a symlink or a reparse
/// point such as a junction. The write probe would follow it to another folder,
/// so this runs before any probe is created.
pub(super) fn refuse_links(root: &Path) -> Result<(), Error> {
    let working = child_ci(root, "Working");
    let binaries = working
        .as_deref()
        .and_then(|working| child_ci(working, "Binaries"));
    for path in [Some(root.to_path_buf()), working, binaries]
        .into_iter()
        .flatten()
    {
        let meta = std::fs::symlink_metadata(&path)?;
        if is_link(&meta) {
            return Err(StorageError::UnsafeFile.into());
        }
    }
    Ok(())
}

/// A symbolic link, or on Windows a reparse point (junction included).
fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_type().is_symlink() || meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// The folder can be changed in place: a probe is created and deleted under
/// `root`, and one is created, renamed and deleted in `Working\Binaries`. The
/// probes are removed on every path.
pub(super) fn check_writable(root: &Path) -> Result<(), Error> {
    let Some(binaries) = working_binaries(root) else {
        tracing::warn!(target: "adoption", "in-place adoption refused: the folder has no Working\\Binaries to change");
        let check = rejection::Rejection::new("folder_not_writable")
            .entry("Working/Binaries")
            .expected("present")
            .found("missing");
        return Err(rejection::reject(Error::FolderNotWritable, check));
    };
    let id = Uuid::new_v4();
    let root_probe = root.join(format!(".cimmeria-write-probe-{id}"));
    let binaries_probe = binaries.join(format!(".cimmeria-write-probe-{id}"));
    let renamed = binaries.join(format!(".cimmeria-write-probe-{id}-renamed"));
    let result = (|| -> std::io::Result<()> {
        std::fs::File::create_new(&root_probe)?;
        std::fs::remove_file(&root_probe)?;
        std::fs::File::create_new(&binaries_probe)?;
        std::fs::rename(&binaries_probe, &renamed)?;
        std::fs::remove_file(&renamed)
    })();
    let _ = std::fs::remove_file(&root_probe);
    let _ = std::fs::remove_file(&binaries_probe);
    let _ = std::fs::remove_file(&renamed);
    result.map_err(|error| {
        tracing::warn!(target: "adoption", ?error, "in-place adoption refused: the folder is not writable");
        let check = rejection::Rejection::new("folder_not_writable")
            .entry(".")
            .expected("writable");
        rejection::reject(Error::FolderNotWritable, with_io_kind(check, &error))
    })
}

/// Imported configuration this copy cannot honour. It is refused, never rewritten.
/// A status query only asks for the blocker, so this records nothing.
pub fn import_blocker(imported: &migration::LegacyImport) -> Option<Error> {
    blocker(imported).map(|(error, _)| error)
}

/// The blocker of an import an operation refuses. The `Err` it returns is recorded
/// as the rejection, so the refusal is named once.
pub(super) fn refuse_blocker(imported: &migration::LegacyImport) -> Option<Error> {
    blocker(imported).map(|(error, check)| rejection::reject(error, check))
}

/// The error an import this copy cannot honour fails with, and its rejection.
fn blocker(imported: &migration::LegacyImport) -> Option<(Error, rejection::Rejection)> {
    if imported.config.manifest_url != crate::catalog::URL {
        let check = rejection::Rejection::new("unsupported_catalog")
            .entry("manifest")
            .expected("built-in catalog")
            .found("custom manifest URL");
        return Some((Error::UnsupportedCatalog, check));
    }
    if imported.config.client_patches.dll_override.is_some() {
        let check = rejection::Rejection::new("unsupported_configuration")
            .expected("no diagnostics module")
            .found("custom diagnostics module");
        return Some((Error::UnsupportedConfiguration, check));
    }
    None
}

/// The folder a source is adopted from: the imported game folder, or the folder
/// root a picked folder resolved to.
pub(super) fn root(source: &AdoptionSource) -> &Path {
    match source {
        AdoptionSource::Legacy(imported) => &imported.source.game_directory,
        AdoptionSource::Folder(folder) => &folder.root,
    }
}

/// Every folder a source reads from. A copy must not overlap any of them.
pub(super) fn directories(source: &AdoptionSource) -> Vec<&Path> {
    match source {
        AdoptionSource::Legacy(imported) => vec![
            imported.source.game_directory.as_path(),
            imported.source.launcher_directory.as_path(),
        ],
        AdoptionSource::Folder(folder) => vec![folder.root.as_path()],
    }
}

/// Every folder a source reads from must already be canonical: nothing is
/// adopted through a link or an alias.
pub(super) fn canonical_paths(source: &AdoptionSource) -> Result<(), Error> {
    for path in directories(source) {
        if path.canonicalize()? != path {
            tracing::warn!(target: "adoption", reason = ?UnsafeReason::Link, "adoption refused: a source folder is not at its canonical path");
            return Err(Error::Unsafe(Box::new(UnsafeReport {
                entries: vec![UnsafeEntry {
                    reason: UnsafeReason::Link,
                    path: None,
                }],
                total: 1,
            })));
        }
    }
    Ok(())
}

/// Locks the old launcher and verifies the imported files. Only a legacy source
/// has a lock and an import to verify; a folder source has neither.
pub(super) fn resolve(source: AdoptionSource) -> Result<(AdoptionSource, Option<File>), Error> {
    let AdoptionSource::Legacy(imported) = source else {
        return Ok((source, None));
    };
    let lock = lock_source(&imported)?;
    verify_import(&imported)?;
    Ok((AdoptionSource::Legacy(imported), Some(lock)))
}

/// The legacy import of a source. A folder source is not adopted yet.
pub(super) fn legacy(source: &AdoptionSource) -> Result<&migration::LegacyImport, Error> {
    match source {
        AdoptionSource::Legacy(imported) => Ok(imported),
        AdoptionSource::Folder(_) => Err(Error::UnsupportedMode),
    }
}

/// A borrowed view of a source, for the verification that compares it.
pub(super) fn borrowed(source: &AdoptionSource) -> AdoptionSourceRef<'_> {
    match source {
        AdoptionSource::Legacy(imported) => AdoptionSourceRef::Legacy(imported),
        AdoptionSource::Folder(folder) => AdoptionSourceRef::Folder(folder),
    }
}

pub(super) fn servers(imported: &migration::LegacyImport) -> Vec<crate::client_setup::LoginServer> {
    imported
        .config
        .login_servers
        .iter()
        .map(|s| crate::client_setup::LoginServer {
            name: s.name.clone(),
            url: s.url.clone(),
        })
        .collect()
}

/// The login servers a source launches with. A legacy source keeps its imported
/// list; a folder source takes the saved preference, or the defaults when none.
pub(super) fn servers_of(
    source: &AdoptionSource,
    prefs: &Preferences,
) -> Vec<crate::client_setup::LoginServer> {
    match source {
        AdoptionSource::Legacy(imported) => servers(imported),
        AdoptionSource::Folder(_) if prefs.login_servers.is_empty() => {
            crate::client_setup::login_servers::default_servers()
        }
        AdoptionSource::Folder(_) => prefs.login_servers.clone(),
    }
}

pub(super) fn lock_source(imported: &migration::LegacyImport) -> Result<File, Error> {
    // Import already created the same executable-adjacent legacy lock. Adoption
    // never creates or replaces anything in either legacy folder.
    let file = inventory::open(&imported.source.launcher_directory.join("launcher.lock"))?;
    file.try_lock().map_err(|_| {
        let check = rejection::Rejection::new("folder_in_use")
            .entry("launcher.lock")
            .expected("unlocked")
            .found("locked");
        rejection::reject(StorageError::InUse.into(), check)
    })?;
    Ok(file)
}

pub(super) fn verify_import(imported: &migration::LegacyImport) -> Result<(), Error> {
    let mut hash = Sha256::new();
    let source = serde_json::to_vec(&imported.source).map_err(|_| StorageError::Corrupt)?;
    hash.update((source.len() as u64).to_le_bytes());
    hash.update(source);
    for path in [
        imported
            .source
            .launcher_directory
            .join("launcher-config.json"),
        imported.source.launcher_directory.join("install.json"),
        imported
            .source
            .game_directory
            .join("launcher-installed.json"),
    ] {
        let mut bytes = Vec::new();
        inventory::open(&path)?
            .take(12 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 12 * 1024 {
            return Err(StorageError::TooLarge.into());
        }
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if actual != imported.confirmation {
        // The three files are hashed together, so no single file can be named.
        let check = rejection::Rejection::new("folder_changed_during_check")
            .expected(digest_prefix(&imported.confirmation))
            .found(digest_prefix(&actual));
        return Err(rejection::reject(Error::SourceChanged, check));
    }
    Ok(())
}
