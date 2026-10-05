//! Original signed bytes, not a reserialized manifest. No network fallback.
use super::*;
use crate::catalog::{verify_release, CatalogError, VerifiedRelease};
use uuid::Uuid;

// Four-byte body length, the catalog's 1 MiB body and 256-byte signature limit.
const MAX_EVIDENCE: usize = 4 + 1024 * 1024 + 256;

/// A signed release reference, independent of the permanent installation owner.
/// The evidence ID selects retained original bytes; the digest binds their exact
/// signed manifest. Neither field establishes installation or mutation authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseIdentity {
    pub evidence_id: Uuid,
    pub manifest_digest: [u8; 32],
}

impl InstallIntent {
    pub fn release_identity(&self) -> ReleaseIdentity {
        ReleaseIdentity {
            evidence_id: self.operation_id,
            manifest_digest: self.manifest_digest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    Storage(StorageError),
    Verification(CatalogError),
    IdentityMismatch,
}
impl From<StorageError> for EvidenceError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}
fn name(id: Uuid) -> String {
    format!("release-evidence-{id}.bin")
}

impl DesktopState {
    pub(super) fn save_release_evidence(
        &mut self,
        id: Uuid,
        release: &VerifiedRelease,
    ) -> Result<(), StorageError> {
        let (body, signature) = release.evidence();
        let mut bytes = Vec::with_capacity(4 + body.len() + signature.len());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(signature);
        if let Err(error) =
            atomic::write_bytes(&self.directory.root, &name(id), &bytes, MAX_EVIDENCE)
        {
            self.preferences_uncertain |= error == StorageError::PersistenceUncertain;
            return Err(error);
        }
        Ok(())
    }

    /// Reverify with the current embedded signing policy and bind to the current
    /// durable intent. Missing/corrupt evidence never triggers a new URL fetch.
    pub fn cached_install_release(&self) -> Result<VerifiedRelease, EvidenceError> {
        let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
        self.release_for_intent(&intent)
    }

    pub(super) fn release_for_intent(
        &self,
        intent: &InstallIntent,
    ) -> Result<VerifiedRelease, EvidenceError> {
        self.verify_release_identity(intent.release_identity())
    }

    /// Reverify retained signed bytes without manufacturing an Install intent.
    /// Update plans can bind old and new releases while retaining one owner.
    pub fn verify_release_identity(
        &self,
        identity: ReleaseIdentity,
    ) -> Result<VerifiedRelease, EvidenceError> {
        if identity.evidence_id.is_nil() {
            return Err(EvidenceError::IdentityMismatch);
        }
        let path = self.directory.root.join(name(identity.evidence_id));
        ensure_regular_or_absent(&path)?;
        let file = File::open(path).map_err(|_| StorageError::Io)?;
        let mut bytes = Vec::new();
        file.take(MAX_EVIDENCE as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| StorageError::Io)?;
        if bytes.len() > MAX_EVIDENCE {
            return Err(StorageError::TooLarge.into());
        }
        let length = bytes.get(..4).ok_or(StorageError::Corrupt)?;
        let length =
            u32::from_le_bytes(length.try_into().map_err(|_| StorageError::Corrupt)?) as usize;
        let body = bytes.get(4..).ok_or(StorageError::Corrupt)?;
        if length > body.len() {
            return Err(StorageError::Corrupt.into());
        }
        let (body, signature) = body.split_at(length);
        let release = verify_release(body, signature).map_err(EvidenceError::Verification)?;
        if release.digest() != identity.manifest_digest {
            return Err(EvidenceError::IdentityMismatch);
        }
        Ok(release)
    }
}

#[cfg(test)]
mod tests;
