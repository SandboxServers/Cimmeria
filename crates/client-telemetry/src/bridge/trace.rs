//! `trace_line`: one collision line trace through `UWorld::SingleLineCheck`
//! (lab autopilot contract C14, DLL half).
//!
//! The one bridge method that calls an engine function. It reads collision
//! and writes only the caller's stack buffer (the `FCheckResult`), which is
//! the stated exception to C13 item 6.
//!
//! Gate: [`dispatch`] runs inside the main-thread drain, which only runs from
//! the `FEngineLoop::Tick` hook. `boot.rs` `gate_and_install` returns before
//! `hooks::install_all` when the fingerprint gate is not usable, so the Tick
//! hook, and with it this code, never runs on an unfingerprinted build.
//!
//! Latch: `SingleLineCheck` reaches `MultiLineCheck` `0x008721B0`, which
//! pushes a `GMem` stack mark. An access violation mid-call can leave the
//! mark unrestored, so the first fault latches the method off for the rest of
//! the client session. Only a client restart clears it.

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};

use super::dispatch::{RpcResponse, INVALID_PARAMS};
use super::seh::SehFault;
use crate::hooks::camera::layout::{
    ENTITY_MANAGER_VA, ENTITY_PAWN_OFF, GLEVEL_PTR_VA, MANAGER_LOCAL_ENTITY_OFF,
};
use crate::hooks::entity_trace::map::Mem;

/// `-32000`: the trace cannot run now (the same code `call_native` style
/// refusals use for "unavailable").
pub const UNAVAILABLE: i32 = -32000;

/// `UWorld::SingleLineCheck`, 7 stack arguments, `ret 0x1c` (live L1).
#[cfg(all(target_arch = "x86", windows))]
const SINGLE_LINE_CHECK_VA: usize = 0x0087_5970;
/// `UWorld` vtable (live L7: `GWorld` is class `World` with this vtable).
pub(crate) const UWORLD_VTABLE_VA: u32 = 0x0183_8E7C;

/// Level geometry only. The default.
pub const FLAGS_LEVEL: u32 = 0x2086;
/// Level geometry and pawns.
pub const FLAGS_PAWNS: u32 = 0x20bf;

const CHECK_RESULT_LEN: usize = 0x48;
const OFF_ACTOR: usize = 0x04;
const OFF_LOCATION: usize = 0x08;
const OFF_NORMAL: usize = 0x14;
const OFF_TIME: usize = 0x20;
const OFF_ITEM: usize = 0x24;
const OFF_LEVEL_INDEX: usize = 0x40;

/// Set by the first faulting call; cleared only by a client restart.
pub(crate) static TRACE_LATCHED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(crate) fn reset_latch() {
    TRACE_LATCHED.store(false, Ordering::SeqCst);
}

/// A parsed request.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceReq {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub flags: u32,
}

/// The engine call, behind a seam: `(world, hit, source, end, start, flags)`.
/// Extent is always zero and the trailing argument always 0.
pub type Native<'a> =
    dyn Fn(u32, *mut u8, u32, &[f32; 3], &[f32; 3], u32) -> Result<i32, SehFault> + 'a;

fn vec3(v: Option<&Value>) -> Option<[f32; 3]> {
    let a = v?.as_array()?;
    if a.len() != 3 {
        return None;
    }
    let mut out = [0f32; 3];
    for (o, x) in out.iter_mut().zip(a) {
        let f = x.as_f64()? as f32;
        if !f.is_finite() {
            return None;
        }
        *o = f;
    }
    Some(out)
}

/// Validate params. The flags rule is the whole safety story: only the two
/// values proven live (L2) are accepted; `0x3fff` access-violated.
pub fn parse(params: &Value) -> Result<TraceReq, String> {
    let shape = "trace_line: start and end must be [x, y, z]".to_string();
    let (Some(start), Some(end)) = (vec3(params.get("start")), vec3(params.get("end"))) else {
        return Err(shape);
    };
    let flags = match params.get("flags") {
        None | Some(Value::Null) => FLAGS_LEVEL,
        Some(v) => match v.as_u64().and_then(|f| u32::try_from(f).ok()) {
            Some(f @ (FLAGS_LEVEL | FLAGS_PAWNS)) => f,
            _ => return Err("trace_line: flags must be 0x2086 or 0x20bf".to_string()),
        },
    };
    Ok(TraceReq { start, end, flags })
}

/// The initial `FCheckResult`: `Time` 1.0, `Item` -1, `LevelIndex` -1.
pub fn init_check_result() -> [u8; CHECK_RESULT_LEN] {
    let mut b = [0u8; CHECK_RESULT_LEN];
    b[OFF_TIME..OFF_TIME + 4].copy_from_slice(&1.0f32.to_le_bytes());
    b[OFF_ITEM..OFF_ITEM + 4].copy_from_slice(&(-1i32).to_le_bytes());
    b[OFF_LEVEL_INDEX..OFF_LEVEL_INDEX + 4].copy_from_slice(&(-1i32).to_le_bytes());
    b
}

fn f32_at(b: &[u8; CHECK_RESULT_LEN], off: usize) -> f32 {
    f32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn u32_at(b: &[u8; CHECK_RESULT_LEN], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn v3_at(b: &[u8; CHECK_RESULT_LEN], off: usize) -> [f32; 3] {
    [f32_at(b, off), f32_at(b, off + 4), f32_at(b, off + 8)]
}

/// Turn the return value and buffer into the result. `SingleLineCheck`
/// returns true (1) when nothing was hit, so `hit = (ret == 0)`.
pub fn decode(ret: i32, buf: &[u8; CHECK_RESULT_LEN]) -> Value {
    let time = f32_at(buf, OFF_TIME);
    if ret != 0 {
        return json!({ "hit": false, "time": time });
    }
    json!({
        "hit": true,
        "time": time,
        "location": v3_at(buf, OFF_LOCATION),
        "normal": v3_at(buf, OFF_NORMAL),
        "actor": u32_at(buf, OFF_ACTOR),
        "item": u32_at(buf, OFF_ITEM) as i32,
    })
}

/// The local player's pawn, through the chain `read_pawn_pose` walks
/// (`hooks/camera/read.rs`). `None` when any link is null or unreadable.
pub fn pawn_actor(mem: &dyn Mem) -> Option<u32> {
    let nonnull = |v: u32| (v != 0).then_some(v);
    let manager = nonnull(mem.u32_at(ENTITY_MANAGER_VA)?)?;
    let entity = nonnull(mem.u32_at(manager.wrapping_add(MANAGER_LOCAL_ENTITY_OFF))?)?;
    nonnull(mem.u32_at(entity.wrapping_add(ENTITY_PAWN_OFF))?)
}

fn unavailable(id: Value, reason: &str) -> RpcResponse {
    RpcResponse::error(id, UNAVAILABLE, format!("trace_line unavailable: {reason}"))
}

/// Handle `trace_line` against the live client.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    #[cfg(all(target_arch = "x86", windows))]
    {
        dispatch_with(
            id,
            params,
            &crate::hooks::entity_trace::map::LiveMem,
            &real_native,
        )
    }
    #[cfg(not(all(target_arch = "x86", windows)))]
    {
        // Host builds read no game memory, so this always answers "world not
        // ready"; the native is never reached.
        struct NoMem;
        impl Mem for NoMem {
            fn u32_at(&self, _addr: u32) -> Option<u32> {
                None
            }
        }
        dispatch_with(id, params, &NoMem, &|_, _, _, _, _, _| Ok(1))
    }
}

/// Order of checks: parse, latch, world and vtable, then the call. (The
/// fingerprint gate is upstream: see the module docs.)
pub fn dispatch_with(id: Value, params: &Value, mem: &dyn Mem, native: &Native<'_>) -> RpcResponse {
    let req = match parse(params) {
        Ok(r) => r,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, e),
    };
    // debug_assert: the drain that called us runs only from the Tick hook,
    // which is installed only after the fingerprint gate passed.
    if TRACE_LATCHED.load(Ordering::SeqCst) {
        return RpcResponse::error_with_data(
            id,
            UNAVAILABLE,
            "trace_line unavailable: faulted earlier; restart the client",
            json!({ "trace_latched": true }),
        );
    }
    let world = match mem.u32_at(GLEVEL_PTR_VA) {
        Some(w) if w != 0 && mem.u32_at(w) == Some(UWORLD_VTABLE_VA) => w,
        _ => return unavailable(id, "world not ready"),
    };
    let source = pawn_actor(mem).unwrap_or(0);
    let mut buf = init_check_result();
    // No lock is held across the guarded call (seh.rs drop-across-unwind rule).
    match native(
        world,
        buf.as_mut_ptr(),
        source,
        &req.end,
        &req.start,
        req.flags,
    ) {
        Ok(ret) => RpcResponse::ok(id, decode(ret, &buf)),
        Err(fault) => {
            TRACE_LATCHED.store(true, Ordering::SeqCst);
            RpcResponse::error_with_data(
                id,
                UNAVAILABLE,
                fault.message(),
                fault.to_exception_json(),
            )
        }
    }
}

#[cfg(all(target_arch = "x86", windows))]
fn real_native(
    world: u32,
    hit: *mut u8,
    source: u32,
    end: &[f32; 3],
    start: &[f32; 3],
    flags: u32,
) -> Result<i32, SehFault> {
    let extent = [0f32; 3];
    let (end, start, extent_p) = (
        end as *const [f32; 3],
        start as *const [f32; 3],
        &extent as *const [f32; 3],
    );
    super::seh::guard(|| {
        // SAFETY: `SINGLE_LINE_CHECK_VA` is fingerprint-gated engine code
        // (see module docs); the signature is live-verified (L1: 7 stack
        // arguments, `ret 0x1c`). `hit` is a 0x48-byte buffer owned by the
        // caller and all pointers outlive the call. A fault is absorbed by
        // the guard and latches the method off.
        unsafe {
            let f: unsafe extern "thiscall" fn(
                u32,
                *mut u8,
                u32,
                *const [f32; 3],
                *const [f32; 3],
                u32,
                *const [f32; 3],
                u32,
            ) -> i32 = core::mem::transmute(SINGLE_LINE_CHECK_VA);
            f(world, hit, source, end, start, flags, extent_p, 0)
        }
    })
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod tests;
