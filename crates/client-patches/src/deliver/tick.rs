//! The `FEngineLoop::Tick` detour: the main-thread drain, and the send
//! natives' registration.

use core::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::lua_api::{FfiLua, LuaApi};
use super::{drain, ui_lua_state, PER_FRAME_BUDGET};
use crate::memory::ProcessMemory;
use crate::send::engine::note_main_thread;
use crate::send::natives::register_on_main_thread;
use crate::{COUNTERS, EVENTS};

/// MinHook's trampoline to the original `Tick`. Set before the hook is
/// enabled.
pub(crate) static TICK_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

/// The `lua51.dll` exports, resolved once at start-up before any hook goes
/// in. Delivery is off without them, and so are the hooks.
pub(crate) static LUA_API: OnceLock<LuaApi> = OnceLock::new();

/// Frames between checks that `CimmeriaBMNative` is registered: about half
/// a second at 60 frames a second, which is how soon the table appears
/// after the UI (re)creates its `lua_State`.
const REGISTER_EVERY_FRAMES: u32 = 30;

/// Frames seen by the detour, for [`REGISTER_EVERY_FRAMES`].
static FRAMES: AtomicU32 = AtomicU32::new(0);

/// `FEngineLoop::Tick`. Its prologue sets up a C++ exception frame, and
/// UE3 reports fatal errors by throwing, so the `-unwind` ABI lets that
/// reach the engine's handler instead of aborting inside the detour.
type TickFn = unsafe extern "thiscall-unwind" fn(this: *mut c_void);

/// Detour for `FEngineLoop::Tick` (`0x00416ec0`): `__thiscall(FEngineLoop*
/// this)`, no stack arguments. Records the main thread for the send
/// natives, keeps `CimmeriaBMNative` registered, and delivers queued calls
/// before the frame, the
/// same place the telemetry DLL's lab bridge drains its commands, then runs
/// the original. A frame with nothing queued costs one atomic load.
pub(crate) unsafe extern "thiscall-unwind" fn engine_tick_detour(this: *mut c_void) {
    note_main_thread();
    // Registration is checked every few frames rather than every frame:
    // when the table is already there, the check is four memory reads and
    // a protected call with a handful of raw lookups.
    if FRAMES
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(REGISTER_EVERY_FRAMES)
    {
        let _ = std::panic::catch_unwind(register_on_main_thread);
    }
    if !EVENTS.is_empty() {
        let _ = std::panic::catch_unwind(drain_on_main_thread);
    }
    let original = TICK_ORIGINAL.load(Ordering::Acquire);
    if original != 0 {
        // SAFETY: MinHook's trampoline for this exact function and signature.
        unsafe {
            let original = core::mem::transmute::<usize, TickFn>(original);
            original(this);
        }
    }
}

fn drain_on_main_thread() {
    let Some(api) = LUA_API.get() else {
        return;
    };
    // Until the UI has created its lua_State, calls stay queued.
    let Some(state) = ui_lua_state(&ProcessMemory) else {
        return;
    };
    let mut lua = FfiLua {
        api,
        state: state as *mut c_void,
    };
    drain(&EVENTS, &COUNTERS, &mut lua, PER_FRAME_BUDGET);
}
