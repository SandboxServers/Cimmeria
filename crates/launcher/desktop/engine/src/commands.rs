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
#[path = "commands_tests.rs"]
mod tests;
