//! The Lua C functions behind `CimmeriaBMNative`, and their registration
//! from the `Tick` detour.

use core::ffi::c_void;

use super::engine::EngineSender;
use super::register::{self, Registration};
use super::{run, Native, NATIVE_TABLE, VERSION};
use crate::counters::{bump, is_log_worthy};
use crate::deliver::lua_api::{AbortOnPanic, CFunction, FfiLua, State};
use crate::deliver::tick::LUA_API;
use crate::deliver::ui_lua_state;
use crate::log;
use crate::memory::ProcessMemory;
use crate::COUNTERS;

/// The C function for `native`.
pub(crate) fn function(native: Native) -> CFunction {
    match native {
        Native::Search => bm_search,
        Native::Create => bm_create,
        Native::Bid => bm_bid,
        Native::Cancel => bm_cancel,
        Native::Watch => bm_watch,
        Native::TechCompetency => bm_tech_competency,
    }
}

// Each is a `lua_CFunction`. `C-unwind`, because a Lua error raised by an
// API call inside (in practice, out of memory pushing the reason string) is
// a C++ exception that must unwind through to the script's `pcall`.
unsafe extern "C-unwind" fn bm_search(state: State) -> i32 {
    entry(state, Native::Search)
}

unsafe extern "C-unwind" fn bm_create(state: State) -> i32 {
    entry(state, Native::Create)
}

unsafe extern "C-unwind" fn bm_bid(state: State) -> i32 {
    entry(state, Native::Bid)
}

unsafe extern "C-unwind" fn bm_cancel(state: State) -> i32 {
    entry(state, Native::Cancel)
}

unsafe extern "C-unwind" fn bm_watch(state: State) -> i32 {
    entry(state, Native::Watch)
}

unsafe extern "C-unwind" fn bm_tech_competency(state: State) -> i32 {
    entry(state, Native::TechCompetency)
}

fn entry(state: State, native: Native) -> i32 {
    // A Rust panic must not unwind into Lua's C++ `catch (...)`. The
    // engine call inside `run` has its own `catch_unwind`; this guard only
    // covers reading the arguments and pushing the result.
    let _guard = AbortOnPanic;
    // The table is registered only after the API resolved, so this is
    // always set; with nothing pushed the script sees `nil`.
    let Some(api) = LUA_API.get() else {
        return 0;
    };
    let mut lua = FfiLua { api, state };
    run(&mut lua, native, &mut EngineSender::live(), &COUNTERS)
}

/// Make sure the UI Lua has `CimmeriaBMNative`. Called from the `Tick`
/// detour on the main thread; does nothing until the UI `lua_State` is up.
pub(crate) fn register_on_main_thread() {
    let Some(api) = LUA_API.get() else {
        return;
    };
    let Some(state) = ui_lua_state(&ProcessMemory) else {
        return;
    };
    let mut lua = FfiLua {
        api,
        state: state as *mut c_void,
    };
    match register::ensure(&mut lua) {
        Registration::AlreadyPresent => {}
        Registration::Registered { replaced } => {
            let n = bump(&COUNTERS.natives_registered);
            log::line(format_args!(
                "registered {NATIVE_TABLE} (version {VERSION}) in lua_State 0x{state:08x}{} (#{n})",
                if replaced {
                    ", replacing a stale value"
                } else {
                    ""
                }
            ));
        }
        Registration::NoStackSpace => {
            let n = bump(&COUNTERS.register_failed);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "registering {NATIVE_TABLE} failed: no Lua stack space (#{n})"
                ));
            }
        }
        Registration::SetupError { status, message } => {
            let n = bump(&COUNTERS.register_failed);
            if is_log_worthy(n) {
                let short: String = message.chars().take(200).collect();
                log::line(format_args!(
                    "registering {NATIVE_TABLE} raised a Lua error (status {status}): {short} (#{n})"
                ));
            }
        }
    }
}
