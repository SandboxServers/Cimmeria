//! The game's display mode, owned by the launcher (C-PD6, D-PD14). The launcher
//! is the source of truth: Play writes it into the client profile before every
//! launch, so a change made in the game's own options lasts one session.
use super::{atomic, validate_preferences, DesktopState, Preferences, StorageError};
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "display_options_tests.rs"]
mod tests;

/// How the game window is shown. Borderless is macOS-only and not offered yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    Windowed,
    /// Exclusive fullscreen. Never offered on macOS (D-PD15).
    Fullscreen,
}

/// The saved display choice: a mode and the window size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayOptions {
    pub mode: DisplayMode,
    pub width: u32,
    pub height: u32,
}

impl Default for DisplayOptions {
    /// D-PD14 (owner D3): windowed 1024x768 on every platform.
    fn default() -> Self {
        Self {
            mode: DisplayMode::Windowed,
            width: 1024,
            height: 768,
        }
    }
}

/// The widths a save accepts.
pub const WIDTH: std::ops::RangeInclusive<u32> = 640..=7680;
/// The heights a save accepts.
pub const HEIGHT: std::ops::RangeInclusive<u32> = 480..=4320;

/// The modes this platform offers (D-PD15, D-PD21).
pub fn allowed_modes() -> &'static [DisplayMode] {
    if cfg!(target_os = "macos") {
        &[DisplayMode::Windowed]
    } else {
        &[DisplayMode::Windowed, DisplayMode::Fullscreen]
    }
}

impl DisplayOptions {
    /// A default display is stored as no key (R10).
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// An allowed mode and a size inside `WIDTH` and `HEIGHT`.
    pub fn valid(self) -> bool {
        allowed_modes().contains(&self.mode)
            && WIDTH.contains(&self.width)
            && HEIGHT.contains(&self.height)
    }

    /// What Play applies: on macOS always a window, and the size clamped into range.
    pub fn effective(self) -> DisplayOptions {
        let mode = if cfg!(target_os = "macos") {
            DisplayMode::Windowed
        } else {
            self.mode
        };
        DisplayOptions {
            mode,
            width: self.width.clamp(*WIDTH.start(), *WIDTH.end()),
            height: self.height.clamp(*HEIGHT.start(), *HEIGHT.end()),
        }
    }
}

impl DesktopState {
    /// The display mode and size. Takes effect at the next Play, with the same
    /// admission as `save_client_options`: it may change while the game runs.
    pub fn save_display_options(
        &mut self,
        display: DisplayOptions,
        expected_revision: u64,
    ) -> Result<Preferences, StorageError> {
        self.finalize_summaries();
        self.ensure_updater_idle()?;
        if self
            .operations
            .snapshot()
            .operation
            .as_ref()
            .is_some_and(|op| !op.state.terminal() && op.kind != crate::OperationKind::Launch)
        {
            return Err(StorageError::Busy);
        }
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        if expected_revision != self.preferences.revision {
            return Err(StorageError::StaleRevision);
        }
        if !display.valid() {
            return Err(StorageError::InvalidDisplayOptions);
        }
        let next = Preferences {
            schema_version: super::PREFERENCES_SCHEMA,
            revision: self.next_preferences_revision()?,
            display,
            ..self.preferences.clone()
        };
        validate_preferences(&next)?;
        let written = atomic::write(&self.directory.root, "preferences.json", &next);
        match written {
            Ok(()) => self.preferences = next,
            Err(error) => {
                self.preferences_uncertain = error == StorageError::PersistenceUncertain;
            }
        }
        written?;
        Ok(self.preferences.clone())
    }
}
