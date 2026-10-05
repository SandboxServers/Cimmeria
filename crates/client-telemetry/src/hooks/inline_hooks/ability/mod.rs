//! The ability press chain, the router's send decision and the Mercury
//! sequence join: `client.ability.press`, `.press_dropped`, `.sent` and
//! `.sent_seq` (AB-C1, AB-C2).
//!
//! Anchors: `docs/reverse-engineering/findings/ability-client-hook-anchors.md`
//! (static, Ghidra 2026-10-04), plus the GamePet send `0x00d3a820` and the
//! two `start*Message` functions, read from the QA image for this packet.
//! None of these hooks has run in a live client yet: every anchor's live
//! status is UNVERIFIED in `docs/architecture/client-telemetry.md`.
//!
//! | Hook | Address | Signature | Role |
//! |---|---|---|---|
//! | `useAction` tolua thunk | `0x00aa94e0` | `cdecl int(lua_State*)` | starts a `hotbar` press; no next step = `bad_args` |
//! | `useAbility` tolua thunk | `0x00aa2910` | `cdecl int(lua_State*)` | starts a `lua` press; no next step = `bad_args` |
//! | `FUN_00ad9580` | `0x00ad9580` | `cdecl void(actionId, self)` | slot; empty slot = `no_action` |
//! | `FUN_00d2afc0` | `0x00d2afc0` | `thiscall(set, abilityId, targetId)`, `ret 8` | press row; no send = `not_known` |
//! | `FUN_00d2ae40` | `0x00d2ae40` | `thiscall(set, record, targetId)`, `ret 8` | posted (or ground reticle): pending send |
//! | `PetAbilityAction::execute` | `0x00e3cf40` | `thiscall(action, self)`, `ret 4` | pet press row; no pet send = `pet_missing` |
//! | GamePet send | `0x00d3a820` | `thiscall(pet, abilityId, targetId)`, `ret 8` | three gates, else pending `petInvokeAbility` |
//! | `startEntityMessage` | `0x00dd6a60` | `thiscall(conn, msgId, entityId)`, `ret 8` | marks the router's send as reached |
//! | `startProxyMessage` | `0x00dd6980` | `thiscall(conn, msgId)`, `ret 4` | same, base route |
//! | `Channel::send` | `0x01576f90` | `thiscall(channel) -> int` | tags the detached bundle |
//! | `Nub::send` | `0x01582160` | `thiscall(nub, addr, bundle, channel)`, `ret 0xc` | claims the tag, collects the sequence numbers |
//! | reliable sequence counter | `0x0158bb40` | `thiscall(channelInternal) -> u32` | one value per packet |
//!
//! The router itself (`0x00c6fc40`) is already hooked by [`super::net_out`],
//! which calls [`route::RouteProbe`] around the original. The third router
//! exit, `0x00dd8010`, is a 15-byte wrapper that tail-calls
//! `startEntityMessage`, so it needs no hook of its own.
//!
//! Called, not hooked (and therefore in the fingerprint gate too): the
//! event-bag readers `GetInt` `0x00e3cba0`, `GetFloat` `0x00e3cc20` and
//! `GetByte` `0x00d434d0`.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::hooks::ability_trace::press::{PendingTable, Press, Source};
use crate::hooks::ability_trace::seq_join::Joiner;
use crate::hooks::ability_trace::{self as at, now_ms};
use crate::queue::Producer;

mod press_detours;
pub(super) mod route;
mod seq;

#[cfg(test)]
pub(super) use press_detours::{
    ADDR_GAME_PET_SEND, ADDR_LOOKUP, ADDR_PET_ACTION_EXECUTE, ADDR_SEND_BUILDER, ADDR_SLOT,
    ADDR_USE_ABILITY_THUNK, ADDR_USE_ACTION_THUNK,
};
#[cfg(test)]
pub(super) use route::{
    ADDR_GET_BYTE, ADDR_GET_FLOAT, ADDR_GET_INT, ADDR_START_ENTITY_MESSAGE,
    ADDR_START_PROXY_MESSAGE,
};
#[cfg(test)]
pub(super) use seq::{ADDR_CHANNEL_SEND, ADDR_NUB_SEND, ADDR_SEQ_NEXT};

/// Hooks this module installs.
pub(super) const HOOK_COUNT: u32 = 12;

pub(super) unsafe fn install_all(producer: &Producer) {
    unsafe {
        press_detours::install_all(producer);
        route::install_all(producer);
        seq::install_all(producer);
    }
}

// ---------------------------------------------------------------------
// Shared state

static NEXT_PRESS_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_SEND_ID: AtomicU32 = AtomicU32::new(1);

/// Posted presses waiting for the router (main thread both sides).
static PENDING: Mutex<Option<PendingTable>> = Mutex::new(None);
/// Sends waiting for their packets (router and `Channel::send` on the
/// main thread, `Nub::send` on the network thread).
static JOINER: Mutex<Option<Joiner>> = Mutex::new(None);

fn with_pending<R>(f: impl FnOnce(&mut PendingTable) -> R) -> R {
    let mut g = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(PendingTable::default))
}

fn with_joiner<R>(f: impl FnOnce(&mut Joiner) -> R) -> R {
    let mut g = JOINER.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Joiner::default))
}

fn next_send_id() -> u32 {
    NEXT_SEND_ID.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    /// The press the current thread's Lua binding started.
    static PRESS: Cell<Option<Press>> = const { Cell::new(None) };
}

/// Run `f` on the current press, if there is one, and store it back.
fn with_press<R>(f: impl FnOnce(&mut Press) -> R) -> Option<R> {
    PRESS
        .try_with(|c| {
            let mut p = c.get()?;
            let r = f(&mut p);
            c.set(Some(p));
            Some(r)
        })
        .ok()
        .flatten()
}

/// Catch a Rust panic so it never unwinds into the game.
fn guarded<R>(f: impl FnOnce() -> R) -> Option<R> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

/// Report events, swallowing a panic.
fn report(outs: Vec<at::Out>) {
    if !outs.is_empty() {
        let _ = guarded(|| at::report(outs));
    }
}

/// One Lua binding call. Restores the previous press when dropped, which
/// also runs when the binding raises a Lua error (a C++ exception) through
/// the detour: that is how `bad_args` is seen.
struct PressScope {
    prev: Option<Press>,
}

impl PressScope {
    fn begin(source: Source) -> Self {
        let id = NEXT_PRESS_ID.fetch_add(1, Ordering::Relaxed);
        let expired = guarded(|| with_pending(|t| t.take_expired(now_ms()))).unwrap_or(0);
        let press = Press::begin(source, id, expired);
        Self {
            prev: PRESS.try_with(|c| c.replace(Some(press))).ok().flatten(),
        }
    }
}

impl Drop for PressScope {
    fn drop(&mut self) {
        let outs = guarded(|| with_press(Press::thunk_left))
            .flatten()
            .unwrap_or_default();
        let _ = PRESS.try_with(|c| c.set(self.prev));
        report(outs);
    }
}

/// Install one hook through the shared MinHook plumbing.
unsafe fn install(
    producer: &Producer,
    name: &'static str,
    address: usize,
    detour: *mut c_void,
    slot: &OnceLock<usize>,
) {
    unsafe { super::install_one(producer, name, address, detour, slot) }
}

#[cfg(test)]
mod tests;
