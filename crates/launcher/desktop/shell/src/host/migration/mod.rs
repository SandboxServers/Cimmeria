//! Native-selected preview capability; web requests never carry source paths.
use super::NativeHost;
use cimmeria_launcher_engine::{
    migration::{LegacyImport, LegacySource, MigrationError},
    NativeCommand, NativeSnapshot, StorageError,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub imported: LegacyImport,
    pub preferences_revision: u64,
}
#[derive(Debug, Serialize)]
pub struct MigrationStatus {
    pub schema_version: u32,
    pub native: NativeSnapshot,
    pub imported: Option<LegacyImport>,
    pub preview: Option<Preview>,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum MigrationCommand {
    Inspect {
        schema_version: u32,
    },
    Dismiss {
        schema_version: u32,
    },
    Confirm {
        schema_version: u32,
        confirmation: String,
        preferences_revision: u64,
        confirmed: bool,
    },
}
impl NativeHost {
    /// Called only with folders returned by the native dialog (or isolated fixtures).
    pub fn preview_migration(
        &self,
        source: LegacySource,
    ) -> Result<MigrationStatus, MigrationError> {
        let mut pending = self
            .migration_preview
            .lock()
            .map_err(|_| StorageError::Io)?;
        *pending = None;
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        let imported = state.preview_legacy_import(&source)?;
        *pending = Some(Preview {
            imported,
            preferences_revision: state.preferences().revision,
        });
        Ok(MigrationStatus {
            schema_version: 1,
            native: state.dispatch(NativeCommand::Inspect { schema_version: 1 })?,
            imported: state.legacy_import()?,
            preview: pending.clone(),
        })
    }
    pub fn migration_command(
        &self,
        request: MigrationCommand,
    ) -> Result<MigrationStatus, MigrationError> {
        let version = match &request {
            MigrationCommand::Inspect { schema_version }
            | MigrationCommand::Dismiss { schema_version }
            | MigrationCommand::Confirm { schema_version, .. } => *schema_version,
        };
        if version != 1 {
            return Err(MigrationError::UnsupportedSchema);
        }
        let mut pending = self
            .migration_preview
            .lock()
            .map_err(|_| StorageError::Io)?;
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| StorageError::Io)?;
        match request {
            MigrationCommand::Inspect { .. } => {}
            MigrationCommand::Dismiss { .. } => *pending = None,
            MigrationCommand::Confirm {
                confirmation,
                preferences_revision,
                confirmed,
                ..
            } => {
                let preview = pending.as_ref().ok_or(MigrationError::SourceChanged)?;
                if !confirmed
                    || confirmation != preview.imported.confirmation
                    || preferences_revision != preview.preferences_revision
                {
                    return Err(MigrationError::SourceChanged);
                }
                let result = state.import_legacy(
                    &preview.imported.source,
                    &confirmation,
                    preferences_revision,
                );
                // A failed/stale confirmation requires a new native preview. Never replay a mutation.
                *pending = None;
                result?;
            }
        }
        Ok(MigrationStatus {
            schema_version: 1,
            native: state.dispatch(NativeCommand::Inspect { schema_version: 1 })?,
            imported: state.legacy_import()?,
            preview: pending.clone(),
        })
    }
}
