//! Previously authenticated managed outputs, never a scan of player-owned files.
use super::*;
use std::collections::BTreeSet;

impl DesktopState {
    pub(crate) fn direct_reconstruction_files(
        &self,
        intent: &InstallIntent,
    ) -> Result<Vec<PathBuf>, StorageError> {
        let mut selected: BTreeSet<PathBuf> =
            self.direct_adopted_files(intent)?.into_iter().collect();
        for entry in std::fs::read_dir(&self.directory.root).map_err(|_| StorageError::Io)? {
            let entry = entry.map_err(|_| StorageError::Io)?;
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            if filename.starts_with("repair-plan-") {
                let saved: super::super::Plan =
                    read(&entry.path())?.ok_or(StorageError::Corrupt)?;
                if saved.installation != *intent {
                    continue;
                }
                if let Some(committed) = read::<super::super::commit::Record>(
                    &self
                        .directory
                        .root
                        .join(format!("repair-commit-{}.json", saved.id)),
                )? {
                    if committed.schema_version == 2
                        && committed.plan == saved
                        && committed.phase == super::super::commit::Phase::Published
                    {
                        self.verify_release_identity(saved.release_identity())
                            .map_err(|_| StorageError::Corrupt)?;
                        selected.extend(paths(&saved, &saved.work_directory())?);
                    }
                }
            } else if filename.starts_with("update-plan-") {
                let saved: crate::storage::update::Plan =
                    read(&entry.path())?.ok_or(StorageError::Corrupt)?;
                if saved.owner != *intent {
                    continue;
                }
                if let Some(committed) = read::<crate::storage::update::commit::Record>(
                    &self
                        .directory
                        .root
                        .join(format!("update-commit-{}.json", saved.id)),
                )? {
                    if committed.schema_version == 2
                        && committed.plan == saved
                        && committed.phase == crate::storage::update::commit::Phase::Published
                    {
                        self.verify_release_identity(saved.target)
                            .map_err(|_| StorageError::Corrupt)?;
                        selected.extend(paths(&saved, &saved.work_directory())?);
                    }
                }
            }
        }
        for relative in &selected {
            validate_relative(relative)?;
        }
        Ok(selected.into_iter().collect())
    }
}
