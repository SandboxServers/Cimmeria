//! The game's display mode, owned by the launcher (C-PD6, D-PD14). The launcher
//! is the source of truth: Play writes it into the client profile before every
//! launch, so a change made in the game's own options lasts one session.
use super::{atomic, validate_preferences, DesktopState, Preferences, StorageError};
use crate::display_targets::{self, DisplayInfo};
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

/// The most placements `Preferences::display_placements` keeps (C-PD8).
pub const MAX_PLACEMENTS: usize = 8;

/// The monitor the game opens on for one display setup (C-PD8): the setup's
/// `display_targets::setup_key` and that monitor's rectangle, in the units the
/// game sees (`DisplayInfo`'s `x, y, width, height`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub setup: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The monitor a `save_display_options` command picks, as the rectangle of one
/// of the snapshot's `displays`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The rectangle `setup_key` hashes for one display.
fn rect_of(d: &DisplayInfo) -> (i32, i32, u32, u32) {
    (d.x, d.y, d.width, d.height)
}

/// Records `monitor` for `setup`: replaces that setup's entry (the replacement
/// counts as the newest), removes it on `None` ("main display"), and keeps the
/// newest `MAX_PLACEMENTS`, dropping the oldest.
pub(crate) fn remember(list: &mut Vec<Placement>, setup: &str, monitor: Option<MonitorRect>) {
    list.retain(|p| p.setup != setup);
    if let Some(m) = monitor {
        list.push(Placement {
            setup: setup.to_owned(),
            x: m.x,
            y: m.y,
            width: m.width,
            height: m.height,
        });
    }
    let excess = list.len().saturating_sub(MAX_PLACEMENTS);
    list.drain(..excess);
}

/// The display the current setup remembers: the entry for `setup_key(displays)`,
/// then the display whose `(x, y, width, height)` equals the stored rectangle.
/// `None` (no entry, no matching display, or no displays) means the main display.
pub(crate) fn placement_for(
    placements: &[Placement],
    displays: &[DisplayInfo],
) -> Option<DisplayInfo> {
    if displays.is_empty() {
        return None;
    }
    let key = display_targets::setup_key(displays);
    let stored = placements.iter().find(|p| p.setup == key)?;
    let want = (stored.x, stored.y, stored.width, stored.height);
    displays.iter().find(|d| rect_of(d) == want).copied()
}

impl DesktopState {
    /// The display mode and size, and the monitor for the current display setup
    /// (`None` means the main display). Takes effect at the next Play, with the
    /// same admission as `save_client_options`: it may change while the game runs.
    pub fn save_display_options(
        &mut self,
        display: DisplayOptions,
        monitor: Option<MonitorRect>,
        expected_revision: u64,
    ) -> Result<Preferences, StorageError> {
        let displays = display_targets::list();
        self.save_display_options_on(display, monitor, expected_revision, &displays)
            .map(|(preferences, _)| preferences)
    }

    /// `save_display_options` against one `display_targets::list()` result, which
    /// gives both the setup key and the rectangle a monitor must match. Also
    /// returns whether the current setup now remembers a monitor. An empty list
    /// shares one key with every failed enumeration, so the placements are then
    /// left exactly as they were (PD-31 ruling 1, amendment 1a); so is a
    /// `monitor` that matches no listed display.
    pub(crate) fn save_display_options_on(
        &mut self,
        display: DisplayOptions,
        monitor: Option<MonitorRect>,
        expected_revision: u64,
        displays: &[DisplayInfo],
    ) -> Result<(Preferences, bool), StorageError> {
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
        let mut display_placements = self.preferences.display_placements.clone();
        let mut remembered = false;
        if !displays.is_empty() {
            let listed = monitor.map(|m| {
                displays
                    .iter()
                    .any(|d| rect_of(d) == (m.x, m.y, m.width, m.height))
            });
            if listed != Some(false) {
                remember(
                    &mut display_placements,
                    &display_targets::setup_key(displays),
                    monitor,
                );
                remembered = monitor.is_some();
            }
        }
        let next = Preferences {
            schema_version: super::PREFERENCES_SCHEMA,
            revision: self.next_preferences_revision()?,
            display,
            display_placements,
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
        Ok((self.preferences.clone(), remembered))
    }
}
