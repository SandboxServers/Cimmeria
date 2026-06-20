//! CME EventSignal subscriber registration.
//!
//! The host CME framework (SGW.exe) exposes a typed pub-sub system
//! where every event class is a `TypedEmitInfo` with subscribers
//! stored in an unrtti'd `std::set<CmeMemberCallback*>` keyed by raw
//! pointer. The Subscribe function `0x00a5c150` does an unguarded
//! set insert — no RTTI check, no type gatekeeper. That means we
//! can register fake subscribers from our DLL and the host will
//! invoke our callback at every dispatch of the matching signal.
//!
//! # ABI (recovered from Ghidra, 2026-05-26)
//!
//! `CmeMemberCallback` is a 12-byte struct laid out as:
//!
//! ```text
//! offset 0x0:  vtable*       — pointer to a 6-slot virtual table
//! offset 0x4:  this*         — the `this` pointer passed to the callback
//! offset 0x8:  method_ptr    — extern "thiscall" fn(this, event_data)
//! ```
//!
//! The dispatch site at `CmeEventSignal_InvokeMemberCallback`
//! (`0x00e04570`) unconditionally calls `*(this+0x8)` — slot 5 of
//! our fake vtable can be left as the real `0x00e04570` because
//! the host's own invoker IS the dispatch trampoline.
//!
//! # Lifetime
//!
//! The signal stores subscribers by pointer with NO refcount and
//! NO destructor (the Unsubscribe path doesn't exist — see
//! `client-instrumentation-hookpoints.md`). So our
//! `CmeMemberCallback` objects MUST live for process lifetime —
//! `static`s in `.data`, not heap allocations. The DLL's
//! `DLL_PROCESS_DETACH` is intentionally a no-op for the same
//! reason: freeing `.data` would dangle the host's subscriber
//! pointers.

#![allow(dead_code)] // Phase 2: surface exists for hook installers in same PR.

use std::ffi::c_void;

/// Address of `CmeEventSignal_GetSystem` — returns the global CME
/// EventSignal system handle (a `void*`). All other CME functions
/// take this handle as their first parameter.
///
/// Anchor from `docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`.
#[cfg(windows)]
pub const ADDR_GET_SYSTEM: usize = 0x0155f790;

/// Address of `CmeEventSignal_LookupByName`. Signature:
/// `extern "thiscall" fn(system, *const u8 name) -> *mut c_void`.
/// Returns a signal handle, or null on miss.
#[cfg(windows)]
pub const ADDR_LOOKUP_BY_NAME: usize = 0x00a5c0f0;

/// Address of `CmeEventSignal_Subscribe`. Signature:
/// `extern "thiscall" fn(signal_handle, callback: *const CmeMemberCallback)`.
/// Always succeeds (no error return) — the underlying set insert is
/// idempotent.
#[cfg(windows)]
pub const ADDR_SUBSCRIBE: usize = 0x00a5c150;

/// Address of `CmeEventSignal_InvokeMemberCallback`. Used as the
/// real method-pointer at vtable slot 5 of our fake vtable — the
/// host's own dispatch trampoline routes through this address.
#[cfg(windows)]
pub const ADDR_INVOKE_MEMBER_CALLBACK: usize = 0x00e04570;

/// The 12-byte struct the host's subscriber set stores by pointer.
///
/// `#[repr(C)]` to pin the offsets — Rust must not reorder fields.
#[repr(C)]
pub struct CmeMemberCallback {
    /// Pointer to our 6-slot fake vtable in `.data`. The host
    /// reads slot 5 (`InvokeMemberCallback`) at dispatch time and
    /// follows it; the other slots are touched only in rare
    /// teardown paths that the no-unsubscribe contract avoids.
    pub vtable: *const FakeVtable,
    /// `this` pointer passed to the callback. We use it to store a
    /// pointer to the per-hook state (the producer handle, sampling
    /// counter, etc.) so the callback can read context without a
    /// thread-local lookup.
    pub this_ptr: *const c_void,
    /// `extern "thiscall" fn(this, event_data)` — the actual
    /// observer. `event_data` is whatever the signal emits;
    /// in 2009-era CME this is typically a `void*` to a per-event
    /// struct the subscriber casts according to the signal's type.
    pub method_ptr: *const c_void,
}

// SAFETY: `CmeMemberCallback` is `repr(C)` with three pointer
// fields. The pointers are to immutable .data and process-lifetime
// statics — never dropped. `Sync` is required because we put these
// in a `static` and the host may dispatch on multiple threads.
unsafe impl Sync for CmeMemberCallback {}

/// 6-slot vtable. The host's invoker calls slot 5 (`InvokeMemberCallback`)
/// unconditionally. The other 5 slots are placeholders — the host
/// touches them only in teardown paths the no-unsubscribe contract
/// avoids.
#[repr(C)]
pub struct FakeVtable {
    /// Slot 0: destructor. The host never calls this for static
    /// subscribers (the no-unsubscribe contract), but the slot
    /// exists in the C++ vtable layout. Point to a noop helper to
    /// be safe.
    pub destructor: *const c_void,
    pub slot_1: *const c_void,
    pub slot_2: *const c_void,
    pub slot_3: *const c_void,
    pub slot_4: *const c_void,
    /// Slot 5: `InvokeMemberCallback` — the host's dispatch
    /// trampoline. Points back to `ADDR_INVOKE_MEMBER_CALLBACK`
    /// inside SGW.exe so the host's own code unpacks `this_ptr`
    /// and `method_ptr` and calls our handler.
    pub invoke_member_callback: *const c_void,
}

// SAFETY: same as `CmeMemberCallback` — process-lifetime statics
// of immutable pointers.
unsafe impl Sync for FakeVtable {}

/// No-op destructor used in vtable slot 0. Signature is
/// `extern "thiscall" fn(this) -> *mut c_void` per the C++ ABI
/// for a virtual destructor, but we just return null and ignore
/// `this` since static subscribers never get destroyed.
///
/// # Safety
/// Installed only as a vtable slot the host invokes via the C++ `thiscall`
/// ABI. `_this` is ignored and never dereferenced, so any pointer value the
/// host passes is sound — the function performs no memory access.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub unsafe extern "thiscall" fn noop_destructor(_this: *mut c_void) -> *mut c_void {
    std::ptr::null_mut()
}

/// Sentinel placeholder for unused vtable slots. The host never
/// invokes these for static subscribers, but having a non-null
/// pointer in every slot is defence against an unexpected vcall.
/// Points back at `noop_destructor`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub fn placeholder_slot() -> *const c_void {
    noop_destructor as *const c_void
}

/// Look up a signal handle by name. Returns null on miss (the
/// caller treats null as "this signal doesn't exist on this build
/// of SGW.exe" and skips the subscribe).
///
/// # Safety
///
/// Calls into SGW.exe at hardcoded addresses. The caller must
/// guarantee:
/// 1. The DLL is loaded into SGW.exe (the addresses are otherwise
///    in unrelated memory).
/// 2. ASLR is disabled on SGW.exe (AtreaFixASLR.bat ran on-disk).
/// 3. The DLL's bootstrap thread has reached the point where
///    SGW.exe has finished its own CME registration (we wait for
///    `bootstrap_main` to be called, which is after `DllMain`
///    returns, which is after the SGW process is fully loaded).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub unsafe fn lookup_by_name(name: &[u8]) -> *mut c_void {
    // The system handle is reachable via GetSystem. We could cache
    // it in a OnceLock if many hooks call this, but for the 2
    // hooks we install today the extra call is negligible.
    let get_system: extern "thiscall" fn() -> *mut c_void =
        unsafe { std::mem::transmute(ADDR_GET_SYSTEM) };
    let system = get_system();
    if system.is_null() {
        return std::ptr::null_mut();
    }
    let lookup: extern "thiscall" fn(*mut c_void, *const u8) -> *mut c_void =
        unsafe { std::mem::transmute(ADDR_LOOKUP_BY_NAME) };
    // The name parameter is a null-terminated C string. We require
    // the caller to pass a slice that already ends in `\0`.
    debug_assert!(
        name.ends_with(b"\0"),
        "lookup_by_name requires a null-terminated byte slice"
    );
    lookup(system, name.as_ptr())
}

/// Subscribe a static `CmeMemberCallback` to a signal. Idempotent
/// (the underlying set insert dedupes by pointer).
///
/// # Safety
///
/// Same as `lookup_by_name`, plus: `callback` must point to a
/// `'static` `CmeMemberCallback` (i.e. a `static` in `.data`, not
/// a heap allocation that could be freed).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub unsafe fn subscribe(signal_handle: *mut c_void, callback: *const CmeMemberCallback) {
    if signal_handle.is_null() {
        return;
    }
    let sub: extern "thiscall" fn(*mut c_void, *const CmeMemberCallback) =
        unsafe { std::mem::transmute(ADDR_SUBSCRIBE) };
    sub(signal_handle, callback);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pointer-size check. The struct layout pins below assume a
    /// 32-bit pointer (i686-pc-windows-msvc — the actual DLL
    /// target). On 64-bit hosts pointers are 8 bytes wide, so
    /// the byte counts wouldn't match — gate those tests on the
    /// real target via `cfg`. This pin documents the assumption
    /// without firing a misleading failure on x64 dev hosts.
    #[test]
    fn pointer_size_is_known() {
        // This is a free-form sanity check; the real ABI pins are
        // below and only run on x86 where the byte counts match.
        let size = std::mem::size_of::<*const u8>();
        assert!(
            size == 4 || size == 8,
            "unexpected pointer size {size}; ABI pins assume i686 (4-byte ptr)"
        );
    }

    /// Pin the 12-byte ABI on the actual DLL target (i686). Adding
    /// a field would silently break the host's pointer-based set
    /// membership.
    #[cfg(target_pointer_width = "32")]
    #[test]
    fn cme_member_callback_is_12_bytes() {
        assert_eq!(std::mem::size_of::<CmeMemberCallback>(), 12);
    }

    /// Pin the 24-byte vtable ABI (6 pointer slots × 4 bytes each
    /// on i686). Slot 5 must land at offset 0x14.
    #[cfg(target_pointer_width = "32")]
    #[test]
    fn fake_vtable_layout() {
        assert_eq!(std::mem::size_of::<FakeVtable>(), 24);
        let v = FakeVtable {
            destructor: std::ptr::null(),
            slot_1: std::ptr::null(),
            slot_2: std::ptr::null(),
            slot_3: std::ptr::null(),
            slot_4: std::ptr::null(),
            invoke_member_callback: 0xDEADBEEFusize as *const c_void,
        };
        let base = &v as *const FakeVtable as usize;
        let slot5 = &v.invoke_member_callback as *const _ as usize;
        assert_eq!(slot5 - base, 0x14, "slot 5 must be at offset 0x14");
    }

    /// On 64-bit hosts, pin the SHAPE (6 slots, all pointers).
    /// Byte count differs (8-byte ptrs → 48 bytes, slot 5 at
    /// 0x28) but the struct's logical layout is the same. The
    /// 32-bit pin above is the load-bearing one for the real
    /// DLL.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn fake_vtable_has_six_slots_64bit() {
        assert_eq!(std::mem::size_of::<FakeVtable>(), 48);
        let v = FakeVtable {
            destructor: std::ptr::null(),
            slot_1: std::ptr::null(),
            slot_2: std::ptr::null(),
            slot_3: std::ptr::null(),
            slot_4: std::ptr::null(),
            invoke_member_callback: 0xDEADBEEFusize as *const c_void,
        };
        let base = &v as *const FakeVtable as usize;
        let slot5 = &v.invoke_member_callback as *const _ as usize;
        assert_eq!(slot5 - base, 0x28, "slot 5 at 0x28 on 64-bit");
    }
}
