//! The player's displays and the window sizes that fit each one (C-PD8).
//!
//! Sizes are in the units the game sees: points on a Mac (Wine's `RetinaMode`
//! is off), and on Windows the monitor size divided by its scale, because
//! SGW.exe's manifest declares no DPI awareness.
use serde::Serialize;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// One active display, indexed in the platform's list order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisplayInfo {
    pub index: u32,
    pub main: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// 100, 200, 150 ...
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

/// The active displays: CoreGraphics on macOS, `EnumDisplayMonitors` on
/// Windows. Empty on failure and on other platforms.
pub fn list() -> Vec<DisplayInfo> {
    #[cfg(target_os = "macos")]
    {
        mac::list()
    }
    #[cfg(windows)]
    {
        windows::list()
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests;
