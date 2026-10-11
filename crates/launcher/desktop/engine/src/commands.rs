//! Small versioned command surface shared by Tauri and the logic-UAT harness.
use crate::launcher_compatibility::Identity;
use crate::{DesktopState, Preferences, Snapshot, StorageError};
use cimmeria_launcher_core::client_setup::login_servers::LoginServer;
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
    /// Takes effect at the next Play. Leaves the install path and consent alone.
    SaveClientOptions {
        schema_version: u32,
        expected_revision: u64,
        login_servers: Vec<LoginServer>,
        client_patches_enabled: bool,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeSnapshot {
    pub schema_version: u32,
    pub operation: Snapshot,
    pub preferences: Preferences,
    pub requires_reopen: bool,
    /// The reviewed list an adopted copy keeps at Play (`null` otherwise):
    /// saving `login_servers` does not change what that copy logs in to.
    pub adopted_login_servers: Option<Vec<LoginServer>>,
    /// The running build, for the About line in Settings.
    pub launcher: LauncherBuild,
}

/// Compiled release identity. `tag` is `null` for a development build: one
/// without a well-formed release tag and build stamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LauncherBuild {
    pub version: String,
    pub tag: Option<String>,
}
impl LauncherBuild {
    pub fn current() -> Self {
        let identity = Identity::current();
        Self {
            version: identity.desktop_version,
            tag: identity.compatibility_tag,
        }
    }
}

impl DesktopState {
    pub fn inspect(&self) -> NativeSnapshot {
        NativeSnapshot {
            schema_version: 1,
            operation: self.operations().snapshot().clone(),
            preferences: self.preferences().clone(),
            requires_reopen: self.requires_reopen(),
            adopted_login_servers: self.adopted_login_servers(),
            launcher: LauncherBuild::current(),
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
            NativeCommand::SaveClientOptions {
                schema_version,
                expected_revision,
                login_servers,
                client_patches_enabled,
            } => {
                if schema_version != 1 {
                    return Err(StorageError::UnsupportedSchema);
                }
                self.save_client_options(login_servers, client_patches_enabled, expected_revision)?;
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
    fn stale_client_options_are_refused_and_write_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let save = |expected_revision| NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision,
            login_servers: vec![LoginServer {
                name: "Local".into(),
                url: "http://login.example:8081".into(),
            }],
            client_patches_enabled: false,
        };
        assert!(matches!(
            state.dispatch(save(1)),
            Err(StorageError::StaleRevision)
        ));
        assert_eq!(state.preferences(), &Preferences::default());
        assert!(!dir.path().join("preferences.json").exists());
        let saved = state.dispatch(save(0)).unwrap().preferences;
        assert_eq!(saved.revision, 1);
        assert!(!saved.client_patches_enabled);
        let bytes = std::fs::read(dir.path().join("preferences.json")).unwrap();
        // The revision just moved on, so the same request is stale now.
        assert!(matches!(
            state.dispatch(save(0)),
            Err(StorageError::StaleRevision)
        ));
        assert_eq!(state.preferences(), &saved);
        assert_eq!(
            std::fs::read(dir.path().join("preferences.json")).unwrap(),
            bytes
        );
    }

    #[test]
    fn an_invalid_server_list_is_refused_and_writes_nothing_but_empty_means_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let save = |expected_revision, login_servers| NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision,
            login_servers,
            client_patches_enabled: true,
        };
        let server = |name: &str, url: &str| LoginServer {
            name: name.into(),
            url: url.into(),
        };
        for invalid in [
            vec![server("Local", "login.example:8081")],
            vec![
                server("Good", "http://good.example"),
                server("", "http://x"),
            ],
            vec![server("Quote\"d", "http://login.example")],
        ] {
            assert!(matches!(
                state.dispatch(save(0, invalid.clone())),
                Err(StorageError::InvalidServerList)
            ));
            assert_eq!(state.preferences(), &Preferences::default(), "{invalid:?}");
            assert!(!dir.path().join("preferences.json").exists());
        }
        let saved = state.dispatch(save(0, vec![])).unwrap().preferences;
        assert_eq!(saved.revision, 1);
        assert!(saved.login_servers.is_empty());
        // The code reaches the frontend as one flat string.
        assert_eq!(
            serde_json::to_value(StorageError::InvalidServerList).unwrap(),
            serde_json::json!("invalid_server_list")
        );
    }

    #[test]
    fn bridge_has_no_native_observation_or_arbitrary_field_command() {
        for input in [
            r#"{"command":"observe","schema_version":1,"state":"succeeded"}"#,
            r#"{"command":"inspect","schema_version":1,"root":"/other"}"#,
            r#"{"command":"save_preferences","schema_version":1,"expected_revision":0,"install_directory":null}"#,
            r#"{"command":"save_client_options","schema_version":1,"expected_revision":0,"login_servers":[],"client_patches_enabled":true,"install_directory":"/other"}"#,
        ] {
            assert!(serde_json::from_str::<NativeCommand>(input).is_err());
        }
        // The same command without the extra field is accepted, so the line
        // above fails on the unknown field and not on an unknown command.
        assert!(serde_json::from_str::<NativeCommand>(
            r#"{"command":"save_client_options","schema_version":1,"expected_revision":0,"login_servers":[],"client_patches_enabled":true}"#
        )
        .is_ok());
    }

    #[test]
    fn a_test_build_reports_the_compiled_version_and_no_release_tag() {
        // A test build carries no release tag and stamp, so it is a development
        // build, and its version is the one About and the updater use.
        assert_eq!(
            serde_json::to_value(LauncherBuild::current()).unwrap(),
            serde_json::json!({
                "version": crate::launcher_compatibility::compiled_version(),
                "tag": null,
            })
        );
    }
}
