//! `client.ability.applied`: what the client did with an ability message
//! (AB-C4). Each detour observes a stock handler on the main thread and
//! the calls it makes; `hooks::ability_trace::applied` turns that into the
//! event.
//!
//! | Function | Address | Signature | Role |
//! |---|---|---|---|
//! | `EffectSet` timer handler | `0x00e09160` | `thiscall(this, event, subject)`, `ret 8` | effect-bar add / refresh / clear |
//! | `EffectSet` entry lookup | `0x00e08570` | `thiscall(this, int secondary_id) -> entry*`, `ret 4` | probe: did an entry exist |
//! | effect-bar announce | `0x00e0a9e0` | `thiscall(ui, int* id)`, `ret 4` | probe: a new entry was announced |
//! | effect display-data request | `0x00e0a810` | `thiscall(ui, int* id)`, `ret 4` | probe: the display data was missing |
//! | effect-bar post to the UI | `0x00e0a2d0` | `thiscall(ui, int* id, record*)`, `ret 8` | probe: the add was posted |
//! | `CooldownManager` timer handler | `0x00ea6af0` | `thiscall(this, event, subject)`, `ret 8` | cooldown on a hotbar button |
//! | cooldown button callback | `0x00ea62b0` | `thiscall(this, type, id, float, float)`, `ret 0x10` | probe: a button took it |
//! | `GameBeing` stat handler | `0x00e01f40` | `thiscall(this, event, subject)`, `ret 8` | scope for the current stats |
//! | `GameBeing` base-stat handler | `0x00e02060` | same | scope for the base stats |
//! | current-stat functor | `0x00e004e0` | `thiscall(this, StatId, Max, Min, Current)`, `ret 0x10` | one stat stored |
//! | base-stat functor | `0x00e005b0` | same | one base stat stored |
//!
//! The state-flag row comes from the existing `onStateFieldUpdate` hook
//! (`super::state_flags`). Every address, argument count and `ret` was
//! read from the QA `SGW.exe` on 2026-10-04 (disassembly, prologues in the
//! fingerprint gate); none of it has run in a live client.
//!
//! The handlers' fields are read through the game's own getters
//! (`ability_trace::event_bag`) before the original runs; the probes only
//! note that they were called. Nothing is written.

mod cooldown;
mod effect_bar;
mod stats;

use std::cell::Cell;

use crate::hooks::ability_trace::{admit, applied::TARGET_APPLIED, clock};
use crate::hooks::emit::emit;
use crate::hooks::entity_trace::{
    map::{LiveMem, Mem},
    Fields,
};
use crate::queue::Producer;

#[cfg(test)]
pub(super) use cooldown::{ADDR_COOLDOWN_TIMER, ADDR_COOLDOWN_UI};
#[cfg(test)]
pub(super) use effect_bar::{
    ADDR_EFFECT_ANNOUNCE, ADDR_EFFECT_DATA_REQUEST, ADDR_EFFECT_LOOKUP, ADDR_EFFECT_POST,
    ADDR_EFFECT_TIMER,
};
#[cfg(test)]
pub(super) use stats::{
    ADDR_STAT_BASE_FUNCTOR, ADDR_STAT_BASE_HANDLER, ADDR_STAT_FUNCTOR, ADDR_STAT_HANDLER,
};

/// Hooks [`install_all`] installs.
pub(super) const HOOK_COUNT: u32 = 11;

/// Install the eleven hooks.
pub(super) unsafe fn install_all(producer: &Producer) {
    unsafe {
        effect_bar::install_all(producer);
        cooldown::install_all(producer);
        stats::install_all(producer);
    }
}

/// Restores a thread-local on drop, including when a C++ exception unwinds
/// through the detour that set it.
pub(super) struct Restore<T: Copy + 'static> {
    key: &'static std::thread::LocalKey<Cell<T>>,
    prev: T,
}

impl<T: Copy + 'static> Restore<T> {
    pub(super) fn set(key: &'static std::thread::LocalKey<Cell<T>>, v: T) -> Self {
        Self {
            key,
            prev: key.with(|c| c.replace(v)),
        }
    }
}

impl<T: Copy + 'static> Drop for Restore<T> {
    fn drop(&mut self) {
        let prev = self.prev;
        self.key.with(|c| c.set(prev));
    }
}

/// The game clock now, if the connection exists.
pub(super) fn now() -> Option<f64> {
    clock::game_time(&LiveMem)
}

/// An `i32` field of a game object.
pub(super) fn i32_at(base: *mut std::ffi::c_void, offset: u32) -> Option<i32> {
    LiveMem
        .u32_at((base as u32).wrapping_add(offset))
        .map(|v| v as i32)
}

/// Whether `id` is the local player's entity id (the entity manager's
/// player entity, `[[0x01ef244c]+0xc]+0xc`).
pub(super) fn is_local_player(id: Option<i32>) -> bool {
    use crate::hooks::entity_trace::{map, sequences::layout};
    let local = LiveMem
        .u32_at(layout::ENTITY_MANAGER_SINGLETON)
        .filter(|&m| m != 0)
        .and_then(|m| LiveMem.u32_at(m.wrapping_add(map::manager::LOCAL_PLAYER_ENTITY)))
        .filter(|&p| p != 0)
        .and_then(|p| LiveMem.u32_at(p.wrapping_add(map::entity::ID)));
    id.is_some_and(|id| local == Some(id as u32))
}

/// Emit one applied event through the per-name bucket `key`.
pub(super) fn report(key: &str, level: &'static str, fields: Fields) {
    if let Some(f) = admit(key, || fields) {
        emit(TARGET_APPLIED, level, f);
    }
}
