//! Small versioned command surface shared by Tauri and the logic-UAT harness.
use crate::{DesktopState, Preferences, Snapshot, StorageError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeCommand {
    Inspect {
        schema_version: u32,
    },
    SavePreferences {
        schema_version: u32,
        expected_revision: u64,
        install_directory: Option<PathBuf>,
        launcher_summary_consent: bool,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeSnapshot {
    pub schema_version: u32,
    pub operation: Snapshot,
    pub preferences: Preferences,
    pub requires_reopen: bool,
}

impl DesktopState {
    pub fn inspect(&self) -> NativeSnapshot {
        NativeSnapshot {
            schema_version: 1,
            operation: self.operations().snapshot().clone(),
            preferences: self.preferences().clone(),
            requires_reopen: self.requires_reopen(),
        }
    }

    pub fn dispatch(&mut self, command: NativeCommand) -> Result<NativeSnapshot, StorageError> {
        match command {
            NativeCommand::Inspect { schema_version } => {
                if schema_version != 1 {
                    return Err(StorageError::UnsupportedSchema);
                }
            }
            NativeCommand::SavePreferences {
                schema_version,
                expected_revision,
                install_directory,
                launcher_summary_consent,
            } => {
                if schema_version != 1 {
                    return Err(StorageError::UnsupportedSchema);
                }
                self.save_preferences(
                    install_directory,
                    launcher_summary_consent,
                    expected_revision,
                )?;
            }
        }
        Ok(self.inspect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_version_is_checked_before_saving() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let result = state.dispatch(NativeCommand::SavePreferences {
            schema_version: 99,
            expected_revision: 0,
            install_directory: None,
            launcher_summary_consent: true,
        });
        assert!(matches!(result, Err(StorageError::UnsupportedSchema)));
        assert_eq!(state.preferences(), &Preferences::default());
        assert!(!dir.path().join("preferences.json").exists());
    }

    #[test]
    fn bridge_has_no_native_observation_or_arbitrary_field_command() {
        for input in [
            r#"{"command":"observe","schema_version":1,"state":"succeeded"}"#,
            r#"{"command":"inspect","schema_version":1,"root":"/other"}"#,
            r#"{"command":"save_preferences","schema_version":1,"expected_revision":0,"install_directory":null}"#,
        ] {
            assert!(serde_json::from_str::<NativeCommand>(input).is_err());
        }
    }
}
