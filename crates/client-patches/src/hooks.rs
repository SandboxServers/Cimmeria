//! Installing a MinHook detour on a fingerprinted site.
//!
//! MinHook rather than the telemetry crate's hand-rolled trampoline: two of
//! the sites (`FEngineLoop::Tick` and the drop callee) are also hooked by
//! the telemetry DLL, and whichever DLL comes second has to relocate the
//! first one's `E9 rel32` into its trampoline. MinHook does; a fixed
//! five-byte prologue copy would jump to the wrong place. MinHook also
//! suspends the process's other threads while it writes the jump.
//!
//! The two DLLs each link their own MinHook and do not coordinate. To
//! narrow the window where both patch the same site at once, the prologue
//! is read again after `MH_CreateHook` has copied it: if it changed in
//! between, the hook is removed and rebuilt on top of the new bytes.

use core::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use minhook_sys::{
    MH_CreateHook, MH_EnableHook, MH_Initialize, MH_RemoveHook, MH_ERROR_ALREADY_INITIALIZED, MH_OK,
};

use crate::fingerprint::{classify, hex, Prologue, Site};
use crate::memory::{MemoryReader, ProcessMemory};

/// Attempts before giving up on a site whose prologue keeps changing.
const ATTEMPTS: usize = 3;

/// Initialise this DLL's MinHook. Safe to call more than once.
pub(crate) fn init() -> Result<(), String> {
    // SAFETY: no preconditions.
    match unsafe { MH_Initialize() } {
        MH_OK | MH_ERROR_ALREADY_INITIALIZED => Ok(()),
        status => Err(format!("MH_Initialize failed with status {status}")),
    }
}

/// Hook `site` with `detour`, storing the trampoline in `original` before
/// the hook goes live. Returns what the prologue was when it was hooked.
///
/// # Safety
///
/// `detour` must have exactly the calling convention and signature of the
/// function at `site`, and must read its trampoline from `original`.
pub(crate) unsafe fn install(
    site: &Site,
    detour: usize,
    original: &AtomicUsize,
) -> Result<Prologue, String> {
    let target = site.address as *mut c_void;
    for _ in 0..ATTEMPTS {
        let Some(before) = ProcessMemory.read_bytes(site.address, site.expected.len()) else {
            return Err(format!("{}: prologue unreadable", site.name));
        };
        let prologue = classify(site, Some(&before));
        if !prologue.is_usable() {
            return Err(format!(
                "{}: prologue is now {}, not hooking",
                site.name,
                hex(&before)
            ));
        }

        let mut trampoline: *mut c_void = core::ptr::null_mut();
        // SAFETY: a fingerprinted function entry and a detour of the same
        // signature (the caller's contract).
        let status = unsafe { MH_CreateHook(target, detour as *mut c_void, &mut trampoline) };
        if status != MH_OK {
            return Err(format!(
                "{}: MH_CreateHook failed with status {status}",
                site.name
            ));
        }

        let after = ProcessMemory.read_bytes(site.address, site.expected.len());
        if after.as_deref() != Some(before.as_slice()) {
            // Someone patched the site while MinHook was copying it.
            // SAFETY: the hook was created above and never enabled.
            unsafe { MH_RemoveHook(target) };
            continue;
        }

        original.store(trampoline as usize, Ordering::Release);
        // SAFETY: the trampoline is in place for the detour to call.
        let status = unsafe { MH_EnableHook(target) };
        if status != MH_OK {
            // SAFETY: created above; not enabled.
            unsafe { MH_RemoveHook(target) };
            original.store(0, Ordering::Release);
            return Err(format!(
                "{}: MH_EnableHook failed with status {status}",
                site.name
            ));
        }
        return Ok(prologue);
    }
    Err(format!(
        "{}: the prologue changed during each of {ATTEMPTS} attempts",
        site.name
    ))
}
