//! The signed reference is reconstructed and compared with the adopted source.
//! The game directory is `Folder.root` for a folder source or the imported game
//! folder for a legacy one; only a legacy source has an import to verify.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn reference_and_comparison(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    backend: Backend,
    transport: Option<artifacts::Transport>,
    preparation: &mut preparation::Ownership,
    adopted: AdoptionSourceRef<'_>,
    scanned: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: crate::install_progress::ProgressSink,
) -> Result<(reference::Reference, Vec<Difference>), Error> {
    let (folder, legacy) = match adopted {
        AdoptionSourceRef::Legacy(imported) => (&imported.source.game_directory, Some(imported)),
        AdoptionSourceRef::Folder(picked) => (&picked.root, None),
    };
    let fetched;
    let artifacts = match &request.artifacts {
        Some(artifacts) => artifacts,
        None => {
            let transport = match transport {
                Some(transport) => transport,
                None => artifacts::Transport::production()?,
            };
            let root = state
                .lock()
                .map_err(|_| StorageError::Io)?
                .state_root()
                .to_path_buf();
            fetched = artifacts::fetch(&root, &transport, &request.release, cancel, &progress)?;
            &fetched
        }
    };
    let adapter: Option<Box<dyn crate::install::SeedExtractor>> = match backend {
        Backend::Native => None,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => {
            preparation.uncertain = true;
            Some(Box::new(
                tokio::runtime::Handle::try_current()
                    .map_err(|_| StorageError::Io)?
                    .block_on(crate::mac_wine::WineSeedExtractor::prepare(
                        state.clone(),
                        preparation.record.id,
                        helper.path().to_path_buf(),
                        cancel.clone(),
                        progress.clone(),
                    ))
                    .map_err(|_| StorageError::PersistenceUncertain)?,
            ))
        }
    };
    let reference = reference::reconstruct(
        &preparation.record.directory,
        &request.release,
        artifacts,
        servers,
        cancel,
        progress,
        adapter.as_deref(),
    )?;
    preparation.uncertain = false;
    drop(adapter);
    let files = comparison::compare(folder, scanned, &reference)?;
    if !files.iter().any(|d| {
        matches!(
            d.classification,
            Classification::Matched | Classification::KnownTransform
        )
    }) {
        return Err(Error::NoReusableFiles);
    }
    if inventory::scan(folder, cancel)? != *scanned {
        return Err(Error::SourceChanged);
    }
    if let Some(imported) = legacy {
        source::verify_import(imported)?;
    }
    Ok((reference, files))
}
