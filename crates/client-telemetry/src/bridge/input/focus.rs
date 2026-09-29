//! Virtual focus: while the lab has it on, the game's own "am I the
//! active window?" checks answer yes, so it keeps reading input while the
//! desktop has focus elsewhere.
//!
//! `GetForegroundWindow` is already swapped by the telemetry IAT hooks
//! (which consult [`super::virtual_focus_hwnd`]). This module swaps the
//! two other window-state imports SGW.exe has, `GetFocus` and
//! `GetActiveWindow`, lab sessions only, and `GetCursorPos`: the UI cursor
//! follows the OS cursor, so a lab-set virtual cursor stands in for it. Slots are from the QA SGW.exe
//! import directory (image base `0x00400000`) and are verified against
//! `GetProcAddress` before the swap, like every other IAT hook.

use core::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

const IAT_GET_FOCUS: usize = 0x017E_FDB0;
const IAT_GET_CURSOR_POS: usize = 0x017E_FD70;
const IAT_GET_ACTIVE_WINDOW: usize = 0x017E_FE44;

static ORIG_GET_FOCUS: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_ACTIVE_WINDOW: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_CURSOR_POS: AtomicUsize = AtomicUsize::new(0);
static SCREEN_TO_CLIENT: AtomicUsize = AtomicUsize::new(0);

type FnGetCursorPos = unsafe extern "stdcall" fn(pt: *mut [i32; 2]) -> i32;
type FnScreenToClient = unsafe extern "stdcall" fn(hwnd: *mut c_void, pt: *mut [i32; 2]) -> i32;

/// `GetCursorPos`: while the lab has a virtual cursor and virtual focus,
/// report that cursor (UI/client pixels) converted to screen coordinates,
/// so the game's own `GetCursorPos` + `ScreenToClient` lands on it.
unsafe extern "stdcall" fn get_cursor_pos_detour(pt: *mut [i32; 2]) -> i32 {
    if let (Some(hwnd), Some((x, y)), false) = (
        super::virtual_focus_hwnd(),
        super::virtual_cursor(),
        pt.is_null(),
    ) {
        let s2c = SCREEN_TO_CLIENT.load(Ordering::Acquire);
        if s2c != 0 {
            // The client origin in screen space: ScreenToClient(0,0) = -origin.
            let mut origin = [0i32, 0];
            // SAFETY: the real ScreenToClient; `origin` is a POINT.
            let ok = unsafe {
                core::mem::transmute::<usize, FnScreenToClient>(s2c)(
                    hwnd as *mut c_void,
                    &mut origin,
                )
            };
            if ok != 0 {
                // SAFETY: the caller's POINT.
                unsafe { *pt = [x - origin[0], y - origin[1]] };
                return 1;
            }
        }
    }
    // SAFETY: the original GetCursorPos, stored before the swap.
    unsafe {
        core::mem::transmute::<usize, FnGetCursorPos>(ORIG_GET_CURSOR_POS.load(Ordering::Acquire))(
            pt,
        )
    }
}

type FnHwnd = unsafe extern "stdcall" fn() -> *mut c_void;

unsafe extern "stdcall" fn get_focus_detour() -> *mut c_void {
    if let Some(h) = super::virtual_focus_hwnd() {
        return h as *mut c_void;
    }
    // SAFETY: the original GetFocus, stored before the swap.
    unsafe { core::mem::transmute::<usize, FnHwnd>(ORIG_GET_FOCUS.load(Ordering::Acquire))() }
}

unsafe extern "stdcall" fn get_active_window_detour() -> *mut c_void {
    if let Some(h) = super::virtual_focus_hwnd() {
        return h as *mut c_void;
    }
    // SAFETY: the original GetActiveWindow, stored before the swap.
    unsafe {
        core::mem::transmute::<usize, FnHwnd>(ORIG_GET_ACTIVE_WINDOW.load(Ordering::Acquire))()
    }
}

fn user32_export(symbol: &core::ffi::CStr) -> Result<usize, String> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    let wide: Vec<u16> = "user32.dll".encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated strings; no reference kept.
    unsafe {
        let m = GetModuleHandleW(wide.as_ptr());
        if m.is_null() {
            return Err("user32.dll not loaded".into());
        }
        GetProcAddress(m, symbol.as_ptr().cast()).map(|f| f as usize)
    }
    .ok_or_else(|| format!("{symbol:?} not exported"))
}

/// Swap one user32 import after checking the slot holds it.
///
/// # Safety
/// `slot` must be SGW.exe's IAT entry for `symbol`; `detour` is stdcall.
pub(super) unsafe fn swap(
    slot: usize,
    symbol: &core::ffi::CStr,
    detour: usize,
    orig: &AtomicUsize,
) -> Result<(), String> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    let wide: Vec<u16> = "user32.dll".encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated strings; no reference kept.
    let expected = unsafe {
        let m = GetModuleHandleW(wide.as_ptr());
        if m.is_null() {
            return Err("user32.dll not loaded".into());
        }
        GetProcAddress(m, symbol.as_ptr().cast()).map(|f| f as usize)
    }
    .ok_or_else(|| format!("{symbol:?} not exported"))?;
    let current = cimmeria_client_hookgate::os::read_bytes(slot, 4)
        .and_then(|b| b.try_into().ok())
        .map(|b: [u8; 4]| u32::from_le_bytes(b) as usize);
    if current != Some(expected) {
        return Err(format!("{symbol:?} slot 0x{slot:08x} holds {current:x?}"));
    }
    orig.store(expected, Ordering::Release);
    // SAFETY: slot verified above.
    unsafe { crate::hooks::primitives::replace_iat_slot(slot, detour) }
        .map(|_| ())
        .map_err(|e| format!("{symbol:?} swap failed: {e:?}"))
}

/// Install the `GetFocus` / `GetActiveWindow` swaps.
///
/// # Safety
/// Lab sessions only; called once at bridge start.
pub unsafe fn install() -> Result<(), String> {
    unsafe {
        swap(
            IAT_GET_FOCUS,
            c"GetFocus",
            get_focus_detour as *const c_void as usize,
            &ORIG_GET_FOCUS,
        )?;
        swap(
            IAT_GET_ACTIVE_WINDOW,
            c"GetActiveWindow",
            get_active_window_detour as *const c_void as usize,
            &ORIG_GET_ACTIVE_WINDOW,
        )?;
        SCREEN_TO_CLIENT.store(user32_export(c"ScreenToClient")?, Ordering::Release);
        swap(
            IAT_GET_CURSOR_POS,
            c"GetCursorPos",
            get_cursor_pos_detour as *const c_void as usize,
            &ORIG_GET_CURSOR_POS,
        )
    }
}
