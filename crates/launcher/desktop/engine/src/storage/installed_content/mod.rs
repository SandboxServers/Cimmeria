//! Installation identity outlives the journal's latest operation. This is not a
//! readiness receipt: consumers still need operation, content and runtime checks.
use super::*;
use crate::{catalog::VerifiedRelease, OperationKind, OperationState};

const NAME: &str = "installed-content.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    intent: InstallIntent,
}

/// Reverified ownership and signed release identity, independent of preferences
/// and the active operation. Never use this alone as permission to launch/delete.
pub struct InstalledContent {
    pub intent: InstallIntent,
    pub release: VerifiedRelease,
}

impl DesktopState {
    /// Migrates an older successful install on first access. Does not infer
    /// installation from the user-selected folder or adopt unrelated content.
    /// A damaged game file does not erase identity needed by Repair.
    pub fn installed_content(&mut self) -> Result<Option<InstalledContent>, StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        let mut record: Option<Record> = read(&self.directory.root.join(NAME))?;
        if record.is_none() {
            let successful = self
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| {
                    op.kind == OperationKind::Install && op.state == OperationState::Succeeded
                });
            if !successful {
                return Ok(None);
            }
            self.remember_prepared_content()?;
            record = read(&self.directory.root.join(NAME))?;
        }
        let record = record.ok_or(StorageError::Corrupt)?;
        if record.schema_version != 1 || record.intent.schema_version != 1 {
            return Err(StorageError::UnsupportedSchema);
        }
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| {
                op.id == record.intent.operation_id && op.state != OperationState::Succeeded
            })
        {
            return Err(StorageError::Busy);
        }
        self.verify_installed_identity(record.intent, None)
            .map(Some)
    }

    /// Called only after content validation/promotion, before committing success.
    /// A write failure keeps the operation in recovery. The reference alone is
    /// deliberately insufficient to bypass the current operation's recovery gate.
    pub(super) fn remember_prepared_content(&mut self) -> Result<(), StorageError> {
        self.remember_content(None)
    }

    /// Recovery already read this owner through its exclusively locked handle.
    /// Reopening that file to read would fail on Windows. Caller retains the lock.
    pub(super) fn remember_prepared_content_with_owner(
        &mut self,
        owner: &InstallIntent,
    ) -> Result<(), StorageError> {
        self.remember_content(Some(owner))
    }

    fn remember_content(&mut self, owner: Option<&InstallIntent>) -> Result<(), StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
        self.verify_installed_identity(intent.clone(), owner)?;
        let result = atomic::write(
            &self.directory.root,
            NAME,
            &Record {
                schema_version: 1,
                intent,
            },
        );
        if result == Err(StorageError::PersistenceUncertain) {
            self.preferences_uncertain = true;
        }
        result
    }

    pub(super) fn forget_installed_content(
        &mut self,
        intent: &InstallIntent,
    ) -> Result<(), StorageError> {
        let path = self.directory.root.join(NAME);
        if let Some(record) = read::<Record>(&path)? {
            if record.schema_version != 1 {
                return Err(StorageError::UnsupportedSchema);
            }
            if record.intent != *intent {
                return Err(StorageError::Corrupt);
            }
            std::fs::remove_file(path).map_err(|_| StorageError::Io)?;
            #[cfg(unix)]
            if File::open(&self.directory.root)
                .and_then(|f| f.sync_all())
                .is_err()
            {
                self.preferences_uncertain = true;
                return Err(StorageError::PersistenceUncertain);
            }
        }
        Ok(())
    }

    fn verify_installed_identity(
        &self,
        intent: InstallIntent,
        locked_owner: Option<&InstallIntent>,
    ) -> Result<InstalledContent, StorageError> {
        let saved: InstallIntent = read(
            &self
                .directory
                .root
                .join(format!("install-intent-{}.json", intent.operation_id)),
        )?
        .ok_or(StorageError::Corrupt)?;
        if saved != intent {
            return Err(StorageError::Corrupt);
        }
        let release = self
            .release_for_intent(&intent)
            .map_err(|error| match error {
                EvidenceError::Storage(error) => error,
                _ => StorageError::Corrupt,
            })?;
        let parent = intent
            .destination
            .parent()
            .ok_or(StorageError::InvalidDirectory)?;
        let name = intent
            .destination
            .file_name()
            .ok_or(StorageError::InvalidDirectory)?;
        if parent
            .canonicalize()
            .map_err(|_| StorageError::InvalidDirectory)?
            .join(name)
            != intent.destination
        {
            return Err(StorageError::InvalidDirectory);
        }
        ordinary_directory(&intent.destination)?;
        let game = intent.destination.join("game");
        match std::fs::symlink_metadata(&game) {
            Err(error) if error.kind() == ErrorKind::NotFound => (), // Repair needs identity even when all content is missing.
            Err(_) => return Err(StorageError::Io),
            Ok(_) => ordinary_directory(&game)?,
        }
        let owner = match locked_owner {
            Some(owner) => owner.clone(),
            None => read(&intent.destination.join(".cimmeria-install.json"))?
                .ok_or(StorageError::Corrupt)?,
        };
        let receipt: InstallIntent =
            read(&intent.destination.join("content-ready.json"))?.ok_or(StorageError::Corrupt)?;
        if owner != intent || receipt != intent {
            return Err(StorageError::Corrupt);
        }
        Ok(InstalledContent { intent, release })
    }
}

fn ordinary_directory(path: &Path) -> Result<(), StorageError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| StorageError::InvalidDirectory)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(StorageError::UnsafeFile);
        }
    }
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(StorageError::UnsafeFile);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
