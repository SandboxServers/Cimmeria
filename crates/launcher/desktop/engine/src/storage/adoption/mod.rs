//! Verified, separately owned copies on macOS; verified in-place ownership of
//! the imported folder elsewhere (D-LX12). A legacy ledger is never verification.
//! Native composition only: dispatch these blocking functions on retained workers.
mod artifacts;
mod comparison;
mod in_place;
#[cfg(not(target_os = "macos"))]
mod in_place_preview;
// Where the in-place backend is the one previews use.
#[cfg(all(test, not(target_os = "macos")))]
mod folder_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod in_place_recovery_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod in_place_tests;
mod inventory;
mod model;
pub(crate) mod preparation;
mod publication;
pub use preparation::{
    abandon_preparation, inspect_preparation, list_preparations, PreparationRecord,
};
mod reference;
mod source;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod verification;
mod worker;
pub use worker::{start_confirmation, start_preview, ConfirmationWorker, PreviewWorker};
#[cfg(all(test, target_os = "macos"))]
mod artifacts_tests;
#[cfg(all(test, target_os = "macos"))]
mod cab_fixture;
// Every platform: it needs no copy, only a persisted plan and its journal.
#[cfg(test)]
mod digest_tests;
#[cfg(test)]
mod golden_tests;
#[cfg(test)]
mod index_parity_tests;
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
pub use source::import_blocker;
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
    source: AdoptionSource,
    before: Preferences,
    preparation: preparation::Ownership,
    release: VerifiedRelease,
    scanned: inventory::Index,
    reference: reference::Reference,
    _legacy_lock: Option<File>,
}
impl Preview {
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn signed_release(&self) -> &VerifiedRelease {
        &self.release
    }
    /// The legacy import this preview adopts. A folder source has none yet.
    pub(super) fn legacy(&self) -> Result<&migration::LegacyImport, Error> {
        source::legacy(&self.source)
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
        self.legacy()
            .is_ok_and(|imported| imported.config.telemetry.opted_in)
            && !self.report.game_telemetry_available
    }
    /// The one consent rule, shared by review presentation and confirmation.
    pub fn consent(&self, choices: &Choices) -> Result<(), Error> {
        if self.report.blocking().next().is_some() {
            return Err(Error::VerificationFailed);
        }
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
/// Which source a preview adopts, as the caller names it.
pub enum SourceRequest {
    /// The saved import, as today. The digest is the one the player confirmed.
    Legacy { import_digest: String },
    /// A natively picked folder; AI-10 resolves it.
    Folder { selected: PathBuf },
}
pub struct PreviewRequest {
    pub source: SourceRequest,
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
    // macOS makes a verified copy. Every other host adopts the imported folder
    // in place (D-LX12); there is no copy destination to choose.
    #[cfg(not(target_os = "macos"))]
    {
        let Backend::Native = backend;
        in_place_preview::preview(state, request, cancel, progress, transport)
    }
    #[cfg(target_os = "macos")]
    copy_preview(state, request, cancel, progress, backend, transport)
}
#[cfg(target_os = "macos")]
fn copy_preview(
    state: Arc<Mutex<DesktopState>>,
    request: PreviewRequest,
    cancel: CancellationToken,
    progress: crate::install_progress::ProgressSink,
    backend: Backend,
    transport: Option<artifacts::Transport>,
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
    let (adopted, scanned) = source::request_source(&state, &request.source, AdoptionMode::Copy)?;
    let (imported, before, destination) = {
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        // A folder source is refused by `legacy` until AI-12 copies it.
        let imported = source::legacy(&adopted)?.clone();
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
    source::canonical_paths(&imported)?;
    let (adopted, legacy_lock) = source::resolve(adopted)?;
    let imported = source::legacy(&adopted)?;
    let servers = source::servers(imported);
    let identity = match &backend {
        Backend::Native => ExtractionBackend::Native,
        #[cfg(target_os = "macos")]
        Backend::Wine(helper) => helper.backend(),
    };
    let mut preparation =
        preparation::Ownership::claim(state.clone(), &request, identity, servers.clone())?;
    let prepared = verification::reference_and_comparison(
        &state,
        &request,
        backend,
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
        imported_identity: Some(imported.identity.clone()),
        requested_config: Some(imported.config.clone()),
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
        download: None,
        backup: None,
    };
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
/// Read-only accessor for effective settings; adoption stays the only reader of
/// its own records.
pub(super) fn published_plan(
    state: &DesktopState,
    intent: &InstallIntent,
    provenance: &Provenance,
) -> Result<Plan, StorageError> {
    publication::published_plan(state, intent, provenance).map_err(|error| match error {
        Error::Storage(e) => e,
        _ => StorageError::Corrupt,
    })
}
