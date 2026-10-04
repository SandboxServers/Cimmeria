//! Verified, separately owned copies. A legacy ledger is never verification.
//! Native composition only: dispatch these blocking functions on retained workers.
mod artifacts;
mod comparison;
mod inventory;
mod model;
pub(crate) mod preparation;
mod publication;
pub use preparation::{
    abandon_preparation, inspect_preparation, list_preparations, PreparationRecord,
};
mod reference;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod worker;
pub use worker::{start_confirmation, start_preview, ConfirmationWorker, PreviewWorker};
#[cfg(all(test, target_os = "macos"))]
mod artifacts_tests;
#[cfg(all(test, target_os = "macos"))]
mod cab_fixture;
#[cfg(all(test, target_os = "macos"))]
mod preparation_tests;
#[cfg(all(test, target_os = "macos"))]
mod publication_tests;
#[cfg(all(test, target_os = "macos"))]
mod recovery_tests;
#[cfg(all(test, target_os = "macos"))]
mod reference_tests;
#[cfg(all(test, target_os = "macos"))]
mod tests;
use super::*;
use crate::{catalog::VerifiedRelease, OperationKind, OperationState};
pub use model::*;
pub use publication::{abandon, inspect, interrupted, recover, Interrupted};
use sha2::{Digest, Sha256};
use std::{io::Read, sync::Mutex};
use tokio_util::sync::CancellationToken;
#[cfg(target_os = "macos")]
pub use worker::start_preview_wine;
pub(super) enum Backend {
    Native,
    #[cfg(target_os = "macos")]
    Wine(crate::mac_wine::HelperResource),
}
use uuid::Uuid;

/// This handle cannot be serialized or fabricated by an IPC caller. It retains
/// the source launcher lock and private authenticated reference until confirmation.
pub struct Preview {
    state: Arc<Mutex<DesktopState>>,
    report: Report,
    imported: migration::LegacyImport,
    before: Preferences,
    preparation: preparation::Ownership,
    release: VerifiedRelease,
    source: inventory::Index,
    reference: reference::Reference,
    _legacy_lock: File,
}
impl Preview {
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn signed_release(&self) -> &VerifiedRelease {
        &self.release
    }
    /// Reference bytes replace managed files that are missing, modified or carry
    /// the launcher's known setup transform; that needs explicit consent.
    pub fn requires_normalization(&self) -> bool {
        self.report.files.iter().any(|d| {
            matches!(
                d.classification,
                Classification::Modified | Classification::Missing | Classification::KnownTransform
            )
        })
    }
    /// Imported game telemetry consent cannot be honoured by this build.
    pub fn requires_telemetry_acceptance(&self) -> bool {
        self.imported.config.telemetry.opted_in && !self.report.game_telemetry_available
    }
    /// The one consent rule, shared by review presentation and confirmation.
    pub fn consent(&self, choices: &Choices) -> Result<(), Error> {
        if !choices.old_game_closed
            || (self.requires_normalization() && !choices.normalize_managed_files)
            || (self.requires_telemetry_acceptance() && !choices.accept_unavailable_game_telemetry)
        {
            return Err(Error::ConsentRequired);
        }
        Ok(())
    }
    /// Release the source lock and private reference without confirming. Blocking:
    /// the caller must not hold the state guard.
    pub fn discard(mut self) {
        self.preparation.release();
    }
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
pub struct PreviewRequest {
    pub import_digest: String,
    pub destination: PathBuf,
    pub operation_revision: u64,
    pub preferences_revision: u64,
    pub release: VerifiedRelease,
    /// Local fixture paths only. `None` fetches the signed blobs natively.
    pub artifacts: Option<Artifacts>,
}
/// Native worker entry point; no renderer-supplied release/path is accepted by a
/// bridge. Dropping a Preview cancels its unconfirmed private reference only.
pub fn preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: crate::install_progress::ProgressSink,
) -> Result<Preview, Error> {
    preview_using(state, request, cancel, progress, Backend::Native, None)
}

pub(super) fn preview_using(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: crate::install_progress::ProgressSink,
    backend: Backend,
    transport: Option<artifacts::Transport>,
) -> Result<Preview, Error> {
    // Mac-native filesystem identity and no-clobber promotion are implemented in
    // this bounded phase. Other hosts fail closed pending their native tests.
    if !cfg!(target_os = "macos") {
        return Err(Error::UnsupportedConfiguration);
    }
    let (imported, before, destination) = {
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
            return Err(blocker);
        }
        let destination =
            super::install_intent::fresh_destination(&request.destination, &owner.directory.root)?;
        // Confirmation creates this folder itself. Refuse an existing one now,
        // before a long preparation, rather than after the review.
        if std::fs::symlink_metadata(&destination).is_ok() {
            return Err(StorageError::InvalidDirectory.into());
        }
        for source in [
            &imported.source.game_directory,
            &imported.source.launcher_directory,
        ] {
            if destination.starts_with(source)
                || source.starts_with(&destination)
                || owner.directory.root.starts_with(source)
                || source.starts_with(&owner.directory.root)
            {
                return Err(StorageError::InvalidDirectory.into());
            }
        }
        (imported, owner.preferences.clone(), destination)
    };
    for path in [
        &imported.source.game_directory,
        &imported.source.launcher_directory,
    ] {
        if path.canonicalize()? != *path {
            return Err(StorageError::UnsafeFile.into());
        }
    }
    let legacy_lock = lock_source(&imported)?;
    verify_import(&imported)?;
    let source = inventory::scan(&imported.source.game_directory, &cancel)?;
    let servers = servers(&imported);
    let identity = match &backend {
        Backend::Native => ExtractionBackend::Native,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => helper.backend(),
    };
    let mut preparation =
        preparation::Ownership::claim(state.clone(), &request, identity, servers.clone())?;
    let prepared = reference_and_comparison(
        &state,
        &request,
        backend,
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
            // This worker holds no state guard, so settle the owner now rather
            // than leave a Running operation that nothing is working on.
            preparation.release();
            return Err(error);
        }
    };
    let report = Report {
        preview_handle: preparation.record.id,
        source: imported.source.game_directory.clone(),
        destination,
        release_digest: request.release.digest(),
        imported_identity: imported.identity.clone(),
        requested_config: imported.config.clone(),
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
    };
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
#[allow(clippy::too_many_arguments)]
fn reference_and_comparison(
    state: &Arc<Mutex<DesktopState>>,
    request: &PreviewRequest,
    backend: Backend,
    transport: Option<artifacts::Transport>,
    preparation: &mut preparation::Ownership,
    imported: &migration::LegacyImport,
    source: &inventory::Index,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: crate::install_progress::ProgressSink,
) -> Result<(reference::Reference, Vec<Difference>), Error> {
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
    let files = comparison::compare(&imported.source.game_directory, source, &reference)?;
    if !files.iter().any(|d| {
        matches!(
            d.classification,
            Classification::Matched | Classification::KnownTransform
        )
    }) {
        return Err(Error::NoReusableFiles);
    }
    if inventory::scan(&imported.source.game_directory, cancel)? != *source {
        return Err(Error::SourceChanged);
    }
    verify_import(imported)?;
    Ok((reference, files))
}
/// Work ID is the only new identity provided at confirmation; content ownership
/// gets a separate UUID. The caller consumes the exact native-held Preview once.
pub fn confirm(
    preview: Preview,
    work_id: Uuid,
    preview_handle: Uuid,
    choices: Choices,
    cancel: CancellationToken,
) -> Result<Provenance, Error> {
    publication::confirm(
        preview,
        work_id,
        preview_handle,
        choices,
        cancel,
        &crate::install_progress::ProgressSink::latest().0,
        |_| Ok(()),
    )
}
fn servers(imported: &migration::LegacyImport) -> Vec<crate::client_setup::LoginServer> {
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
fn idle(
    state: &DesktopState,
    operation_revision: u64,
    preferences_revision: u64,
) -> Result<(), Error> {
    state.ensure_updater_idle()?;
    if state.requires_reopen() {
        return Err(StorageError::PersistenceUncertain.into());
    }
    if state.operations.snapshot().revision != operation_revision {
        return Err(ContractError::StaleRevision.into());
    }
    if state.preferences.revision != preferences_revision {
        return Err(StorageError::StaleRevision.into());
    }
    if state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| !op.state.terminal())
    {
        return Err(ContractError::Busy.into());
    }
    Ok(())
}
fn lock_source(imported: &migration::LegacyImport) -> Result<File, Error> {
    // Import already created the same executable-adjacent legacy lock. Adoption
    // never creates or replaces anything in either legacy folder.
    let file = inventory::open(&imported.source.launcher_directory.join("launcher.lock"))?;
    file.try_lock().map_err(|_| StorageError::InUse)?;
    Ok(file)
}
fn verify_import(imported: &migration::LegacyImport) -> Result<(), Error> {
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

pub(super) fn verify_provenance(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<(), StorageError> {
    publication::verify_provenance(state, intent, provenance).map_err(|error| match error {
        Error::Storage(e) => e,
        _ => StorageError::Corrupt,
    })
}
