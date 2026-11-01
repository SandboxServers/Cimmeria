//! `watch_start` / `watch_read` / `watch_stop` and the per-frame sampler
//! (lab autopilot contract C6).
//!
//! Read-only: a sample is built from the existing guarded `Mem` readers, so
//! a stale pointer omits its field instead of faulting. One watch at a
//! time. [`tick`] runs on the main thread from the Tick hub; [`disconnect`]
//! runs on the bridge IO thread, so the state sits behind a `Mutex` and the
//! tick side only ever `try_lock`s: the game thread never waits on it.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{RpcResponse, INVALID_PARAMS};
use super::tick::WATCH_ACTIVE;
use super::watch_ring::{Ring, Sample};
use crate::hooks::ability_trace::clock::game_time;
use crate::hooks::camera::layout::{
    GLEVEL_PTR_VA, LEVEL_ACTORS_OFF, PLAYER_CONTROLLER_VTABLE_VA, WORLD_LEVEL_OFF,
};
use crate::hooks::camera::read::{find_player_controller, read_pawn_pose};
use crate::hooks::entity_trace::map::Mem;
use crate::hooks::nav::controller::read_controller;

/// The one watch. `None` before the first start and after a disconnect.
static WATCH: Mutex<Option<Ring>> = Mutex::new(None);

/// The controller found by the last scan. Re-scanning the Actors array costs
/// a 32 KB read, so it is re-checked by vtable each frame and re-scanned only
/// when that check fails.
static PC_CACHE: AtomicU32 = AtomicU32::new(0);

const DEFAULT_PERIOD: u64 = 1;
const DEFAULT_CAP: usize = 1024;
const DEFAULT_MAX: usize = 256;
const MAX_READ: usize = 1024;

#[derive(Debug, Default, Deserialize)]
struct StartParams {
    period_frames: Option<u64>,
    cap: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
struct ReadParams {
    since: Option<u64>,
    max: Option<usize>,
}

/// `null` params mean "all defaults", like `{}`.
fn parse<T: for<'de> Deserialize<'de> + Default>(params: &Value) -> Result<T, serde_json::Error> {
    if params.is_null() {
        Ok(T::default())
    } else {
        serde_json::from_value(params.clone())
    }
}

fn lock() -> MutexGuard<'static, Option<Ring>> {
    WATCH.lock().unwrap_or_else(|e| e.into_inner())
}

/// Handle `watch_start` (contract C6).
pub fn dispatch_start(id: Value, params: &Value) -> RpcResponse {
    let p: StartParams = match parse(params) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("watch_start: {e}")),
    };
    let frame = super::heartbeat_count();
    let ring = Ring::new(
        p.period_frames.unwrap_or(DEFAULT_PERIOD),
        p.cap.unwrap_or(DEFAULT_CAP),
        frame,
    );
    let replaced = {
        let mut g = lock();
        let replaced = g.replace(ring).is_some_and(|old| old.running);
        PC_CACHE.store(0, Ordering::Relaxed);
        WATCH_ACTIVE.store(true, Ordering::SeqCst);
        replaced
    };
    RpcResponse::ok(id, json!({ "started_frame": frame, "replaced": replaced }))
}

/// Handle `watch_read` (contract C6); moves no cursor.
pub fn dispatch_read(id: Value, params: &Value) -> RpcResponse {
    let p: ReadParams = match parse(params) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("watch_read: {e}")),
    };
    let since = p.since.unwrap_or(0);
    let max = p.max.unwrap_or(DEFAULT_MAX).clamp(1, MAX_READ);
    let guard = lock();
    let result = match guard.as_ref() {
        Some(r) => {
            let (samples, next) = r.read(since, max);
            json!({ "samples": samples, "next": next, "dropped": r.dropped(), "running": r.running })
        }
        None => json!({ "samples": [], "next": since, "dropped": 0, "running": false }),
    };
    RpcResponse::ok(id, result)
}

/// Handle `watch_stop` (contract C6); keeps the ring readable.
pub fn dispatch_stop(id: Value) -> RpcResponse {
    let mut guard = lock();
    WATCH_ACTIVE.store(false, Ordering::SeqCst);
    let result = match guard.as_mut() {
        Some(r) => {
            // Kept, not dropped: the caller can still read what was taken.
            r.running = false;
            json!({ "samples_taken": r.taken(), "dropped": r.dropped() })
        }
        None => json!({ "samples_taken": 0, "dropped": 0 }),
    };
    RpcResponse::ok(id, result)
}

/// Main-thread, once per Tick while the watch is active.
pub(crate) fn tick() {
    let Ok(mut guard) = WATCH.try_lock() else {
        return; // never block the game thread; the next frame samples
    };
    let Some(ring) = guard.as_mut().filter(|r| r.running) else {
        return;
    };
    let frame = super::heartbeat_count();
    if !ring.due(frame) {
        return;
    }
    ring.push(live_sample(frame, super::meta::mono_ms()));
}

/// IO thread, when the daemon connection ends.
pub(crate) fn disconnect() {
    // Flag and ring change under one lock hold; `tick` only `try_lock`s, so the
    // game thread never waits on this.
    let mut guard = lock();
    WATCH_ACTIVE.store(false, Ordering::SeqCst);
    *guard = None;
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn live_sample(frame: u64, ms: u64) -> Sample {
    sample(frame, ms, &crate::hooks::entity_trace::map::LiveMem)
}

/// Host builds read no game memory: the sample carries the stamp only.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
fn live_sample(frame: u64, ms: u64) -> Sample {
    struct NoMem;
    impl Mem for NoMem {
        fn u32_at(&self, _addr: u32) -> Option<u32> {
            None
        }
    }
    sample(frame, ms, &NoMem)
}

/// Slot count of the persistent level's Actors array: `[[GWorld+0x50]+0x40]`
/// (AX-01 `actor_count`: confirmed). Persistent level only, null slots
/// included, so consumers must treat it as a lower bound.
fn actor_count(mem: &impl Mem) -> Option<u32> {
    let world = mem.u32_at(GLEVEL_PTR_VA).filter(|&w| w != 0)?;
    let level = mem
        .u32_at(world.wrapping_add(WORLD_LEVEL_OFF))
        .filter(|&l| l != 0)?;
    mem.u32_at(level.wrapping_add(LEVEL_ACTORS_OFF).wrapping_add(4))
}

fn player_controller(mem: &impl Mem) -> Option<u32> {
    let cached = PC_CACHE.load(Ordering::Relaxed);
    if cached != 0 && mem.u32_at(cached) == Some(PLAYER_CONTROLLER_VTABLE_VA) {
        return Some(cached);
    }
    let found = find_player_controller(mem);
    PC_CACHE.store(found.unwrap_or(0), Ordering::Relaxed);
    found
}

/// Build one sample. Each reader that returns `None` leaves its field absent.
pub(crate) fn sample(frame: u64, ms: u64, mem: &impl Mem) -> Sample {
    let mut s = Sample::bare(frame, ms);
    s.t = game_time(mem);
    if let Some(pose) = read_pawn_pose(mem) {
        s.pos = Some(pose.pos);
        s.rot = Some([pose.pitch, pose.yaw, 0]);
    }
    if let Some(c) = player_controller(mem).and_then(|pc| read_controller(mem, pc)) {
        s.hover = Some(c.hover_id as u32);
        s.mode = Some(c.mode);
        s.buttons = Some(c.buttons);
    }
    s.actors = actor_count(mem);
    s
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
