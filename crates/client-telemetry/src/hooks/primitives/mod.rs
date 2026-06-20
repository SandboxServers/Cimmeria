//! Reusable low-level hooking primitives.
//!
//! This module extracts the *mechanical* core of the three hooking
//! techniques used by the telemetry hooks into a small, documented,
//! technique-agnostic API:
//!
//! - [`install_inline_hook`] — overwrite the first bytes of a target
//!   function with a relative `JMP` to a detour, building a call-
//!   original *trampoline* from the displaced prologue bytes.
//! - [`replace_iat_slot`] — rewrite an Import Address Table slot to
//!   point at a detour, returning the original import pointer.
//! - [`swap_vtable_slot`] — rewrite one entry of a C++ vtable to
//!   point at a detour, returning the original method pointer.
//!
//! # Why a separate primitives module
//!
//! The telemetry crate hand-rolls IAT and vtable swaps and delegates
//! inline hooks to MinHook. The future client-patch crates need
//! the *same* page-protect / pointer-rewrite /
//! cache-flush mechanics but without MinHook (a patch DLL that
//! rewrites cipher entry points wants a self-contained, auditable
//! trampoline it owns end-to-end). Pulling the mechanics here gives
//! both call sites one tested implementation.
//!
//! The IAT and vtable swaps in `iat_hooks.rs` / `vtable_hooks.rs`
//! call straight through to [`replace_iat_slot`] /
//! [`swap_vtable_slot`] — behaviour-identical, just deduplicated.
//!
//! The inline trampoline here is **not** wired into the telemetry
//! inline hooks: those stay on MinHook (which also coordinates a
//! thread freeze on install — a guarantee this lighter primitive
//! deliberately does not make). This primitive exists for the
//! patch crates and is exercised by the unit tests below.
//!
//! # Page protection discipline
//!
//! Every write here follows the same four-step dance:
//!
//! 1. `VirtualProtect` the bytes to a writable protection.
//! 2. Perform the pointer / byte write.
//! 3. Restore the original protection.
//! 4. `FlushInstructionCache` when we wrote *code* (inline hooks);
//!    IAT/vtable slots are *data* the CPU re-reads each indirect
//!    call, so they need no i-cache flush.

#![allow(clippy::missing_safety_doc)] // Safety documented per-fn below.

use thiserror::Error;

/// Errors from a primitive hook install. Every variant carries the
/// `usize` address it failed against so a caller can log which slot /
/// function tripped.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HookError {
    /// `VirtualProtect` returned 0 (failure) making the page
    /// writable. The address is almost certainly not mapped, or not
    /// owned by this process.
    #[error("VirtualProtect failed to make 0x{0:08x} writable")]
    ProtectFailed(usize),

    /// Allocating executable memory for the trampoline failed.
    #[error("failed to allocate trampoline for hook at 0x{0:08x}")]
    TrampolineAllocFailed(usize),
}

/// Minimum bytes overwritten by a relative-JMP detour:
/// `E9 <rel32>` = 1 opcode byte + 4 displacement bytes.
pub const JMP_REL32_LEN: usize = 5;

// ─── Inline (JMP-trampoline) hook ───────────────────────────────

/// A live inline hook. Holds everything needed to (a) call the
/// original function via [`InlineHook::trampoline`] and (b) restore
/// the original bytes on [`InlineHook::remove`] or `Drop`.
///
/// The struct is only constructible on `target_arch = "x86"` — the
/// trampoline layout (relative `JMP rel32`) and the
/// `transmute`-to-call pattern are 32-bit specific.
#[cfg(target_arch = "x86")]
#[derive(Debug)]
pub struct InlineHook {
    /// Address of the patched target function.
    target_addr: usize,
    /// Heap-allocated, executable trampoline: the displaced original
    /// prologue bytes followed by a `JMP` back into the target just
    /// past the patch. Call this as the original function.
    trampoline: *const core::ffi::c_void,
    /// Raw allocation backing `trampoline`, freed on drop.
    trampoline_alloc: *mut core::ffi::c_void,
    /// How many bytes of `trampoline_alloc` we committed (prologue +
    /// 5-byte jump-back).
    trampoline_len: usize,
    /// The original bytes we overwrote at `target_addr`, kept so
    /// `remove`/`Drop` can put them back verbatim.
    original_prologue: [u8; JMP_REL32_LEN],
    /// Set once removed so `Drop` doesn't double-restore.
    removed: bool,
}

#[cfg(target_arch = "x86")]
impl InlineHook {
    /// Pointer to call the original (un-hooked) function. The
    /// returned pointer runs the displaced prologue then jumps back
    /// into the target past the patch — i.e. it behaves exactly like
    /// the original function.
    ///
    /// `transmute` it to the original's `extern` fn signature to
    /// invoke it. The pointer is valid until this `InlineHook` is
    /// dropped or [`remove`](Self::remove)d.
    #[must_use]
    pub fn trampoline(&self) -> *const core::ffi::c_void {
        self.trampoline
    }

    /// The patched target address.
    #[must_use]
    pub fn target(&self) -> usize {
        self.target_addr
    }

    /// Restore the original prologue bytes, unhooking the target.
    /// Idempotent — calling twice (or `remove` then `Drop`) is safe.
    ///
    /// # Safety
    ///
    /// No other thread may be executing inside the patched prologue
    /// region (the first [`JMP_REL32_LEN`] bytes of the target) when
    /// this runs. The caller is responsible for that quiescence —
    /// this primitive does **not** freeze threads (unlike MinHook).
    pub unsafe fn remove(&mut self) -> Result<(), HookError> {
        if self.removed {
            return Ok(());
        }
        // SAFETY: caller guarantees prologue quiescence; we restore
        // exactly the bytes we saved into the exact region we patched.
        unsafe {
            write_code_bytes(self.target_addr, &self.original_prologue)?;
        }
        self.removed = true;
        Ok(())
    }
}

#[cfg(target_arch = "x86")]
impl Drop for InlineHook {
    fn drop(&mut self) {
        // Best-effort restore. We can't surface an error from Drop,
        // so a failed restore is swallowed — the alternative (leaving
        // the patch in place) is strictly worse for the patch crate's
        // teardown story.
        // SAFETY: same contract as `remove`; the caller is expected
        // to have quiesced the prologue before dropping.
        unsafe {
            let _ = self.remove();
        }
        if !self.trampoline_alloc.is_null() {
            // SAFETY: `trampoline_alloc` came from `alloc_exec` with
            // `trampoline_len`; freeing it once on drop is sound, and
            // `is_null` + the field being private prevents reuse.
            unsafe {
                free_exec(self.trampoline_alloc, self.trampoline_len);
            }
            self.trampoline_alloc = core::ptr::null_mut();
        }
    }
}

/// Install an inline (JMP-trampoline) hook: overwrite the first
/// [`JMP_REL32_LEN`] bytes of the function at `target_addr` with a
/// relative `JMP` to `detour_addr`, and build a trampoline that runs
/// the displaced prologue then jumps back into the target so the
/// original remains callable.
///
/// Returns an [`InlineHook`] that unhooks on drop. Call
/// [`InlineHook::trampoline`] to reach the original.
///
/// # Prologue length
///
/// This primitive overwrites exactly [`JMP_REL32_LEN`] bytes. It does
/// **not** disassemble the prologue to find the next instruction
/// boundary — it assumes the first `JMP_REL32_LEN` bytes form a whole
/// number of instructions (the common case for the function entry
/// points we patch, which begin with a standard `push ebp; mov ebp,
/// esp; ...` prologue that is comfortably > 5 bytes and never splits
/// an instruction at offset 5 for the targets in scope). A caller
/// patching an arbitrary function must verify this; mis-splitting an
/// instruction corrupts the trampoline. A length-disassembler is left
/// to a follow-up if a target ever needs it.
///
/// # Safety
///
/// - `target_addr` must point to the first byte of a real function
///   with at least [`JMP_REL32_LEN`] bytes of patchable prologue that
///   form whole instructions.
/// - `detour_addr` must be a function with the **exact** calling
///   convention and signature of the target — a mismatch corrupts the
///   stack on call/return.
/// - No thread may be executing within the patched prologue during
///   install or removal. This primitive does not freeze threads.
#[cfg(target_arch = "x86")]
pub unsafe fn install_inline_hook(
    target_addr: usize,
    detour_addr: usize,
) -> Result<InlineHook, HookError> {
    // Save the prologue bytes we're about to clobber so we can both
    // build the trampoline and restore on unhook.
    // SAFETY: caller guarantees target_addr points at >= 5 bytes of
    // readable code.
    let mut original_prologue = [0u8; JMP_REL32_LEN];
    unsafe {
        core::ptr::copy_nonoverlapping(
            target_addr as *const u8,
            original_prologue.as_mut_ptr(),
            JMP_REL32_LEN,
        );
    }

    // Build the trampoline: [displaced prologue][JMP back to
    // target+5]. Total = 5 (prologue) + 5 (jump back).
    let trampoline_len = JMP_REL32_LEN + JMP_REL32_LEN;
    // SAFETY: allocates `trampoline_len` RWX bytes; null-checked next.
    let trampoline_alloc = unsafe { alloc_exec(trampoline_len) };
    if trampoline_alloc.is_null() {
        return Err(HookError::TrampolineAllocFailed(target_addr));
    }
    let tramp = trampoline_alloc as usize;

    // SAFETY: `tramp` owns `trampoline_len` bytes; we write exactly
    // that many (5 prologue + 5 jump).
    unsafe {
        // Copy the displaced prologue to the front of the trampoline.
        core::ptr::copy_nonoverlapping(original_prologue.as_ptr(), tramp as *mut u8, JMP_REL32_LEN);
        // After the prologue, JMP back to (target_addr + 5). The
        // rel32 is computed from the END of the jump instruction:
        // disp = dest - (jmp_addr + 5).
        let jmp_back_at = tramp + JMP_REL32_LEN;
        write_rel32_jmp(jmp_back_at, target_addr + JMP_REL32_LEN);
    }

    // Patch the target: JMP rel32 → detour. rel32 computed from the
    // byte after the 5-byte jump at the target.
    // SAFETY: code write through the protect/flush dance; bytes are a
    // well-formed `E9 <rel32>`.
    unsafe {
        let mut patch = [0u8; JMP_REL32_LEN];
        encode_rel32_jmp(target_addr, detour_addr, &mut patch);
        write_code_bytes(target_addr, &patch)?;
    }

    Ok(InlineHook {
        target_addr,
        trampoline: tramp as *const core::ffi::c_void,
        trampoline_alloc,
        trampoline_len,
        original_prologue,
        removed: false,
    })
}

// ─── IAT / vtable pointer-slot swap ─────────────────────────────

/// Replace an IAT (Import Address Table) slot at `slot_addr` with
/// `detour_addr`, returning the original import pointer so the caller
/// can chain to it.
///
/// IAT slots are 4-byte (on x86) function pointers in the `.idata`
/// section, marked read-only after loader fixup. This makes the page
/// writable, swaps the pointer, restores protection, and returns the
/// displaced value. No instruction-cache flush is needed — the slot
/// is *data* the CPU re-reads on each indirect `call [slot]`.
///
/// # Safety
///
/// `slot_addr` must point to a valid, writable-after-`VirtualProtect`
/// pointer-sized IAT slot owned by this process. `detour_addr` must
/// have the **exact** calling convention of the imported function
/// (Win32 imports are `__stdcall`; C-library imports are `__cdecl`).
pub unsafe fn replace_iat_slot(slot_addr: usize, detour_addr: usize) -> Result<usize, HookError> {
    // SAFETY: forwarded contract — caller guarantees a valid slot.
    unsafe { swap_pointer_slot(slot_addr, detour_addr) }
}

/// Swap one vtable entry at `slot_addr` (= `vtable_base +
/// slot_index * size_of::<usize>()`) with `detour_addr`, returning
/// the original method pointer.
///
/// Mechanically identical to [`replace_iat_slot`] — vtable slots are
/// also pointer-sized data in `.rdata`. Split out as a distinct name
/// so call sites read intentionally (and so a future divergence,
/// e.g. RTTI-walked multi-slot swaps, has a home).
///
/// # Safety
///
/// `slot_addr` must point to a valid pointer-sized vtable slot owned
/// by this process. `detour_addr` must match the virtual method's
/// `__thiscall` signature exactly.
pub unsafe fn swap_vtable_slot(slot_addr: usize, detour_addr: usize) -> Result<usize, HookError> {
    // SAFETY: forwarded contract — caller guarantees a valid slot.
    unsafe { swap_pointer_slot(slot_addr, detour_addr) }
}

/// Shared engine for the IAT + vtable swaps: protect → read original
/// → write detour → restore protection. Returns the original pointer.
///
/// # Safety
///
/// `slot_addr` must point to a valid pointer-sized slot owned by this
/// process. `detour_addr`'s ABI must match the slot's expected callee.
unsafe fn swap_pointer_slot(slot_addr: usize, detour_addr: usize) -> Result<usize, HookError> {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        use windows_sys::Win32::System::Memory::{
            VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
        };

        let slot_ptr = slot_addr as *mut usize;
        let mut old_protect: PAGE_PROTECTION_FLAGS = 0;

        // Step 1: open the slot's page for writing.
        // SAFETY: `slot_ptr` is caller-guaranteed valid; `old_protect`
        // is a live local.
        let ok = unsafe {
            VirtualProtect(
                slot_ptr as *const core::ffi::c_void,
                core::mem::size_of::<usize>(),
                PAGE_READWRITE,
                &mut old_protect,
            )
        };
        if ok == 0 {
            return Err(HookError::ProtectFailed(slot_addr));
        }

        // Step 2: read original, write detour. Non-atomic write is
        // fine — slot swaps happen during single-threaded DLL
        // bootstrap before any concurrent caller can race the slot.
        // SAFETY: page is now writable; pointer is valid + aligned.
        let original = unsafe { *slot_ptr };
        unsafe {
            *slot_ptr = detour_addr;
        }

        // Step 3: restore the original page protection.
        let mut _unused: PAGE_PROTECTION_FLAGS = 0;
        // SAFETY: same valid pointer; restoring the saved protection.
        let _ = unsafe {
            VirtualProtect(
                slot_ptr as *const core::ffi::c_void,
                core::mem::size_of::<usize>(),
                old_protect,
                &mut _unused,
            )
        };

        Ok(original)
    }

    // On the host (non-x86) builds — the rlib still compiles for unit
    // tests of the portable layers, but pointer-slot swapping against
    // a foreign process's data is x86/Windows-only. We use raw memory
    // writes guarded by the test harness instead; see the cfg'd test
    // path which calls a portable in-memory variant. This branch only
    // exists to keep the signature available; it is never reached on
    // the real DLL target.
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    {
        let _ = (slot_addr, detour_addr);
        // SAFETY: test-only portable path — caller passes a slot in
        // owned, writable test memory.
        let slot_ptr = slot_addr as *mut usize;
        let original = unsafe { *slot_ptr };
        unsafe {
            *slot_ptr = detour_addr;
        }
        Ok(original)
    }
}

// ─── Low-level helpers ──────────────────────────────────────────

/// Encode a relative `JMP rel32` (`E9 <disp32>`) at `from_addr`
/// targeting `to_addr`. The displacement is relative to the byte
/// *after* the 5-byte instruction: `disp = to - (from + 5)`.
#[cfg(target_arch = "x86")]
fn encode_rel32_jmp(from_addr: usize, to_addr: usize, out: &mut [u8; JMP_REL32_LEN]) {
    out[0] = 0xE9; // JMP rel32
    let disp = (to_addr as i32).wrapping_sub((from_addr as i32).wrapping_add(JMP_REL32_LEN as i32));
    out[1..5].copy_from_slice(&disp.to_le_bytes());
}

/// Write a `JMP rel32` directly into already-writable memory (the
/// trampoline buffer, which is RWX from `alloc_exec`). Unlike
/// [`write_code_bytes`] this does no `VirtualProtect` — the caller
/// owns freshly-allocated executable memory.
///
/// # Safety
///
/// `at_addr` must own at least [`JMP_REL32_LEN`] writable bytes.
#[cfg(target_arch = "x86")]
unsafe fn write_rel32_jmp(at_addr: usize, to_addr: usize) {
    let mut buf = [0u8; JMP_REL32_LEN];
    encode_rel32_jmp(at_addr, to_addr, &mut buf);
    // SAFETY: caller guarantees `at_addr` owns >= 5 writable bytes.
    unsafe {
        core::ptr::copy_nonoverlapping(buf.as_ptr(), at_addr as *mut u8, JMP_REL32_LEN);
    }
}

/// Write `bytes` into code memory at `addr` through the
/// protect → write → restore → flush dance. Used to patch a live
/// function prologue (install) or restore it (unhook).
///
/// # Safety
///
/// `addr` must point at `bytes.len()` bytes of code owned by this
/// process. No thread may be executing within `[addr, addr+len)`.
#[cfg(target_arch = "x86")]
unsafe fn write_code_bytes(addr: usize, bytes: &[u8]) -> Result<(), HookError> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
        use windows_sys::Win32::System::Memory::{
            VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        let mut old_protect: PAGE_PROTECTION_FLAGS = 0;
        // SAFETY: caller-guaranteed valid code region; live local out.
        let ok = unsafe {
            VirtualProtect(
                addr as *const core::ffi::c_void,
                bytes.len(),
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            )
        };
        if ok == 0 {
            return Err(HookError::ProtectFailed(addr));
        }

        // SAFETY: page is RWX; we write exactly bytes.len() bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
        }

        let mut _unused: PAGE_PROTECTION_FLAGS = 0;
        // SAFETY: same region; restore saved protection.
        let _ = unsafe {
            VirtualProtect(
                addr as *const core::ffi::c_void,
                bytes.len(),
                old_protect,
                &mut _unused,
            )
        };

        // We wrote *code*: the CPU may hold stale decoded bytes in
        // the i-cache. Flush so the next execution sees the patch.
        // SAFETY: process handle is a pseudo-handle; region is ours.
        unsafe {
            FlushInstructionCache(
                GetCurrentProcess(),
                addr as *const core::ffi::c_void,
                bytes.len(),
            );
        }

        Ok(())
    }

    // Non-Windows x86 (e.g. i686 Linux unit-test host): the test
    // exercises trampoline mechanics against `mprotect`-backed RWX
    // memory it allocates itself, so a plain in-place write suffices.
    #[cfg(not(target_os = "windows"))]
    {
        // SAFETY: test path — caller owns writable+exec memory at
        // `addr` (allocated via `alloc_exec`).
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
        }
        Ok(())
    }
}

/// Allocate `len` bytes of executable (RWX) memory for a trampoline.
/// Returns null on failure.
///
/// # Safety
///
/// The returned pointer, if non-null, owns `len` bytes and must be
/// released exactly once via [`free_exec`] with the same `len`.
#[cfg(target_arch = "x86")]
unsafe fn alloc_exec(len: usize) -> *mut core::ffi::c_void {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE,
        };
        // SAFETY: standard RWX allocation; null-checked by caller.
        unsafe {
            VirtualAlloc(
                core::ptr::null(),
                len,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            )
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // i686 Linux test host: mmap an anonymous RWX page.
        // SAFETY: libc mmap with valid args; MAP_FAILED checked.
        unsafe {
            let p = libc_mmap_rwx(len);
            if p == usize::MAX {
                core::ptr::null_mut()
            } else {
                p as *mut core::ffi::c_void
            }
        }
    }
}

/// Free a trampoline allocation from [`alloc_exec`].
///
/// # Safety
///
/// `ptr` must be a non-null allocation from `alloc_exec` with the
/// same `len`, freed exactly once.
#[cfg(target_arch = "x86")]
unsafe fn free_exec(ptr: *mut core::ffi::c_void, len: usize) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};
        let _ = len; // MEM_RELEASE frees the whole reservation.
                     // SAFETY: `ptr` came from VirtualAlloc; MEM_RELEASE wants size 0.
        unsafe {
            VirtualFree(ptr, 0, MEM_RELEASE);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // SAFETY: `ptr`/`len` pair came from `libc_mmap_rwx`.
        unsafe {
            libc_munmap(ptr as usize, len);
        }
    }
}

// ─── i686 Linux test-host shims (no libc dep in Cargo.toml) ─────
//
// The unit tests run on whatever host CI gives us. On i686 Linux we
// need RWX memory; rather than add a `libc` dependency just for the
// test, we declare the two syscalls we need directly. These are NOT
// compiled on the real DLL target (Windows x86), which takes the
// VirtualAlloc path above.

#[cfg(all(target_arch = "x86", not(target_os = "windows")))]
extern "C" {
    fn mmap(addr: usize, length: usize, prot: i32, flags: i32, fd: i32, offset: isize) -> isize;
    fn munmap(addr: usize, length: usize) -> i32;
}

#[cfg(all(target_arch = "x86", not(target_os = "windows")))]
unsafe fn libc_mmap_rwx(len: usize) -> usize {
    // PROT_READ|WRITE|EXEC = 7, MAP_PRIVATE|ANONYMOUS = 0x22.
    const PROT_RWX: i32 = 0x1 | 0x2 | 0x4;
    const MAP_PRIVATE_ANON: i32 = 0x02 | 0x20;
    // SAFETY: anonymous mapping, fd = -1, offset 0; result checked
    // against MAP_FAILED (-1) by the caller via usize::MAX sentinel.
    let r = unsafe { mmap(0, len, PROT_RWX, MAP_PRIVATE_ANON, -1, 0) };
    if r == -1 {
        usize::MAX
    } else {
        r as usize
    }
}

#[cfg(all(target_arch = "x86", not(target_os = "windows")))]
unsafe fn libc_munmap(addr: usize, len: usize) {
    // SAFETY: addr/len pair came from libc_mmap_rwx.
    unsafe {
        munmap(addr, len);
    }
}

#[cfg(all(test, target_arch = "x86"))]
mod tests;
