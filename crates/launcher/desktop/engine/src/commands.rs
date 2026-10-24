//! Small versioned command surface shared by Tauri and the logic-UAT harness.
use crate::display_targets::{self, DisplayInfo};
use crate::launcher_compatibility::Identity;
use crate::storage::display_options::{self, DisplayMode, DisplayOptions, FrameCap, MonitorRect};
use crate::storage::settings_events;
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
    /// Takes effect at the next Play, like the client options. `monitor` is the
    /// rectangle of one of the snapshot's `displays`, remembered for the current
    /// display setup; absent or `null` means the main display (C-PD8).
    SaveDisplayOptions {
        schema_version: u32,
        expected_revision: u64,
        display: DisplayOptions,
        #[serde(default)]
        monitor: Option<MonitorRect>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeSnapshot {
    pub schema_version: u32,
    pub operation: Snapshot,
    #[serde(serialize_with = "preferences_with_display")]
    pub preferences: Preferences,
    pub requires_reopen: bool,
    /// The reviewed list an adopted copy keeps at Play (`null` otherwise):
    /// saving `login_servers` does not change what that copy logs in to.
    pub adopted_login_servers: Option<Vec<LoginServer>>,
    /// The running build, for the About line in Settings.
    pub launcher: LauncherBuild,
    /// Whether setup diagnostics have an https endpoint to go to.
    pub setup_diagnostics_endpoint: bool,
    /// The display modes this platform offers (`display_options::allowed_modes`).
    pub display_modes: Vec<DisplayMode>,
    /// The active displays, in the units the game sees (C-PD8).
    pub displays: Vec<DisplayInfo>,
    /// Per display, the window sizes that fit it (`display_targets::sizes_for`).
    pub display_sizes: Vec<Vec<[u32; 2]>>,
    /// The `index` in `displays` the current display setup remembers
    /// (`display_options::placement_for`); `null` means the main display.
    pub display_monitor: Option<u32>,
    /// The frame caps this platform offers (`display_options::frame_caps`); empty off macOS.
    pub frame_caps: Vec<FrameCap>,
}

/// Storage omits a default `display` (R10); the snapshot always carries it, so
/// the frontend never holds the default (C-PD6). The destructure has no `..`:
/// a new `Preferences` field fails to compile here instead of going missing.
/// `display_placements` stays out: the snapshot's `display_monitor` carries
/// what the frontend needs from it.
fn preferences_with_display<S: serde::Serializer>(
    preferences: &Preferences,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let Preferences {
        schema_version,
        revision,
        install_directory,
        launcher_summary_consent,
        login_servers,
        client_patches_enabled,
        display,
        display_placements: _,
    } = preferences;
    #[derive(Serialize)]
    struct Mirror<'a> {
        schema_version: &'a u32,
        revision: &'a u64,
        install_directory: &'a Option<PathBuf>,
        launcher_summary_consent: &'a bool,
        login_servers: &'a Vec<LoginServer>,
        client_patches_enabled: &'a bool,
        display: &'a DisplayOptions,
    }
    Mirror {
        schema_version,
        revision,
        install_directory,
        launcher_summary_consent,
        login_servers,
        client_patches_enabled,
        display,
    }
    .serialize(serializer)
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
        let displays = display_targets::list(); // once per snapshot; cheap
        NativeSnapshot {
            schema_version: 1,
            operation: self.operations().snapshot().clone(),
            preferences: self.preferences().clone(),
            requires_reopen: self.requires_reopen(),
            adopted_login_servers: self.adopted_login_servers(),
            launcher: LauncherBuild::current(),
            setup_diagnostics_endpoint: self.summary_endpoint_configured(),
            display_modes: display_options::allowed_modes().to_vec(),
            display_sizes: displays.iter().map(display_targets::size_pairs).collect(),
            display_monitor: display_options::placement_for(
                &self.preferences().display_placements,
                &displays,
            )
            .map(|d| d.index),
            displays,
            frame_caps: display_options::frame_caps().to_vec(),
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
                let before = self.preferences().clone();
                self.save_preferences(
                    install_directory,
                    launcher_summary_consent,
                    expected_revision,
                )?;
                let after = self.preferences();
                settings_events::preferences_saved(
                    before.install_directory != after.install_directory,
                    after.launcher_summary_consent,
                );
                if before.launcher_summary_consent != after.launcher_summary_consent {
                    settings_events::setup_diagnostics_changed(after.launcher_summary_consent);
                }
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
                let after = self.preferences();
                settings_events::client_options_saved(
                    &after.login_servers,
                    after.client_patches_enabled,
                );
            }
            NativeCommand::SaveDisplayOptions {
                schema_version,
                expected_revision,
                display,
                monitor,
            } => {
                if schema_version != 1 {
                    return Err(StorageError::UnsupportedSchema);
                }
                let displays = display_targets::list(); // once per save (PD-31 ruling 1)
                self.save_display(display, monitor, expected_revision, &displays)?;
            }
        }
        Ok(self.inspect())
    }

    /// The `save_display_options` arm against one display list, so a test can
    /// pass the list a sleeping Mac or a failed enumeration returns.
    fn save_display(
        &mut self,
        display: DisplayOptions,
        monitor: Option<MonitorRect>,
        expected_revision: u64,
        displays: &[DisplayInfo],
    ) -> Result<(), StorageError> {
        let (saved, remembered) =
            self.save_display_options_on(display, monitor, expected_revision, displays)?;
        settings_events::display_options_saved(saved.display, remembered);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_capture::capture;
    use std::sync::{Mutex, MutexGuard};

    static SERIAL: Mutex<()> = Mutex::new(());
    fn serial() -> MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn saving_client_options_reports_a_count_not_the_servers() {
        let _serial = serial(); // first line: one capture test of this module at a time
                                // A second live Dispatch keeps callsite interest per dispatcher.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let server = |name: &str, url: &str| LoginServer {
            name: name.into(),
            url: url.into(),
        };
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache(); // first line in the closure
            state.dispatch(NativeCommand::SaveClientOptions {
                schema_version: 1,
                expected_revision: 0,
                login_servers: vec![
                    server("A", "https://a.example"),
                    server("B", "https://b.example"),
                ],
                client_patches_enabled: false,
            })
        });
        assert!(result.is_ok());
        let saved: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("client_options_saved"))
            .collect();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].level, tracing::Level::INFO);
        assert_eq!(saved[0].target, "launcher.settings");
        assert_eq!(saved[0].field("count"), Some("2"));
        assert_eq!(saved[0].field("custom_servers"), Some("true"));
        assert_eq!(saved[0].field("client_patches_enabled"), Some("false"));
        for event in &events {
            for value in event.fields.values() {
                assert!(!value.contains("a.example") && !value.contains("b.example"));
            }
        }
    }

    #[test]
    fn saving_display_options_logs_the_mode_and_size() {
        let _serial = serial(); // first line: one capture test of this module at a time
                                // A second live Dispatch keeps callsite interest per dispatcher.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let command: NativeCommand = serde_json::from_str(
            r#"{"command":"save_display_options","schema_version":1,"expected_revision":0,"display":{"mode":"windowed","width":1600,"height":900}}"#,
        )
        .unwrap();
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache(); // first line in the closure
            state.dispatch(command)
        });
        let snapshot = result.unwrap();
        assert_eq!(snapshot.preferences.display.width, 1600);
        assert_eq!(snapshot.display_modes, display_options::allowed_modes());
        let saved: Vec<_> = events
            .iter()
            .filter(|e| e.field("event") == Some("display_options_saved"))
            .collect();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].level, tracing::Level::INFO);
        assert_eq!(saved[0].target, "launcher.settings");
        assert_eq!(saved[0].field("mode"), Some("windowed"));
        assert_eq!(saved[0].field("width"), Some("1600"));
        assert_eq!(saved[0].field("height"), Some("900"));
        assert_eq!(saved[0].field("monitor_remembered"), Some("false"));
        assert_eq!(saved[0].field("frame_cap"), Some("fps30")); // no frame_cap sent: the default
        assert_eq!(snapshot.frame_caps, display_options::frame_caps());
    }

    #[test]
    fn a_monitor_saved_while_no_display_is_listed_is_not_remembered() {
        let _serial = serial(); // first line: one capture test of this module at a time
                                // A second live Dispatch keeps callsite interest per dispatcher.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let display = DisplayOptions::default();
        let side = DisplayInfo {
            index: 1,
            main: false,
            x: 1710,
            y: 0,
            width: 1920,
            height: 1080,
            scale_percent: 100,
            refresh_hz: 60,
        };
        let main = DisplayInfo {
            index: 0,
            main: true,
            x: 0,
            y: 0,
            width: 1710,
            height: 1107,
            scale_percent: 200,
            refresh_hz: 120,
        };
        let monitor = Some(MonitorRect {
            x: 1710,
            y: 0,
            width: 1920,
            height: 1080,
        });
        let remembered = |events: &[crate::test_capture::Captured]| {
            let saved: Vec<_> = events
                .iter()
                .filter(|e| e.field("event") == Some("display_options_saved"))
                .collect();
            assert_eq!(saved.len(), 1);
            saved[0].field("monitor_remembered").map(str::to_owned)
        };
        let (result, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache(); // first line in the closure
            state.save_display(display, monitor, 0, &[main, side])
        });
        result.unwrap();
        assert_eq!(remembered(&events).as_deref(), Some("true"));
        let before = state.preferences().display_placements.clone();
        assert_eq!(before.len(), 1);
        // An empty list (a sleeping Mac, a failed enumeration) stores, replaces
        // and removes nothing, whether a monitor is given or not.
        for (revision, monitor) in [(1, monitor), (2, None)] {
            let (result, events) = capture(|| {
                tracing::callsite::rebuild_interest_cache(); // first line in the closure
                state.save_display(display, monitor, revision, &[])
            });
            result.unwrap();
            assert_eq!(remembered(&events).as_deref(), Some("false"));
            assert_eq!(state.preferences().display_placements, before);
        }
    }

    #[test]
    fn saving_preferences_reports_the_consent_change_only_when_it_changes() {
        let _serial = serial();
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let save = |expected_revision, consent| NativeCommand::SavePreferences {
            schema_version: 1,
            expected_revision,
            install_directory: None,
            launcher_summary_consent: consent,
        };
        let named = |events: &[crate::test_capture::Captured], name: &str| {
            events
                .iter()
                .filter(|e| e.field("event") == Some(name))
                .count()
        };
        let (stale, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(7, true))
        });
        assert!(matches!(stale, Err(StorageError::StaleRevision)));
        assert!(events.iter().all(|e| e.target != "launcher.settings"));
        let (first, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(0, true))
        });
        assert!(first.is_ok());
        assert_eq!(named(&events, "preferences_saved"), 1);
        assert_eq!(named(&events, "setup_diagnostics_changed"), 1);
        let (second, events) = capture(|| {
            tracing::callsite::rebuild_interest_cache();
            state.dispatch(save(1, true))
        });
        assert!(second.is_ok());
        assert_eq!(named(&events, "preferences_saved"), 1);
        assert_eq!(named(&events, "setup_diagnostics_changed"), 0);
    }

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

    #[test]
    fn the_snapshot_always_carries_the_display_though_storage_omits_a_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = DesktopState::open(dir.path()).unwrap();
        let shown = |state: &DesktopState| {
            serde_json::to_value(state.inspect()).unwrap()["preferences"]["display"].clone()
        };
        assert_eq!(
            shown(&state),
            serde_json::json!({"mode":"windowed","width":1024,"height":768})
        );
        state
            .dispatch(NativeCommand::SaveDisplayOptions {
                schema_version: 1,
                expected_revision: 0,
                display: DisplayOptions {
                    mode: DisplayMode::Windowed,
                    width: 1600,
                    height: 900,
                    ..DisplayOptions::default()
                },
                monitor: None,
            })
            .unwrap();
        assert_eq!(
            shown(&state),
            serde_json::json!({"mode":"windowed","width":1600,"height":900})
        );
    }
}
