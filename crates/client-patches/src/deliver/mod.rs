//! The deliver side: hand decoded calls to the UI Lua, on the main thread.
//!
//! A detour on `FEngineLoop::Tick` checks the queue each frame (one atomic
//! load when it is empty). When there is work and the UI's `lua_State` is
//! up, it pops up to [`PER_FRAME_BUDGET`] calls and makes each one a call
//! into the overlay (`CimmeriaBM.onOpen(...)` and so on; the full contract
//! is in [`plan`]). While the `lua_State` is not up yet, calls stay queued.
//! If the overlay is missing, a call is dropped and counted.
//!
//! Lua is only ever touched here, on the main thread.

pub mod lua_stack;
pub mod plan;

#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod lua_api;
#[cfg(all(windows, target_arch = "x86"))]
pub(crate) mod tick;

#[cfg(test)]
pub(crate) mod fake_lua;
#[cfg(test)]
mod tests;

use cimmeria_patch_wire::black_market::{ClientCall, ClientMethod};

use crate::addresses::{LUA_STATE_TT, LUA_TTHREAD, SGW_UI_MANAGER, UI_MANAGER_LUA_SLOT};
use crate::counters::{bump, is_log_worthy, Counters};
use crate::log;
use crate::memory::MemoryReader;
use crate::queue::EventQueue;
use lua_stack::{Delivery, LuaStack};

/// Most calls delivered in one frame. A search page is one call, so this
/// only matters for a burst, which then spreads over a few frames.
pub const PER_FRAME_BUDGET: usize = 16;

/// The UI `lua_State`: `*(*(*(0x01ee2a58) + 0x10))`, each hop non-null,
/// and the result's type tag byte `LUA_TTHREAD`. `None` until the UI is up.
pub fn ui_lua_state<M: MemoryReader>(mem: &M) -> Option<usize> {
    let manager = mem.read_ptr(SGW_UI_MANAGER)?;
    let slot = mem.read_ptr(manager.checked_add(UI_MANAGER_LUA_SLOT)?)?;
    let state = mem.read_ptr(slot)?;
    let tt = mem.read_bytes(state.checked_add(LUA_STATE_TT)?, 1)?;
    (tt[0] == LUA_TTHREAD).then_some(state)
}

/// Deliver up to `budget` queued calls through `lua`. Returns how many were
/// taken off the queue. The queue lock is held only while popping.
pub fn drain<L: LuaStack>(
    queue: &EventQueue<ClientCall>,
    counters: &Counters,
    lua: &mut L,
    budget: usize,
) -> usize {
    let mut taken = 0;
    while taken < budget {
        let Some(call) = queue.pop() else {
            break;
        };
        taken += 1;
        let planned = plan::plan(&call);
        let outcome = lua_stack::deliver(lua, &planned);
        record(counters, call.method(), planned.function, &outcome);
    }
    taken
}

/// Count an outcome, and log it when the count is worth a line.
fn record(counters: &Counters, method: ClientMethod, handler: &str, outcome: &Delivery) {
    let name = method.name();
    match outcome {
        Delivery::Delivered => {
            let n = bump(&counters.delivered);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "delivered {name} to {}.{handler} (#{n})",
                    plan::TABLE
                ));
            }
        }
        Delivery::NoOverlay => {
            let n = bump(&counters.dropped_no_overlay);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "{name} dropped: the global {} is not defined, so the UI overlay is not installed (#{n})",
                    plan::TABLE
                ));
            }
        }
        Delivery::NoHandler => {
            let n = bump(&counters.dropped_no_handler);
            if is_log_worthy(n) {
                log::line(format_args!(
                    "{name} dropped: {}.{handler} is not a function (#{n})",
                    plan::TABLE
                ));
            }
        }
        Delivery::NoStackSpace => {
            let n = bump(&counters.handler_failed);
            if is_log_worthy(n) {
                log::line(format_args!("{name} dropped: no Lua stack space (#{n})"));
            }
        }
        Delivery::SetupError { status, message } => {
            let n = bump(&counters.handler_failed);
            if is_log_worthy(n) {
                let short: String = message.chars().take(200).collect();
                log::line(format_args!(
                    "{name} dropped: setting up the Lua call raised an error (status {status}): {short} (#{n})"
                ));
            }
        }
        Delivery::HandlerError { status, message } => {
            let n = bump(&counters.handler_failed);
            if is_log_worthy(n) {
                let short: String = message.chars().take(200).collect();
                log::line(format_args!(
                    "{}.{handler} raised an error (status {status}): {short} (#{n})",
                    plan::TABLE
                ));
            }
        }
    }
}
