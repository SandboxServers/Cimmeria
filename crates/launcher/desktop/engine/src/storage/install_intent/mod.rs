//! Durable native install inputs. Admission does not create or modify game files.
use super::*;
use crate::{catalog::VerifiedRelease, client_setup::LoginServer, OperationKind};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// The extraction backend is immutable operation input, never inferred from the
/// host OS during recovery. Native is omitted to preserve schema-1 intent hashes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExtractionBackend {
    #[default]
    Native,
    Wine {
        runtime_sha256: [u8; 32],
        helper_sha256: [u8; 32],
    },
}
impl ExtractionBackend {
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native)
    }
}

pub struct AdmissionRequest<'a> {
    pub id: Uuid,
    pub operation_revision: u64,
    pub preferences_revision: u64,
    pub release: &'a VerifiedRelease,
    pub login_servers: Vec<LoginServer>,
    pub backend: ExtractionBackend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallIntent {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub preferences_revision: u64,
    /// Canonical existing parent plus the selected final component.
    pub destination: PathBuf,
    pub manifest_digest: [u8; 32],
    pub login_servers: Vec<LoginServer>,
    #[serde(default, skip_serializing_if = "ExtractionBackend::is_native")]
    pub backend: ExtractionBackend,
}
impl InstallIntent {
    pub(super) fn digest(&self) -> Result<[u8; 32], StorageError> {
        let bytes = serde_json::to_vec(self).map_err(|_| StorageError::Corrupt)?;
        Ok(Sha256::digest(bytes).into())
    }
}

#[derive(Debug)]
pub struct InstallAdmission {
    pub intent: InstallIntent,
    /// Only true after both intent and operation admission are durable.
    pub dispatch: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentError {
    LauncherTooOld,
    Storage(StorageError),
    Operation(ContractError),
}
impl From<StorageError> for IntentError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}
impl From<ContractError> for IntentError {
    fn from(error: ContractError) -> Self {
        Self::Operation(error)
    }
}

impl DesktopState {
    /// Native-only: the bridge supplies IDs/revisions, never a manifest or servers.
    /// This first-install contract refuses existing nonempty destinations; adoption
    /// and repair require their own ownership evidence.
    pub fn admit_install(
        &mut self,
        id: Uuid,
        expected_operation_revision: u64,
        expected_preferences_revision: u64,
        release: &VerifiedRelease,
        login_servers: Vec<LoginServer>,
    ) -> Result<InstallAdmission, IntentError> {
        self.admit_install_backend(AdmissionRequest {
            id,
            operation_revision: expected_operation_revision,
            preferences_revision: expected_preferences_revision,
            release,
            login_servers,
            backend: ExtractionBackend::Native,
        })
    }

    pub fn admit_install_backend(
        &mut self,
        request: AdmissionRequest<'_>,
    ) -> Result<InstallAdmission, IntentError> {
        self.admit_install_with(request, |operations, id, digest, revision| {
            operations
                .begin(id, OperationKind::Install, digest, revision)
                .map(|(_, dispatch)| dispatch)
        })
    }

    fn admit_install_with(
        &mut self,
        request: AdmissionRequest<'_>,
        commit: impl FnOnce(
            &mut Operations<FileJournal>,
            Uuid,
            [u8; 32],
            u64,
        ) -> Result<bool, ContractError>,
    ) -> Result<InstallAdmission, IntentError> {
        self.ensure_updater_idle()?;
        let AdmissionRequest {
            id,
            operation_revision: expected_operation_revision,
            preferences_revision: expected_preferences_revision,
            release,
            login_servers,
            backend,
        } = request;
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if self.compatibility.for_release(release).blocks() {
            return Err(IntentError::LauncherTooOld);
        }
        if let Some(operation) = &self.operations.snapshot().operation {
            if operation.id == id {
                let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
                if intent.preferences_revision != expected_preferences_revision
                    || intent.manifest_digest != release.digest()
                    || intent.login_servers != login_servers
                    || intent.backend != backend
                {
                    return Err(ContractError::IdentityConflict.into());
                }
                return Ok(InstallAdmission {
                    intent,
                    dispatch: false,
                });
            }
        }
        if expected_operation_revision != self.operations.snapshot().revision {
            return Err(ContractError::StaleRevision.into());
        }
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal())
        {
            return Err(ContractError::Busy.into());
        }
        if expected_preferences_revision != self.preferences.revision {
            return Err(StorageError::StaleRevision.into());
        }
        let selected = self
            .preferences
            .install_directory
            .as_deref()
            .ok_or(StorageError::InvalidDirectory)?;
        let destination = fresh_destination(selected, &self.directory.root)?;
        let intent = InstallIntent {
            schema_version: 1,
            operation_id: id,
            preferences_revision: expected_preferences_revision,
            destination,
            manifest_digest: release.digest(),
            login_servers,
            backend,
        };
        let digest = intent.digest()?;
        // Preserve authenticated original bytes before intent/admission. Never
        // depend on the mutable GitHub release URL for restart reconciliation.
        self.save_release_evidence(id, release)?;
        // A crash here leaves an orphan intent, never a dispatched mutation.
        if let Err(error) = atomic::write(&self.directory.root, &intent_name(id), &intent) {
            self.preferences_uncertain |= error == StorageError::PersistenceUncertain;
            return Err(error.into());
        }
        let dispatch = commit(
            &mut self.operations,
            id,
            digest,
            expected_operation_revision,
        )?;
        Ok(InstallAdmission { intent, dispatch })
    }

    /// Returns only the immutable intent matching the current admitted operation.
    /// Missing/mismatched/corrupt evidence is never interpreted as permission to run.
    pub fn install_intent(&self) -> Result<Option<InstallIntent>, StorageError> {
        let Some(operation) = &self.operations.snapshot().operation else {
            return Ok(None);
        };
        if operation.kind != OperationKind::Install {
            return Ok(None);
        }
        let intent: InstallIntent = read(&self.directory.root.join(intent_name(operation.id)))?
            .ok_or(StorageError::Corrupt)?;
        if intent.schema_version != 1 {
            return Err(StorageError::UnsupportedSchema);
        }
        if intent.operation_id != operation.id || intent.digest()? != operation.intent_digest {
            return Err(StorageError::Corrupt);
        }
        Ok(Some(intent))
    }
}

fn intent_name(id: Uuid) -> String {
    format!("install-intent-{id}.json")
}

pub(super) fn fresh_destination(
    selected: &Path,
    state_root: &Path,
) -> Result<PathBuf, StorageError> {
    use std::path::Component;
    if !selected.is_absolute()
        || selected
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(StorageError::InvalidDirectory);
    }
    let name = selected.file_name().ok_or(StorageError::InvalidDirectory)?;
    let parent = selected
        .parent()
        .ok_or(StorageError::InvalidDirectory)?
        .canonicalize()
        .map_err(|_| StorageError::InvalidDirectory)?;
    if !parent.is_dir() {
        return Err(StorageError::InvalidDirectory);
    }
    let destination = parent.join(name);
    if destination.starts_with(state_root) || state_root.starts_with(&destination) {
        return Err(StorageError::InvalidDirectory);
    }
    match std::fs::symlink_metadata(&destination) {
        Ok(metadata) => {
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || std::fs::read_dir(&destination)
                    .map_err(|_| StorageError::Io)?
                    .next()
                    .is_some()
            {
                return Err(StorageError::InvalidDirectory);
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => (),
        Err(_) => return Err(StorageError::Io),
    }
    Ok(destination)
}

#[cfg(test)]
mod tests;
