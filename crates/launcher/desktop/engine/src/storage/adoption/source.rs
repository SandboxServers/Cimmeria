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
        request.source = SourceRequest::Folder {
            selected: f.folder().to_path_buf(),
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
}
