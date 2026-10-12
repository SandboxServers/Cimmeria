//! Where an adoption takes its files from. A legacy source is the saved import:
//! its folders are checked, locked and verified before anything is read. A
//! folder source is a natively picked folder; AI-10 resolves it, so until then
//! it is refused before any preparation begins.
use super::*;

/// The saved import a legacy request names, checked under the state guard: it
/// must be the import the player confirmed. The caller refuses an import this
/// build cannot honour (see `import_blocker`).
pub(super) fn saved_import(
    owner: &DesktopState,
    request: &SourceRequest,
) -> Result<migration::LegacyImport, Error> {
    let SourceRequest::Legacy { import_digest } = request else {
        return Err(Error::UnsupportedMode);
    };
    let imported = owner
        .legacy_import()
        .map_err(|_| StorageError::Corrupt)?
        .ok_or(StorageError::Corrupt)?;
    if imported.confirmation != *import_digest {
        return Err(Error::SourceChanged);
    }
    Ok(imported)
}

/// The source a request names, resolved under the state guard before anything
/// is claimed. A folder request resolves to its client root first: when the
/// saved import names that root it is the legacy source, otherwise it is a
/// folder source. In place, the folder must be writable and outside the state.
pub(super) fn request_source(
    owner: &DesktopState,
    request: &SourceRequest,
    mode: AdoptionMode,
) -> Result<AdoptionSource, Error> {
    let SourceRequest::Folder { selected } = request else {
        return saved_import(owner, request).map(AdoptionSource::Legacy);
    };
    let root = resolve_folder(selected, mode)?;
    let saved = owner.legacy_import().map_err(|_| StorageError::Corrupt)?;
    if let Some(imported) = saved.filter(|imported| imported.source.game_directory == root) {
        return Ok(AdoptionSource::Legacy(imported));
    }
    if mode == AdoptionMode::InPlace {
        check_writable(&root)?;
        if root.starts_with(&owner.directory.root) || owner.directory.root.starts_with(&root) {
            return Err(StorageError::InvalidDirectory.into());
        }
    }
    let scanned = inventory::scan(&root, &CancellationToken::new())?;
    Ok(AdoptionSource::Folder(FolderSource {
        selected: selected.clone(),
        root,
        scan_digest: digest(&scanned)?,
    }))
}

/// The client root a picked folder names: the folder itself, its `Working`, or
/// its `Working\Binaries`. A flat folder (`SGW.exe` beside `SGWGame`) is a root
/// only in copy mode. Names match case-blind.
pub(super) fn resolve_folder(selected: &Path, mode: AdoptionMode) -> Result<PathBuf, Error> {
    let selected = selected.canonicalize()?;
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
    if mode == AdoptionMode::Copy
        && holds_sgw(&selected)
        && child_ci(&selected, "SGWGame").is_some()
    {
        return Ok(selected);
    }
    Err(Error::NotAGameFolder)
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

/// The folder can be changed in place: a probe is created and deleted under
/// `root`, and one is created, renamed and deleted in `Working\Binaries`. The
/// probes are removed on every path.
pub(super) fn check_writable(root: &Path) -> Result<(), Error> {
    let binaries = working_binaries(root).ok_or(Error::FolderNotWritable)?;
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
    result.map_err(|_| Error::FolderNotWritable)
}

/// Imported configuration this copy cannot honour. It is refused, never rewritten.
pub fn import_blocker(imported: &migration::LegacyImport) -> Option<Error> {
    if imported.config.manifest_url != crate::catalog::URL {
        return Some(Error::UnsupportedCatalog);
    }
    if imported.config.client_patches.dll_override.is_some() {
        return Some(Error::UnsupportedConfiguration);
    }
    None
}

/// Both imported folders must already be canonical: nothing is adopted through
/// a link or an alias.
pub(super) fn canonical_paths(imported: &migration::LegacyImport) -> Result<(), Error> {
    for path in [
        &imported.source.game_directory,
        &imported.source.launcher_directory,
    ] {
        if path.canonicalize()? != *path {
            return Err(StorageError::UnsafeFile.into());
        }
    }
    Ok(())
}

/// Locks the old launcher and verifies the imported files. Only a legacy source
/// has a lock; a folder source will return `None` once AI-10 adds it.
pub(super) fn resolve(
    imported: migration::LegacyImport,
) -> Result<(AdoptionSource, Option<File>), Error> {
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

pub(super) fn lock_source(imported: &migration::LegacyImport) -> Result<File, Error> {
    // Import already created the same executable-adjacent legacy lock. Adoption
    // never creates or replaces anything in either legacy folder.
    let file = inventory::open(&imported.source.launcher_directory.join("launcher.lock"))?;
    file.try_lock().map_err(|_| StorageError::InUse)?;
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
        return Err(Error::SourceChanged);
    }
    Ok(())
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::super::in_place_tests::Fixture;
    use super::*;

    #[test]
    fn folder_source_is_refused_before_any_preparation_begins() {
        let f = Fixture::new(None);
        let mut request = f.request();
        // A game folder that is not the saved import: a folder source, which
        // `legacy` refuses until part b consumes it.
        let picked = tempfile::tempdir().unwrap();
        layout(picked.path());
        request.source = SourceRequest::Folder {
            selected: picked.path().to_path_buf(),
        };
        let result = preview(
            f.state.clone(),
            request,
            CancellationToken::new(),
            crate::install_progress::ProgressSink::latest().0,
        );
        assert!(matches!(result, Err(Error::UnsupportedMode)));
        let state = f.state.lock().unwrap();
        assert!(state.operations().snapshot().operation.is_none());
        assert!(preparation::list_preparations(&state).unwrap().is_empty());
        let reference_files = std::fs::read_dir(&state.directory.root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".adoption-reference-")
            })
            .count();
        assert_eq!(reference_files, 0);
    }

    /// A minimal PE header: enough for the executable to be recognised.
    fn sgw_exe() -> Vec<u8> {
        let mut exe = vec![0u8; 0x200];
        exe[..2].copy_from_slice(b"MZ");
        exe[60..64].copy_from_slice(&0x128u32.to_le_bytes());
        exe[0x128..0x12c].copy_from_slice(b"PE\0\0");
        exe[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
        exe[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
        exe[0x186..0x188].copy_from_slice(&0x40u16.to_le_bytes());
        exe
    }

    /// Lays `root` out as the installer does: `Working\Binaries\SGW.exe`.
    fn layout(root: &Path) {
        let binaries = root.join("Working").join("Binaries");
        std::fs::create_dir_all(&binaries).unwrap();
        std::fs::write(binaries.join("SGW.exe"), sgw_exe()).unwrap();
    }

    #[test]
    fn picked_root_working_or_binaries_resolve_to_one_root() {
        let dir = tempfile::tempdir().unwrap();
        layout(dir.path());
        let root = dir.path().canonicalize().unwrap();
        for picked in [
            root.clone(),
            root.join("Working"),
            root.join("Working").join("Binaries"),
        ] {
            assert_eq!(
                resolve_folder(&picked, AdoptionMode::InPlace).unwrap(),
                root
            );
        }
    }

    #[test]
    fn folder_without_sgw_is_not_a_game_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Working")).unwrap();
        assert!(matches!(
            resolve_folder(dir.path(), AdoptionMode::Copy),
            Err(Error::NotAGameFolder)
        ));
    }

    #[test]
    fn flat_folder_is_a_root_only_in_copy_mode() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), sgw_exe()).unwrap();
        std::fs::create_dir(dir.path().join("SGWGame")).unwrap();
        let root = dir.path().canonicalize().unwrap();
        assert_eq!(
            resolve_folder(dir.path(), AdoptionMode::Copy).unwrap(),
            root
        );
        assert!(matches!(
            resolve_folder(dir.path(), AdoptionMode::InPlace),
            Err(Error::NotAGameFolder)
        ));
    }

    #[test]
    fn saved_import_naming_the_folder_is_legacy_another_is_folder() {
        let f = Fixture::new(None);
        let state = f.state.lock().unwrap();
        let named = SourceRequest::Folder {
            selected: f.folder().to_path_buf(),
        };
        layout(f.folder());
        let adopted = request_source(&state, &named, AdoptionMode::Copy).unwrap();
        assert!(matches!(adopted, AdoptionSource::Legacy(_)));
        let other = tempfile::tempdir().unwrap();
        layout(other.path());
        let request = SourceRequest::Folder {
            selected: other.path().to_path_buf(),
        };
        let adopted = request_source(&state, &request, AdoptionMode::Copy).unwrap();
        assert!(matches!(adopted, AdoptionSource::Folder(_)));
    }

    /// Restores the write permission the test denies, even when it fails.
    #[cfg(windows)]
    struct RestoreWrite {
        user: String,
        folder: PathBuf,
    }

    #[cfg(windows)]
    impl Drop for RestoreWrite {
        fn drop(&mut self) {
            let _ = std::process::Command::new("icacls")
                .arg(&self.folder)
                .arg("/remove:d")
                .arg(&self.user)
                .status();
        }
    }

    #[cfg(windows)]
    #[test]
    fn root_denied_write_is_folder_not_writable() {
        let dir = tempfile::tempdir().unwrap();
        layout(dir.path());
        let user = std::env::var("USERNAME").unwrap();
        let status = std::process::Command::new("icacls")
            .arg(dir.path())
            .arg("/deny")
            .arg(format!("{user}:(W)"))
            .status()
            .unwrap();
        assert!(status.success());
        let _restore = RestoreWrite {
            user,
            folder: dir.path().to_path_buf(),
        };
        assert!(matches!(
            check_writable(dir.path()),
            Err(Error::FolderNotWritable)
        ));
    }
}
