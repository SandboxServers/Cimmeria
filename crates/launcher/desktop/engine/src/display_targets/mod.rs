//! The player's displays and the window sizes that fit each one (C-PD8).
//!
//! Positions and sizes are in the units the game sees: points on a Mac (Wine's
//! `RetinaMode` is off), and on Windows the monitor's pixels divided by its
//! effective DPI scale (`GetDpiForMonitor`), because SGW.exe's manifest declares
//! no DPI awareness and Windows scales its window by that factor.
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// One active display, indexed in the platform's list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisplayInfo {
    /// Position in the platform's display list, from 0.
    pub index: u32,
    /// The main display (macOS) or the primary monitor (Windows).
    pub main: bool,
    /// Origin in the desktop's global coordinates, in the units the game sees.
    pub x: i32,
    /// Origin in the desktop's global coordinates, in the units the game sees.
    pub y: i32,
    /// In the units the game sees: points on a Mac, pixels divided by the monitor's scale on Windows.
    pub width: u32,
    /// In the units the game sees: points on a Mac, pixels divided by the monitor's scale on Windows.
    pub height: u32,
    /// Pixels per game unit, in percent: 100, 125, 150, 200 ... (Windows: effective DPI / 96).
    pub scale_percent: u32,
    /// 0 = unknown.
    pub refresh_hz: u32,
}

/// The window sizes the launcher offers, smallest first.
pub const SIZES: &[(u32, u32)] = &[
    (1024, 768),
    (1280, 720),
    (1280, 800),
    (1440, 900),
    (1600, 900),
    (1680, 1050),
    (1920, 1080),
    (2560, 1440),
];

/// The menu bar or taskbar plus the window's title bar.
pub const FRAME_MARGIN: u32 = 80;

/// The sizes whose window, title bar and menu bar or taskbar included, fits
/// on `d`. Never empty: 1024x768 stays even on a smaller display.
pub fn sizes_for(d: &DisplayInfo) -> Vec<(u32, u32)> {
    let fits: Vec<(u32, u32)> = SIZES
        .iter()
        .copied()
        .filter(|&(w, h)| w <= d.width && h.saturating_add(FRAME_MARGIN) <= d.height)
        .collect();
    if fits.is_empty() {
        vec![SIZES[0]]
    } else {
        fits
    }
}

/// `sizes_for` as `[width, height]` pairs, the snapshot's wire shape.
pub fn size_pairs(d: &DisplayInfo) -> Vec<[u32; 2]> {
    sizes_for(d).into_iter().map(|(w, h)| [w, h]).collect()
}

/// Identifies a display arrangement: 16 lowercase hex digits of FNV-1a 64 over
/// each display's `(x, y, width, height)`, sorted, so list order does not
/// matter. It carries no display names.
/// An empty list gives the bare FNV offset basis (`cbf29ce484222325`), the same key for every failed enumeration, so callers must not store anything under it.
pub fn setup_key(ds: &[DisplayInfo]) -> String {
    let mut rects: Vec<(i32, i32, u32, u32)> =
        ds.iter().map(|d| (d.x, d.y, d.width, d.height)).collect();
    rects.sort_unstable();
    let mut bytes = Vec::with_capacity(rects.len() * 16);
    for (x, y, w, h) in rects {
        bytes.extend_from_slice(&x.to_le_bytes());
        bytes.extend_from_slice(&y.to_le_bytes());
        bytes.extend_from_slice(&w.to_le_bytes());
        bytes.extend_from_slice(&h.to_le_bytes());
    }
    format!("{:016x}", fnv1a64(&bytes))
}

/// FNV-1a 64, by hand: `DefaultHasher` is not stable across Rust releases.
fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes
        .iter()
        .fold(OFFSET, |hash, &b| (hash ^ u64::from(b)).wrapping_mul(PRIME))
}

/// Windows' effective DPI as a percent scale (96 dpi is 100). 0, meaning the
/// query failed, gives 100.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn scale_percent_from_dpi(dpi: u32) -> u32 {
    if dpi == 0 {
        100
    } else {
        (dpi * 100 + 48) / 96
    }
}

/// Pixels to the units a DPI-unaware game sees, rounded to the nearest unit
/// (halves away from zero).
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn to_logical(px: i32, scale_percent: u32) -> i32 {
    let s = i64::from(scale_percent.max(1));
    let v = i64::from(px) * 100;
    let r = if v >= 0 {
        (v + s / 2) / s
    } else {
        (v - s / 2) / s
    };
    r as i32
}

/// A monitor rectangle in pixels (`left, top, right, bottom`) as the
/// `(x, y, width, height)` a DPI-unaware game sees at `scale_percent`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn logical_rect(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    scale_percent: u32,
) -> (i32, i32, u32, u32) {
    (
        to_logical(left, scale_percent),
        to_logical(top, scale_percent),
        to_logical(right.saturating_sub(left), scale_percent).max(0) as u32,
        to_logical(bottom.saturating_sub(top), scale_percent).max(0) as u32,
    )
}

/// Set while `list()` keeps coming back empty, so the WARN is logged once per run.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
static EMPTY_LOGGED: AtomicBool = AtomicBool::new(false);

/// True for the first empty list after start or after a non-empty one. A
/// non-empty list resets the latch.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
fn first_empty(latch: &AtomicBool, empty: bool) -> bool {
    if empty {
        !latch.swap(true, Ordering::Relaxed)
    } else {
        latch.store(false, Ordering::Relaxed);
        false
    }
}

/// The active displays: CoreGraphics on macOS, `EnumDisplayMonitors` on
/// Windows. Empty on failure and on other platforms.
/// The first empty result in a run logs one WARN `display_list_empty`.
pub fn list() -> Vec<DisplayInfo> {
    #[cfg(any(target_os = "macos", windows))]
    {
        #[cfg(target_os = "macos")]
        let displays = mac::list();
        #[cfg(windows)]
        let displays = windows::list();
        if first_empty(&EMPTY_LOGGED, displays.is_empty()) {
            tracing::warn!(
                target: "launcher.settings",
                event = "display_list_empty",
                "displays: the platform listed no active display; the display picker stays empty"
            );
        }
        displays
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
