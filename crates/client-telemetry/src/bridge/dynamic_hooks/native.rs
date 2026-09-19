//! Native byte-patching for dynamic hooks (issue #686 scope 4).
//!
//! # Scope of the phase-3 native path
//!
//! - **Function-entry only.** We patch the function *prologue* with a
//!   JMP to a detour (via [`crate::hooks::primitives::install_inline_hook`]),
//!   never a mid-function splice. Per ADR open-question 3, entry-only is
//!   the accepted phase-3 shape; mid-function trampolines need a
//!   length-disassembler and are deferred.
//! - **`cdecl` only.** The detour is a fixed 8-argument `extern "cdecl"`
//!   function. Under cdecl the **caller** cleans the stack, so declaring
//!   more parameters than the target really takes is safe: we read up to
//!   8 stack dwords, forward all 8 to the trampoline (the original reads
//!   only the ones it wants), and clean them up on return. A `stdcall` /
//!   `thiscall` target (callee-cleans, and `this` in ECX) would corrupt
//!   the stack with a fixed-arity cdecl detour, so those conventions are
//!   refused here and left to a follow-up (an asm stub, the same one
//!   general-purpose register capture would need).
//! - **No asm.** General-purpose register capture at entry needs a naked
//!   stub; deferred with the above. The cdecl path captures stack args +
//!   typed dereferences, which covers the observation hooks in scope.
//!
//! # Threading and safety
//!
//! Install / remove run on the main thread (the dispatch drain). A detour
//! runs on whatever thread called the hooked function. The detour reads
//! its slot's trampoline and hook-id **lock-free** (atomics), then locks
//! the registry only briefly to record the hit and clone the spec, drops
//! the lock, does never-faulting `VirtualQuery`-guarded reads for the
//! capture, and finally calls the trampoline. It never holds a lock across
//! the trampoline call (which can re-enter the hooked function), and it is
//! `catch_unwind`-wrapped so a capture bug can't unwind into the client.
//!
//! Removing a hook restores the prologue bytes; per the primitives'
//! contract this is unsafe if another thread is executing inside the
//! patched prologue at that instant. This crate does not freeze threads
//! (unlike MinHook), so a `hook_remove` races a concurrent fire — an
//! accepted research-tool risk, documented here and in the ADR.

/// Slots in the hook pool. Eight distinct detour functions, so eight
/// concurrent dynamic hooks. Bounded on purpose: this is a research
/// probe, not a general hooking framework.
pub const NUM_SLOTS: usize = 8;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub use imp::{patch, unpatch};

/// Off-target stubs so the dispatch layer compiles everywhere; the real
/// patching only exists inside SGW.exe.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub fn patch(_addr: usize, _id: u32) -> Result<(), String> {
    Err("hook install is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
pub fn unpatch(_id: u32) -> Result<(), String> {
    Err("hook remove is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod imp {
    use super::NUM_SLOTS;
    use crate::hooks::primitives::{install_inline_hook, InlineHook};
    use std::panic::catch_unwind;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Per-slot trampoline address, read lock-free by the detour. 0 = free.
    static SLOT_TRAMPOLINES: [AtomicUsize; NUM_SLOTS] = [const { AtomicUsize::new(0) }; NUM_SLOTS];
    /// Per-slot hook id, read lock-free by the detour. 0 = free.
    static SLOT_IDS: [AtomicU32; NUM_SLOTS] = [const { AtomicU32::new(0) }; NUM_SLOTS];

    /// Storage for the live `InlineHook` (keeps the trampoline alive and
    /// restores the prologue on drop). Touched only on the main thread
    /// during install / remove; the detour never reads it (it uses the
    /// atomics above). `InlineHook` holds raw pointers, so wrap it to
    /// carry it in a `static` — sound because it never actually crosses
    /// threads (install and remove are both main-thread).
    // Held only for its RAII `Drop`, which restores the patched prologue
    // on `hook_remove`; the trampoline is reached via the atomic, so the
    // field itself is never read.
    struct SendHook(#[allow(dead_code)] InlineHook);
    // SAFETY: the InlineHook is only constructed, stored, and dropped on
    // the main thread; the detour reaches the trampoline via a separate
    // atomic, never through this value.
    unsafe impl Send for SendHook {}

    static SLOT_HOOKS: [Mutex<Option<SendHook>>; NUM_SLOTS] =
        [const { Mutex::new(None) }; NUM_SLOTS];

    /// Install an entry hook at `addr` bound to `id`, allocating a free
    /// slot. Returns an error if the pool is full or the byte-patch fails.
    pub fn patch(addr: usize, id: u32) -> Result<(), String> {
        let slot = (0..NUM_SLOTS)
            .find(|&i| SLOT_IDS[i].load(Ordering::Acquire) == 0)
            .ok_or_else(|| format!("hook pool full ({NUM_SLOTS} slots)"))?;

        let detour = DETOURS[slot] as usize;
        // SAFETY: `addr` is a caller-supplied function entry; the detour
        // has the cdecl ABI documented above. A bad address surfaces as a
        // HookError, not a fault. No thread runs in the prologue at
        // install time in practice (see module docs).
        let hook = unsafe { install_inline_hook(addr, detour) }
            .map_err(|e| format!("install_inline_hook: {e}"))?;

        let tramp = hook.trampoline() as usize;
        // Publish the trampoline + id *before* storing the hook so a fire
        // that races install finds a consistent pair.
        SLOT_TRAMPOLINES[slot].store(tramp, Ordering::Release);
        SLOT_IDS[slot].store(id, Ordering::Release);
        *SLOT_HOOKS[slot].lock().unwrap() = Some(SendHook(hook));
        Ok(())
    }

    /// Remove the hook bound to `id`: restore the prologue and free the
    /// slot. No-op error if the id isn't installed here.
    pub fn unpatch(id: u32) -> Result<(), String> {
        let slot = (0..NUM_SLOTS)
            .find(|&i| SLOT_IDS[i].load(Ordering::Acquire) == id)
            .ok_or_else(|| format!("hook id {id} not installed natively"))?;

        // Clear the atomics first so a concurrent fire stops capturing and
        // (if the InlineHook is already gone) doesn't chase a stale
        // trampoline.
        SLOT_IDS[slot].store(0, Ordering::Release);
        // Dropping the InlineHook restores the original prologue bytes.
        let taken = SLOT_HOOKS[slot].lock().unwrap().take();
        drop(taken);
        SLOT_TRAMPOLINES[slot].store(0, Ordering::Release);
        Ok(())
    }

    /// The shared detour body: record the hit, capture per spec, then call
    /// the original via the trampoline. `args` are the up-to-8 stack dwords
    /// the cdecl detour received.
    #[inline]
    fn on_fire(slot: usize, args: &[u32; 8]) -> u32 {
        let id = SLOT_IDS[slot].load(Ordering::Acquire);
        let tramp = SLOT_TRAMPOLINES[slot].load(Ordering::Acquire);

        if id != 0 {
            // Capture is best-effort and must never unwind into the client.
            let _ = catch_unwind(|| super::super::on_hit(id, args));
        }

        if tramp != 0 {
            // SAFETY: the trampoline runs the displaced prologue then jumps
            // back into the original. cdecl: we forward all 8 dwords; the
            // original reads only its real args; we (the caller) clean up.
            let original: unsafe extern "cdecl" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32 =
                unsafe { core::mem::transmute(tramp) };
            unsafe {
                original(
                    args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7],
                )
            }
        } else {
            // Trampoline gone (removed mid-fire) — return 0. The bytes are
            // already restored, so this detour won't be entered again.
            0
        }
    }

    /// Generate one cdecl detour per slot. Each reads its 8 stack dwords,
    /// forwards to [`on_fire`] with its fixed slot index.
    macro_rules! slot_detour {
        ($name:ident, $slot:literal) => {
            #[allow(improper_ctypes_definitions)]
            unsafe extern "cdecl" fn $name(
                a0: u32,
                a1: u32,
                a2: u32,
                a3: u32,
                a4: u32,
                a5: u32,
                a6: u32,
                a7: u32,
            ) -> u32 {
                on_fire($slot, &[a0, a1, a2, a3, a4, a5, a6, a7])
            }
        };
    }

    slot_detour!(detour_0, 0);
    slot_detour!(detour_1, 1);
    slot_detour!(detour_2, 2);
    slot_detour!(detour_3, 3);
    slot_detour!(detour_4, 4);
    slot_detour!(detour_5, 5);
    slot_detour!(detour_6, 6);
    slot_detour!(detour_7, 7);

    type Detour = unsafe extern "cdecl" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32;

    static DETOURS: [Detour; NUM_SLOTS] = [
        detour_0, detour_1, detour_2, detour_3, detour_4, detour_5, detour_6, detour_7,
    ];
}
