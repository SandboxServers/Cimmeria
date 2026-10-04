//! Explicit, bounded legacy import. The imported ledger grants no content ownership.
mod model;
mod source;
#[cfg(test)]
mod tests;
use super::{atomic, read, DesktopState, Preferences, StorageError};
pub use model::*;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

const NAME: &str = "legacy-import.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    imported: LegacyImport,
    sources: source::Sources,
    before: Preferences,
    after: Preferences,
    complete: bool,
}
impl Record {
    fn validate(&self) -> Result<(), MigrationError> {
        if self.schema_version != 1 {
            return Err(MigrationError::UnsupportedSchema);
        }
        if self.sources.parse(self.imported.source.clone())? != self.imported
            || self.after.revision
                != self
                    .before
                    .revision
                    .checked_add(1)
                    .ok_or(StorageError::Corrupt)?
            || self.after.install_directory.as_ref() != Some(&self.imported.source.game_directory)
            || self.after.launcher_summary_consent != self.before.launcher_summary_consent
        {
            return Err(StorageError::Corrupt.into());
        }
        super::validate_preferences(&self.before)?;
        super::validate_preferences(&self.after)?;
        Ok(())
    }
}
impl DesktopState {
    /// Preview performs no import or source rewrite. It may create the legacy lock file.
    /// Paths must come from a native folder chooser, not an untrusted web caller.
    pub fn preview_legacy_import(
        &self,
        source: &LegacySource,
    ) -> Result<LegacyImport, MigrationError> {
        self.migration_idle()?;
        let source = source::canonical(source)?;
        if source.launcher_directory == self.directory.root {
            return Err(MigrationError::Conflict);
        }
        let _legacy_lock = source::lock(&source)?;
        source::load(&source)?.parse(source)
    }

    /// The shell must display the preview and require explicit confirmation before
    /// passing its digest back. Source edits after preview invalidate confirmation.
    pub fn import_legacy(
        &mut self,
        source: &LegacySource,
        confirmation: &str,
        expected_preferences_revision: u64,
    ) -> Result<LegacyImport, MigrationError> {
        self.import_legacy_with(
            source,
            confirmation,
            expected_preferences_revision,
            atomic::write,
        )
    }
    fn import_legacy_with(
        &mut self,
        source: &LegacySource,
        confirmation: &str,
        expected_revision: u64,
        write: impl FnOnce(&Path, &str, &Record) -> Result<(), StorageError>,
    ) -> Result<LegacyImport, MigrationError> {
        self.migration_idle()?;
        let source = source::canonical(source)?;
        if source.launcher_directory == self.directory.root {
            return Err(MigrationError::Conflict);
        }
        let _legacy_lock = source::lock(&source)?;
        let sources = source::load(&source)?;
        let imported = sources.parse(source)?;
        if imported.confirmation != confirmation {
            return Err(MigrationError::SourceChanged);
        }
        if let Some(record) = self.migration_record()? {
            if record.imported != imported {
                return Err(MigrationError::Conflict);
            }
            self.finish_migration(record)?;
            return Ok(imported);
        }
        if self.preferences.revision != expected_revision {
            return Err(StorageError::StaleRevision.into());
        }
        // Existing desktop content and operation history are never replaced by legacy claims.
        for entry in std::fs::read_dir(&self.directory.root).map_err(|_| StorageError::Io)? {
            let name = entry.map_err(|_| StorageError::Io)?.file_name();
            let name = name.to_string_lossy();
            if name == "installed-content.json"
                || (name.starts_with("install-intent-") && name.ends_with(".json"))
            {
                return Err(MigrationError::Conflict);
            }
        }
        if self.operations.snapshot().operation.is_some() {
            return Err(MigrationError::Conflict);
        }
        let mut after = self.preferences.clone();
        after.revision = after.revision.checked_add(1).ok_or(StorageError::Corrupt)?;
        after.install_directory = Some(imported.source.game_directory.clone());
        // Game opt-in and launcher summaries are independent; never change the latter.
        let record = Record {
            schema_version: 1,
            imported: imported.clone(),
            sources,
            before: self.preferences.clone(),
            after,
            complete: false,
        };
        record.validate()?;
        self.migration_write_result(write(&self.directory.root, NAME, &record))?;
        // Once intent is durable, any later failure requires reopening before mutation.
        if let Err(error) = self.finish_migration(record) {
            self.preferences_uncertain = true;
            return Err(error);
        }
        Ok(imported)
    }
    /// Loaded configuration/identity and historical patch claims; no readiness receipt.
    pub fn legacy_import(&self) -> Result<Option<LegacyImport>, MigrationError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let record = self.migration_record()?;
        if record.as_ref().is_some_and(|record| !record.complete) {
            return Err(StorageError::PersistenceUncertain.into());
        }
        Ok(record.map(|record| record.imported))
    }
    /// Startup recovery completes only the exact saved preference transition.
    /// Called from DesktopState::open before publishing state.
    pub(super) fn recover_legacy_import(&mut self) -> Result<(), StorageError> {
        let result = (|| {
            if let Some(record) = self.migration_record()? {
                if !record.complete {
                    let _legacy_lock = source::lock(&record.imported.source)?;
                    self.finish_migration(record)?;
                }
            }
            Ok(())
        })();
        result.map_err(|e| match e {
            MigrationError::Storage(e) => e,
            MigrationError::Busy => StorageError::InUse,
            MigrationError::UnsupportedSchema => StorageError::UnsupportedSchema,
            _ => StorageError::Corrupt,
        })
    }
    fn migration_record(&self) -> Result<Option<Record>, MigrationError> {
        let record: Option<Record> = read(&self.directory.root.join(NAME))?;
        if let Some(record) = &record {
            record.validate()?;
        }
        Ok(record)
    }
    fn finish_migration(&mut self, mut record: Record) -> Result<(), MigrationError> {
        if record.complete {
            return Ok(());
        }
        if self.preferences == record.before {
            self.migration_write_result(atomic::write(
                &self.directory.root,
                "preferences.json",
                &record.after,
            ))?;
            self.preferences = record.after.clone();
        } else if self.preferences != record.after {
            return Err(MigrationError::Conflict);
        }
        record.complete = true;
        self.migration_write_result(atomic::write(&self.directory.root, NAME, &record))
    }
    fn migration_write_result(
        &mut self,
        result: Result<(), StorageError>,
    ) -> Result<(), MigrationError> {
        if result == Err(StorageError::PersistenceUncertain) {
            self.preferences_uncertain = true;
        }
        result.map_err(Into::into)
    }
    fn migration_idle(&self) -> Result<(), MigrationError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal())
        {
            return Err(MigrationError::Busy);
        }
        Ok(())
    }
}
