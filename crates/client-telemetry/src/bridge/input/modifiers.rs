//! Virtual modifier keys (Shift, Ctrl, Alt) for lab typing.
//!
//! The game turns a key press into a character with `GetKeyboardState` +
//! `ToUnicodeEx`, so the case of a typed letter comes from the real
//! keyboard's Shift, which a posted `WM_KEYDOWN` does not change. While the
//! lab holds a modifier, `GetKeyboardState` and `GetKeyState` report it
//! down. Lab sessions only; slots from the QA SGW.exe import directory,
//! verified before the swap.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

const IAT_GET_KEY_STATE: usize = 0x017E_FDC0;
const IAT_GET_KEYBOARD_STATE: usize = 0x017E_FE84;

static ORIG_GET_KEY_STATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_GET_KEYBOARD_STATE: AtomicUsize = AtomicUsize::new(0);

/// Held modifiers: bit 0 Shift, bit 1 Ctrl, bit 2 Alt.
static HELD: AtomicU8 = AtomicU8::new(0);

pub const SHIFT: u8 = 1;
pub const CTRL: u8 = 2;
pub const ALT: u8 = 4;

/// Set the held modifiers (bitmask of [`SHIFT`], [`CTRL`], [`ALT`]).
pub fn set_held(mask: u8) {
    HELD.store(mask & 7, Ordering::Relaxed);
}

pub fn held() -> u8 {
    HELD.load(Ordering::Relaxed)
}

/// The virtual-key codes a modifier bit covers: the generic key and both
/// sides (`VK_SHIFT`, `VK_LSHIFT`, `VK_RSHIFT`, ...).
pub fn vks_for(mask: u8) -> Vec<u8> {
    let mut v = Vec::new();
    if mask & SHIFT != 0 {
        v.extend([0x10, 0xA0, 0xA1]);
    }
    if mask & CTRL != 0 {
        v.extend([0x11, 0xA2, 0xA3]);
    }
    if mask & ALT != 0 {
        v.extend([0x12, 0xA4, 0xA5]);
    }
    v
}

/// Mark the held modifiers down in a 256-byte keyboard-state array.
pub fn overlay(state: &mut [u8]) {
    for vk in vks_for(held()) {
        if let Some(b) = state.get_mut(usize::from(vk)) {
            *b |= 0x80;
        }
    }
}

type FnGetKeyboardState = unsafe extern "stdcall" fn(state: *mut u8) -> i32;
type FnGetKeyState = unsafe extern "stdcall" fn(vk: i32) -> i16;

unsafe extern "stdcall" fn get_keyboard_state_detour(state: *mut u8) -> i32 {
    // SAFETY: the original GetKeyboardState, stored before the swap.
    let ok = unsafe {
        core::mem::transmute::<usize, FnGetKeyboardState>(
            ORIG_GET_KEYBOARD_STATE.load(Ordering::Acquire),
        )(state)
    };
    if ok != 0 && !state.is_null() && held() != 0 {
        // SAFETY: GetKeyboardState's buffer is 256 bytes.
        overlay(unsafe { core::slice::from_raw_parts_mut(state, 256) });
    }
    ok
}

unsafe extern "stdcall" fn get_key_state_detour(vk: i32) -> i16 {
    if (0..=255).contains(&vk) && vks_for(held()).contains(&(vk as u8)) {
        // High bit set = down.
        return i16::MIN;
    }
    // SAFETY: the original GetKeyState, stored before the swap.
    unsafe {
        core::mem::transmute::<usize, FnGetKeyState>(ORIG_GET_KEY_STATE.load(Ordering::Acquire))(vk)
    }
}

/// Install the swaps.
///
/// # Safety
/// Lab sessions only; called once at bridge start.
pub unsafe fn install() -> Result<(), String> {
    unsafe {
        super::focus::swap(
            IAT_GET_KEYBOARD_STATE,
            c"GetKeyboardState",
            get_keyboard_state_detour as *const c_void as usize,
            &ORIG_GET_KEYBOARD_STATE,
        )?;
        super::focus::swap(
            IAT_GET_KEY_STATE,
            c"GetKeyState",
            get_key_state_detour as *const c_void as usize,
            &ORIG_GET_KEY_STATE,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_sets_generic_and_sided_shift() {
        set_held(SHIFT);
        let mut st = [0u8; 256];
        overlay(&mut st);
        assert_eq!(
            (st[0x10], st[0xA0], st[0xA1], st[0x11]),
            (0x80, 0x80, 0x80, 0)
        );
        set_held(0);
        let mut st = [0u8; 256];
        overlay(&mut st);
        assert!(st.iter().all(|&b| b == 0));
    }
}
