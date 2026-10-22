//! Verified, separately owned copies on macOS; verified in-place ownership of
//! the imported folder elsewhere (D-LX12). A legacy ledger is never verification.
//! Native composition only: dispatch these blocking functions on retained workers.
mod artifacts;
mod comparison;
mod in_place;
mod in_place_abandon;
mod in_place_preview;
mod in_place_replace;
// Where the in-place backend is the one previews use.
#[cfg(all(test, not(target_os = "macos")))]
mod folder_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod in_place_recovery_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod in_place_replace_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod in_place_tests;
mod index_compare;
mod index_source;
mod inventory;
mod light;
#[cfg(all(test, not(target_os = "macos")))]
mod light_cooked_tests;
mod light_order;
mod light_plan;
#[cfg(test)]
mod light_plan_tests;
mod light_replay;
#[cfg(all(test, not(target_os = "macos")))]
mod light_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod light_update_guard_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod light_update_tests;
mod model;
pub(crate) mod preparation;
mod publication;
mod publication_recovery;
mod setup_digest;
// LT-29 onward use `digest`, `relative` and `http_status`.
#[allow(dead_code)]
mod rejection;
mod replay;
#[cfg(all(test, not(target_os = "macos")))]
mod unsafe_source_tests;
pub use preparation::{
    abandon_preparation, inspect_preparation, list_preparations, PreparationRecord,
};
mod reference;
mod source;
mod survey;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod verification;
mod worker;
pub use worker::{start_confirmation, start_preview, ConfirmationWorker, PreviewWorker};
#[cfg(test)]
mod artifacts_tests;
#[cfg(all(test, target_os = "macos"))]
mod cab_fixture;
// Every platform: it needs no copy, only a persisted plan and its journal.
#[cfg(test)]
mod confirmation_tests;
#[cfg(test)]
mod digest_tests;
#[cfg(test)]
mod golden_tests;
#[cfg(test)]
mod index_parity_tests;
#[cfg(test)]
mod preparation_tests;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(all(test, target_os = "macos"))]
mod reference_tests;
#[cfg(test)]
mod replay_tests;
#[cfg(all(test, not(target_os = "macos")))]
mod source_tests;
#[cfg(test)]
mod survey_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod verification_tests;
use super::*;
use crate::{catalog::VerifiedRelease, OperationKind, OperationState};
pub use index_compare::IndexState;
pub use model::*;
pub use publication::{abandon, inspect, interrupted, recover, Interrupted};
use sha2::{Digest, Sha256};
pub use source::{import_blocker, resolve_folder};
use std::{io::Read, sync::Mutex};
use tokio_util::sync::CancellationToken;
#[cfg(target_os = "macos")]
pub use worker::start_preview_wine;
pub(super) enum Backend {
    Native,
    #[cfg(target_os = "macos")]
    Wine(crate::mac_wine::HelperResource),
}
pub use index_source::IndexFallback;
pub use rejection::{refuse, scope as rejection_scope, Rejection};
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
    /// Built indexes only: the reconstructed reference the moves may read.
    reference: Option<reference::Reference>,
    /// The digest and release paths of the post-setup tree (see `Compared`).
    expected_digest: [u8; 32],
    expected_paths: Vec<String>,
    /// The files the replay stored for the adopted tree (AI-09); empty when the
    /// review needs none.
    replacements: Vec<Replacement>,
    /// The published index's digest, recorded in the provenance; `None` when built.
    file_index_sha256: Option<String>,
    /// A light adoption's skipped ops and review summary; `None` for the others.
    light: Option<light::Light>,
    _legacy_lock: Option<File>,
}
impl Preview {
    pub fn report(&self) -> &Report {
        &self.report
    }
    /// The replacement store entries the adopted tree takes, one per target.
    pub fn replacements(&self) -> &[Replacement] {
        &self.replacements
    }
    pub fn signed_release(&self) -> &VerifiedRelease {
        &self.release
    }
    /// The source this preview adopts: a legacy import or a folder.
    pub fn source(&self) -> AdoptionSourceRef<'_> {
        source::borrowed(&self.source)
    }
    /// The legacy import this preview adopts. A folder source has none.
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
        let mut blocking = self.report.blocking(&self.replacements);
        if let Some(first) = blocking.next() {
            let count = 1 + blocking.count();
            return Err(rejection::reject(
                Error::VerificationFailed,
                Rejection::new("blocking_files")
                    .entry(&first.path)
                    .found(count.to_string()),
            ));
        }
        // Replacing a changed or absent file is a choice of its own (D-AI1);
        // a patch-only file needs no extra consent (D-AI7). A copy replaces nothing.
        let replaces = self.report.in_place()
            && self.report.files.iter().any(|d| {
                matches!(
                    d.classification,
                    Classification::Modified | Classification::Missing
                )
            });
        if !choices.old_game_closed
            || (self.requires_normalization() && !choices.normalize_managed_files)
            || (replaces && !choices.replace_modified_files)
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
    /// How the preview adopts the source. It picks the path, not the host OS.
    pub mode: AdoptionMode,
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
    // The request's mode picks the path on every OS. In place adopts the folder
    // where it is (D-LX12); a copy writes a separately owned tree.
    match request.mode {
        AdoptionMode::InPlace => {
            // In place never runs the Wine helper; only a copy extracts through it.
            let _ = backend;
            in_place_preview::preview(state, request, cancel, progress, transport, false)
        }
        // Light adopts in place too; only the comparison differs (LA-11).
        AdoptionMode::Light => {
            let _ = backend;
            in_place_preview::preview(state, request, cancel, progress, transport, true)
        }
        AdoptionMode::Copy => copy_preview(state, request, cancel, progress, backend, transport),
    }
}
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
        if let crate::launcher_compatibility::MinimumStatus::TooOld { required } =
            owner.compatibility.for_release(&request.release)
        {
            return Err(rejection::reject(
                Error::LauncherTooOld,
                Rejection::new("launcher_too_old")
                    .expected(required)
                    .found(crate::launcher_compatibility::compiled_version()),
            ));
        }
        if owner.installed_content()?.is_some() {
            return Err(rejection::reject(
                ContractError::IdentityConflict.into(),
                Rejection::new("identity_conflict"),
            ));
        }
    }
    // Resolved and scanned with no state guard held.
    let (adopted, scanned) = source::request_source(&state, &request.source, AdoptionMode::Copy)?;
    let (before, destination) = {
        let owner = state.lock().map_err(|_| StorageError::Io)?;
        let destination =
            super::install_intent::fresh_destination(&request.destination, &owner.directory.root)?;
        // The folder's last component only: the full path is never recorded.
        let name = destination
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        // Confirmation creates this folder itself. Refuse an existing one now,
        // before a long preparation, rather than after the review.
        if std::fs::symlink_metadata(&destination).is_ok() {
            return Err(rejection::reject(
                StorageError::InvalidDirectory.into(),
                Rejection::new("destination_exists").entry(&name),
            ));
        }
        // The copy never overlaps the folders it reads, nor the launcher state.
        for dir in source::directories(&adopted) {
            if destination.starts_with(dir)
                || dir.starts_with(&destination)
                || owner.directory.root.starts_with(dir)
                || dir.starts_with(&owner.directory.root)
            {
                return Err(rejection::reject(
                    StorageError::InvalidDirectory.into(),
                    Rejection::new("destination_overlaps").entry(&name),
                ));
            }
        }
        (owner.preferences.clone(), destination)
    };
    source::canonical_paths(&adopted)?;
    let (adopted, legacy_lock) = source::resolve(adopted)?;
    let imported = source::legacy(&adopted).ok();
    let servers = source::servers_of(&adopted, &before);
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
        AdoptionMode::Copy,
        &scanned,
        &servers,
        &cancel,
        progress,
    );
    let verification::Compared {
        reference,
        expected_digest,
        expected_paths,
        files,
        download,
        replacements,
        file_index_sha256,
        index_fallback,
        ..
    } = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            // This worker holds no state guard, so settle the owner now rather
            // than leave a Running operation that nothing is working on.
            preparation.release();
            // Unsafe sources and user cancels are owned by other checks (AU-02, cancel), not C14.
            if matches!(
                error,
                Error::Unsafe(_) | Error::Cancelled | Error::Storage(StorageError::UnsafeFile)
            ) {
                return Err(error);
            }
            let check = Rejection::new("comparison_failed").found(rejection::code(&error));
            return Err(rejection::reject(error, check));
        }
    };
    let report = Report {
        preview_handle: preparation.record.id,
        source: source::root(&adopted).to_path_buf(),
        destination,
        release_digest: request.release.digest(),
        imported_identity: imported.map(|imported| imported.identity.clone()),
        requested_config: imported.map(|imported| imported.config.clone()),
        files,
        user_data_remains_in_source: true,
        game_telemetry_available: false,
        index_fallback,
        download: Some(download),
        backup: None,
        login_replaced: false,
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
        expected_digest,
        expected_paths,
        replacements,
        file_index_sha256,
        light: None,
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
    let revision = state.operations.snapshot().revision;
    if revision != operation_revision {
        return Err(rejection::reject(
            ContractError::StaleRevision.into(),
            Rejection::new("stale_revision")
                .expected(operation_revision.to_string())
                .found(revision.to_string()),
        ));
    }
    if state.preferences.revision != preferences_revision {
        return Err(rejection::reject(
            StorageError::StaleRevision.into(),
            Rejection::new("stale_revision")
                .expected(preferences_revision.to_string())
                .found(state.preferences.revision.to_string()),
        ));
    }
    if state
        .operations
        .snapshot()
        .operation
        .as_ref()
        .is_some_and(|op| !op.state.terminal())
    {
        return Err(rejection::reject(
            ContractError::Busy.into(),
            Rejection::new("busy"),
        ));
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
