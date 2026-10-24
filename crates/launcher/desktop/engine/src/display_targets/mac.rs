//! macOS displays through CoreGraphics, by raw FFI (no new crate).
//!
//! `CGDisplayBounds` is in points, which is what the game sees through Wine
//! with `RetinaMode` off.
use super::DisplayInfo;
use std::ffi::c_void;

type CGDirectDisplayID = u32;
type CGError = i32;
type CGDisplayModeRef = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGGetActiveDisplayList(
        max_displays: u32,
        active_displays: *mut CGDirectDisplayID,
        display_count: *mut u32,
    ) -> CGError;
    fn CGDisplayBounds(display: CGDirectDisplayID) -> CGRect;
    fn CGMainDisplayID() -> CGDirectDisplayID;
    #[cfg(test)]
    fn CGDisplayIsAsleep(display: CGDirectDisplayID) -> u32;
    fn CGDisplayCopyDisplayMode(display: CGDirectDisplayID) -> CGDisplayModeRef;
    fn CGDisplayModeGetPixelWidth(mode: CGDisplayModeRef) -> usize;
    fn CGDisplayModeGetRefreshRate(mode: CGDisplayModeRef) -> f64;
    fn CGDisplayModeRelease(mode: CGDisplayModeRef);
}

/// More displays than anyone connects; bounds the buffer.
const MAX_DISPLAYS: u32 = 32;

/// Whether the main display is asleep (the active list is empty then).
#[cfg(test)]
pub(super) fn main_display_asleep() -> bool {
    // SAFETY: plain value calls.
    unsafe { CGDisplayIsAsleep(CGMainDisplayID()) != 0 }
}

pub(super) fn list() -> Vec<DisplayInfo> {
    let mut ids = [0 as CGDirectDisplayID; MAX_DISPLAYS as usize];
    let mut found = 0u32;
    // SAFETY: the buffer holds MAX_DISPLAYS ids and CoreGraphics writes at most that many.
    let status = unsafe { CGGetActiveDisplayList(MAX_DISPLAYS, ids.as_mut_ptr(), &mut found) };
    if status != 0 {
        return Vec::new();
    }
    // SAFETY: no arguments; returns an id.
    let main = unsafe { CGMainDisplayID() };
    ids.iter()
        .take(found.min(MAX_DISPLAYS) as usize)
        .enumerate()
        .map(|(index, &id)| describe(index as u32, id, id == main))
        .collect()
}

fn describe(index: u32, id: CGDirectDisplayID, main: bool) -> DisplayInfo {
    // SAFETY: a plain value call on an id CoreGraphics just listed.
    let bounds = unsafe { CGDisplayBounds(id) };
    let width = bounds.size.width.max(0.0).round() as u32;
    let height = bounds.size.height.max(0.0).round() as u32;
    let (mut scale_percent, mut refresh_hz) = (100, 0);
    // SAFETY: the copied mode is checked for null and released exactly once below.
    let mode = unsafe { CGDisplayCopyDisplayMode(id) };
    if !mode.is_null() {
        // SAFETY: `mode` is a live mode reference until the release.
        let (pixel_width, refresh) = unsafe {
            (
                CGDisplayModeGetPixelWidth(mode),
                CGDisplayModeGetRefreshRate(mode),
            )
        };
        // SAFETY: we own the reference CGDisplayCopyDisplayMode returned.
        unsafe { CGDisplayModeRelease(mode) };
        if width > 0 && pixel_width > 0 {
            scale_percent = ((pixel_width as f64) * 100.0 / f64::from(width)).round() as u32;
        }
        if refresh.is_finite() && refresh > 0.0 {
            refresh_hz = refresh.round() as u32;
        }
    }
    DisplayInfo {
        index,
        main,
        x: bounds.origin.x.round() as i32,
        y: bounds.origin.y.round() as i32,
        width,
        height,
        scale_percent,
        refresh_hz,
    }
}
