use super::*;

// ── Inline hook: install on a real local function, verify the
//    detour runs AND the trampoline calls through to the original.

/// The function we hook. `#[inline(never)]` so it has a real,
/// patchable prologue at a stable address. Returns `base + 1`.
#[inline(never)]
extern "C" fn add_one(base: i32) -> i32 {
    // The black_box keeps the body from folding to a constant and
    // ensures a non-trivial prologue exists to patch.
    std::hint::black_box(base) + 1
}

// Detour state: where the detour stashes the trampoline so it can
// chain to the original. Set by the test before installing.
static mut DETOUR_TRAMPOLINE: usize = 0;
static DETOUR_HITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Detour for `add_one`: record a hit, then call the original via
/// the trampoline and add 100 to its result. So a hooked
/// `add_one(x)` returns `(x + 1) + 100`.
extern "C" fn add_one_detour(base: i32) -> i32 {
    DETOUR_HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    // SAFETY: DETOUR_TRAMPOLINE was set to a valid trampoline ptr
    // by the test before any call routes here.
    let tramp = unsafe { DETOUR_TRAMPOLINE };
    let original: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(tramp) };
    original(base) + 100
}

#[test]
fn inline_hook_detours_and_chains_to_original() {
    // Baseline: unhooked.
    assert_eq!(add_one(5), 6);

    let target = add_one as *const () as usize;
    let detour = add_one_detour as *const () as usize;

    // SAFETY: target is a real #[inline(never)] fn with a > 5 byte
    // prologue; detour matches its `extern "C" fn(i32) -> i32`
    // signature; the test is single-threaded so the prologue is
    // quiescent during patch.
    let hook = unsafe { install_inline_hook(target, detour) }.expect("inline hook should install");

    // Publish the trampoline so the detour can chain through.
    // SAFETY: single-threaded test; no concurrent reader.
    unsafe {
        DETOUR_TRAMPOLINE = hook.trampoline() as usize;
    }
    DETOUR_HITS.store(0, std::sync::atomic::Ordering::SeqCst);

    // Now the detour runs: (5 + 1) + 100 = 106.
    assert_eq!(add_one(5), 106, "detour should run and chain to original");
    assert_eq!(
        DETOUR_HITS.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "detour must have been entered exactly once"
    );

    // Calling the trampoline directly must give the ORIGINAL
    // behaviour (no +100), proving it reconstructs the prologue
    // + jumps back correctly.
    let tramp: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(hook.trampoline()) };
    assert_eq!(tramp(40), 41, "trampoline must call the unhooked original");

    // Drop unhooks: behaviour returns to baseline.
    drop(hook);
    assert_eq!(add_one(5), 6, "drop must restore original prologue");
}

#[test]
fn inline_hook_explicit_remove_restores() {
    assert_eq!(add_one(7), 8);
    let mut hook = unsafe {
        install_inline_hook(
            add_one as *const () as usize,
            add_one_detour as *const () as usize,
        )
    }
    .expect("install");
    unsafe {
        DETOUR_TRAMPOLINE = hook.trampoline() as usize;
    }
    // Hooked.
    assert_eq!(add_one(7), 108);
    // Explicit remove → restored, and idempotent.
    unsafe { hook.remove() }.expect("remove");
    assert_eq!(add_one(7), 8);
    unsafe { hook.remove() }.expect("second remove is a no-op");
    assert_eq!(add_one(7), 8);
}

// ── IAT slot swap against a synthetic table in test memory. ──

extern "stdcall" fn iat_original() -> u32 {
    1
}
extern "stdcall" fn iat_detour() -> u32 {
    2
}

#[test]
fn replace_iat_slot_swaps_and_returns_original() {
    // A synthetic 1-entry IAT: a single pointer-sized slot in our
    // own writable memory holding the "imported" function ptr.
    let mut slot: usize = iat_original as *const () as usize;
    let slot_addr = &mut slot as *mut usize as usize;

    // SAFETY: slot_addr points at an owned, writable usize.
    let original =
        unsafe { replace_iat_slot(slot_addr, iat_detour as *const () as usize) }.expect("iat swap");

    assert_eq!(
        original, iat_original as *const () as usize,
        "must return displaced ptr"
    );
    assert_eq!(
        slot, iat_detour as *const () as usize,
        "slot must now hold the detour"
    );

    // The slot now dispatches to the detour; calling through it
    // proves the swap took.
    let via_slot: extern "stdcall" fn() -> u32 = unsafe { std::mem::transmute(slot) };
    assert_eq!(via_slot(), 2);

    // And the returned original still reaches the real function.
    let via_orig: extern "stdcall" fn() -> u32 = unsafe { std::mem::transmute(original) };
    assert_eq!(via_orig(), 1);
}

// ── Vtable slot swap against a synthetic vtable in test memory. ─

extern "thiscall" fn vfunc_original(_this: *mut core::ffi::c_void) -> u32 {
    10
}
extern "thiscall" fn vfunc_detour(_this: *mut core::ffi::c_void) -> u32 {
    20
}

#[test]
fn swap_vtable_slot_swaps_and_returns_original() {
    // Synthetic vtable: 3 slots, swap slot index 1.
    let mut vtable: [usize; 3] = [
        0xDEAD_BEEF,
        vfunc_original as *const () as usize,
        0xFEED_FACE,
    ];
    let slot_addr = &mut vtable[1] as *mut usize as usize;

    // SAFETY: slot_addr points at an owned, writable vtable entry.
    let original = unsafe { swap_vtable_slot(slot_addr, vfunc_detour as *const () as usize) }
        .expect("vtable swap");

    assert_eq!(original, vfunc_original as *const () as usize);
    assert_eq!(vtable[1], vfunc_detour as *const () as usize);
    // Untouched slots are intact.
    assert_eq!(vtable[0], 0xDEAD_BEEF);
    assert_eq!(vtable[2], 0xFEED_FACE);

    // Dispatch through the swapped slot hits the detour.
    let via_slot: extern "thiscall" fn(*mut core::ffi::c_void) -> u32 =
        unsafe { std::mem::transmute(vtable[1]) };
    assert_eq!(via_slot(core::ptr::null_mut()), 20);
}

/// `JMP_REL32_LEN` is the load-bearing constant for the whole
/// inline-hook layout; pin it so a careless edit can't desync the
/// patch length from the trampoline length.
#[test]
fn jmp_rel32_len_is_five() {
    assert_eq!(JMP_REL32_LEN, 5);
}

/// The rel32 encoding must round-trip: encode a jump, decode the
/// displacement, and confirm it lands on the intended target.
#[test]
fn rel32_jmp_encodes_correct_displacement() {
    let from = 0x0040_1000usize;
    let to = 0x0040_2000usize;
    let mut buf = [0u8; JMP_REL32_LEN];
    encode_rel32_jmp(from, to, &mut buf);
    assert_eq!(buf[0], 0xE9, "opcode must be JMP rel32");
    let disp = i32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]);
    // dest = from + 5 + disp
    let dest = (from as i32 + JMP_REL32_LEN as i32 + disp) as usize;
    assert_eq!(dest, to);
}
