//! Explicit removal of one stopped attempt's partial files, never a promoted game.
use super::*;
use crate::{OperationKind, OperationState};
use uuid::Uuid;
impl DesktopState {
    pub fn can_retry_install(&self) -> bool {
        !self.requires_reopen()
            && self
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| {
                    (op.kind == OperationKind::Install
                        && matches!(op.state, OperationState::Failed | OperationState::Cancelled))
                        || (op.kind == OperationKind::Uninstall
                            && op.state == OperationState::Succeeded)
                })
            && self
                .preferences
                .install_directory
                .as_ref()
                .is_some_and(|path| {
                    install_intent::fresh_destination(path, &self.directory.root).is_ok()
                })
    }

    /// Caller must obtain explicit confirmation. IDs/revisions bind that action
    /// to the inspected attempt. Repeating after a lost reply is safe only after
    /// inspecting state again; this method never admits a replacement install.
    pub fn clean_failed_install(&mut self, id: Uuid, revision: u64) -> Result<(), IntentError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let snapshot = self.operations.snapshot();
        if snapshot.revision != revision {
            return Err(ContractError::StaleRevision.into());
        }
        let op = snapshot
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if op.id != id {
            return Err(ContractError::UnknownOperation.into());
        }
        if op.kind != OperationKind::Install
            || !matches!(op.state, OperationState::Failed | OperationState::Cancelled)
        {
            return Err(ContractError::InvalidTransition.into());
        }
        let intent = self.install_intent()?.ok_or(StorageError::Corrupt)?;
        #[cfg(target_os = "macos")]
        let _stopped = if !intent.backend.is_native() {
            Some(crate::mac_wine::recovery::stop_for_recovery(self)?)
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        if !intent.backend.is_native() {
            return Err(ContractError::InvalidTransition.into());
        }
        let destination = &intent.destination;
        let parent = destination.parent().ok_or(StorageError::InvalidDirectory)?;
        if parent
            .canonicalize()
            .map_err(|_| StorageError::InvalidDirectory)?
            != parent
        {
            return Err(StorageError::InvalidDirectory.into());
        }
        match std::fs::symlink_metadata(destination) {
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
            Ok(m) if plain(&m) && m.is_dir() => (),
            _ => return Err(StorageError::UnsafeFile.into()),
        }
        let stage = format!(".cimmeria-stage-{id}");
        let cache = format!(".cimmeria-cache-{id}");
        let entries = std::fs::read_dir(destination)
            .map_err(|_| StorageError::Io)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StorageError::Io)?;
        if entries.is_empty() {
            return Ok(());
        }
        // Check every top-level name/type before the first deletion. A game,
        // completion receipt, foreign attempt or user file vetoes cleanup.
        for entry in &entries {
            let name = entry.file_name();
            let name = name.to_str().ok_or(StorageError::UnsafeFile)?;
            let meta = std::fs::symlink_metadata(entry.path()).map_err(|_| StorageError::Io)?;
            if !plain(&meta)
                || !((name == stage || name == cache) && meta.is_dir()
                    || name == ".cimmeria-install.json" && meta.is_file())
            {
                return Err(StorageError::UnsafeFile.into());
            }
        }
        let marker = destination.join(".cimmeria-install.json");
        let guard = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&marker)
            .map_err(|_| StorageError::Corrupt)?;
        guard.try_lock().map_err(|_| StorageError::InUse)?;
        let owner: InstallIntent = read_open(&guard)?;
        if owner != intent {
            return Err(StorageError::Corrupt.into());
        }
        for name in [&stage, &cache] {
            let path = destination.join(name);
            if path.exists() {
                validate_tree(&path)?;
            }
        }
        for name in [&stage, &cache] {
            let path = destination.join(name);
            match std::fs::remove_dir_all(path) {
                Ok(()) => (),
                Err(e) if e.kind() == ErrorKind::NotFound => (),
                Err(_) => return Err(StorageError::Io.into()),
            }
        }
        // Windows cannot unlink this locked file. No content writer remains;
        // DesktopState still serializes admission until the command finishes.
        drop(guard);
        std::fs::remove_file(marker).map_err(|_| StorageError::Io)?;
        #[cfg(unix)]
        File::open(destination)
            .and_then(|file| file.sync_all())
            .map_err(|_| StorageError::Io)?;
        Ok(())
    }
}
pub(super) fn plain(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        } // FILE_ATTRIBUTE_REPARSE_POINT
    }
    meta.is_dir() || meta.is_file()
}
pub(super) fn validate_tree(root: &Path) -> Result<(), StorageError> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).map_err(|_| StorageError::Io)? {
            let path = entry.map_err(|_| StorageError::Io)?.path();
            let meta = std::fs::symlink_metadata(&path).map_err(|_| StorageError::Io)?;
            if !plain(&meta) {
                return Err(StorageError::UnsafeFile);
            }
            if meta.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests;
