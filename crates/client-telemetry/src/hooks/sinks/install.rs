//! Install plumbing for the sinks and engine seams: an inline (MinHook)
//! hook and an IAT swap, each reporting through the same `client.hooks.*`
//! events as the older hooks and into the [capabilities](super::caps)
//! record.
//!
//! The mechanics are the ones `inline_hooks` and `iat_hooks` use (MinHook
//! `CreateHook` + `EnableHook`; protect-swap-restore of one IAT word). They
//! are repeated here, small, rather than widened, so a change to the older
//! modules cannot alter what a sink installs, and so this module owns the
//! capabilities bookkeeping.

use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use serde_json::Value;

use super::caps::{self, Outcome};
use crate::queue::Producer;

/// One imported function: its IAT slot in `SGW.exe` and what it imports.
#[derive(Debug, Clone, Copy)]
pub(super) struct SlotImport {
    /// Address of the IAT slot.
    pub slot: usize,
    /// The DLL the import comes from.
    pub module: &'static str,
    /// The exported (decorated) name.
    pub symbol: &'static CStr,
}

impl SlotImport {
    /// The address the loader bound this import to, from the module's export
    /// table. `None` when the module is not loaded or lacks the export.
    fn resolved(&self) -> Option<usize> {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let wide: Vec<u16> = self.module.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated strings that outlive the calls; no reference
        // to the module is kept.
        unsafe {
            let module = GetModuleHandleW(wide.as_ptr());
            if module.is_null() {
                return None;
            }
            GetProcAddress(module, self.symbol.as_ptr().cast()).map(|f| f as usize)
        }
    }

    /// What the slot holds now.
    fn current(&self) -> Option<usize> {
        let bytes = cimmeria_client_hookgate::os::read_bytes(self.slot, 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    }
}

fn addr_value(address: usize) -> Value {
    Value::String(format!("0x{address:08x}"))
}

/// Install one inline hook at `address`. Returns whether it is live.
///
/// # Safety
///
/// MinHook is initialised; `address` is a fingerprinted function entry whose
/// first five bytes are whole instructions; `detour` has the exact
/// signature and calling convention of the target.
pub(super) unsafe fn inline(
    producer: &Producer,
    name: &'static str,
    address: usize,
    detour: *mut c_void,
    trampoline_slot: &OnceLock<usize>,
) -> bool {
    let target = address as *mut c_void;
    let mut trampoline: *mut c_void = std::ptr::null_mut();

    let created = minhook_sys::MH_CreateHook(target, detour, &mut trampoline);
    if created != minhook_sys::MH_OK {
        crate::hooks::emit_warn(
            producer,
            "client.hooks.inline.create_failed",
            [
                ("hook", Value::String(name.into())),
                ("status", serde_json::json!(created as i32)),
                ("address", addr_value(address)),
            ],
        );
        caps::record(name, Outcome::Failed("create_failed"));
        return false;
    }
    let _ = trampoline_slot.set(trampoline as usize);

    let enabled = minhook_sys::MH_EnableHook(target);
    if enabled != minhook_sys::MH_OK {
        crate::hooks::emit_warn(
            producer,
            "client.hooks.inline.enable_failed",
            [
                ("hook", Value::String(name.into())),
                ("status", serde_json::json!(enabled as i32)),
            ],
        );
        caps::record(name, Outcome::Failed("enable_failed"));
        return false;
    }

    crate::hooks::emit_info(
        producer,
        "client.hooks.inline.installed",
        [
            ("hook", Value::String(name.into())),
            ("address", addr_value(address)),
        ],
    );
    caps::record(name, Outcome::Installed);
    true
}

/// Swap one IAT slot. The slot must hold exactly the address its import
/// resolves to (anything else is a different build, another hook, or a
/// delay-load stub, and the swap is skipped). The displaced original is
/// published in `orig` before the swap, so a call that lands in the detour
/// the moment the slot changes still finds it. Returns whether it is live.
///
/// # Safety
///
/// `detour` has the exact signature and calling convention of the import.
pub(super) unsafe fn iat(
    producer: &Producer,
    name: &'static str,
    import: SlotImport,
    detour: usize,
    orig: &AtomicUsize,
) -> bool {
    let (expected, current) = (import.resolved(), import.current());
    let Some(original) = expected.filter(|e| Some(*e) == current) else {
        let show =
            |v: Option<usize>| Value::String(v.map_or("none".into(), |v| format!("0x{v:08x}")));
        crate::hooks::emit_warn(
            producer,
            "client.hooks.iat.slot_mismatch",
            [
                ("hook", Value::String(name.into())),
                ("address", addr_value(import.slot)),
                ("expected", show(expected)),
                ("actual", show(current)),
            ],
        );
        // A module that is not loaded at all is a skip, not a fault.
        caps::record(
            name,
            if expected.is_none() {
                Outcome::Skipped("module or export not loaded")
            } else {
                Outcome::Failed("slot_mismatch")
            },
        );
        return false;
    };
    orig.store(original, Ordering::Release);
    match crate::hooks::replace_iat_slot(import.slot, detour) {
        Ok(displaced) => {
            orig.store(displaced, Ordering::Release);
            crate::hooks::emit_info(
                producer,
                "client.hooks.iat.installed",
                [
                    ("hook", Value::String(name.into())),
                    ("address", addr_value(import.slot)),
                    ("original", addr_value(displaced)),
                ],
            );
            caps::record(name, Outcome::Installed);
            true
        }
        Err(_) => {
            orig.store(0, Ordering::Release);
            crate::hooks::emit_warn(
                producer,
                "client.hooks.iat.protect_failed",
                [
                    ("hook", Value::String(name.into())),
                    ("address", addr_value(import.slot)),
                ],
            );
            caps::record(name, Outcome::Failed("protect_failed"));
            false
        }
    }
}
