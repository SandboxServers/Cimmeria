//! The pure half of window placement (PD-33): which monitor a remembered
//! rectangle names, where a window sits centred on it, and the unit rule that
//! makes the helper's monitor rectangles comparable with the engine's.
//!
//! Rectangles are `[x, y, width, height]`. The engine sends `place_on` in the
//! units the game sees (PD-31): on Windows, a monitor's pixels divided by its
//! effective DPI scale. The helper converts each monitor with the same rule
//! before `choose`; the work area and the window position stay in its own
//! units. Under Wine the DPI reads 96, so the conversion is the identity.
use serde::{Deserialize, Serialize};

/// How placing the game window ended. A closed set, reported once per launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementOutcome {
    /// The window was moved to the chosen monitor.
    Placed,
    /// The window opened on the chosen monitor; nothing was moved.
    AlreadyThere,
    /// No visible window of the game appeared within the wait.
    NoWindow,
    /// No connected monitor matches the remembered rectangle.
    NoMonitor,
}

impl PlacementOutcome {
    /// The serde name, for logs.
    pub fn code(self) -> &'static str {
        match self {
            Self::Placed => "placed",
            Self::AlreadyThere => "already_there",
            Self::NoWindow => "no_window",
            Self::NoMonitor => "no_monitor",
        }
    }
}

/// The monitor `target` names: the one with exactly that rectangle, else the
/// first with the same size (the setup moved, or the units differ slightly),
/// else `None`.
pub fn choose(monitors: &[[i32; 4]], target: [i32; 4]) -> Option<usize> {
    monitors.iter().position(|m| *m == target).or_else(|| {
        monitors
            .iter()
            .position(|m| m[2] == target[2] && m[3] == target[3])
    })
}

/// The top-left corner that centres `window` in `work`, kept inside it. A
/// window larger than the work area is pinned to the work area's top-left.
pub fn centre(work: [i32; 4], window: [i32; 4]) -> (i32, i32) {
    let offset = |space: i32, size: i32| (space.saturating_sub(size) / 2).max(0);
    (
        work[0].saturating_add(offset(work[2], window[2])),
        work[1].saturating_add(offset(work[3], window[3])),
    )
}

/// Windows' effective DPI as a percent scale (96 dpi is 100). 0, meaning the
/// query failed, gives 100. The same rule as the engine's `display_targets`.
pub fn scale_percent_from_dpi(dpi: u32) -> u32 {
    if dpi == 0 {
        100
    } else {
        dpi.saturating_mul(100).saturating_add(48) / 96
    }
}

/// Pixels to the units a DPI-unaware game sees, rounded to the nearest unit
/// (halves away from zero).
pub fn to_logical(px: i32, scale_percent: u32) -> i32 {
    let s = i64::from(scale_percent.max(1));
    let v = i64::from(px) * 100;
    let r = if v >= 0 {
        (v + s / 2) / s
    } else {
        (v - s / 2) / s
    };
    r as i32
}

/// A monitor rectangle in the helper's units as the game units `place_on` uses:
/// each origin and each extent converted on its own.
pub fn game_units(rect: [i32; 4], scale_percent: u32) -> [i32; 4] {
    rect.map(|v| to_logical(v, scale_percent))
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
