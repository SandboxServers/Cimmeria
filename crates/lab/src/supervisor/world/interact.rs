//! `client_interact` (N3): interact with an entity by id, with no screen pick.
//!
//! The stock right-click path is `FUN_00e84b20` (mouse handler) -> resolver
//! `FUN_00e84860` (screen pick -> entity id) -> firer `FUN_00def4b0`. The
//! firer is `__stdcall(uint32 entity_id)`: it builds `Event_NetOut_Interact`
//! with the "OverrideTarget" property and sends it. Calling it directly with
//! an entity id therefore sends exactly the packet a right-click would, for
//! targets whose pixels the hover search cannot hit (a prop behind scenery,
//! a tiny vial, a ring control). Only the pick is skipped.
//!
//! Tier: `native_call` (N3), never a native pass. What the server did with
//! the interact is graded separately (server log, packet tap), so the
//! evidence here is inconclusive by design; it is denied when the avatar is
//! beyond the server's interact range, because the server ignores that.

use rmcp::schemars;
use serde_json::{json, Value};

use super::click_range::{range_json, INTERACT_RANGE_M};
use super::geometry::distance_m;
use super::io::WorldIo;
use super::{NativeLevel, Steps, WorldError};
use crate::proof::{Check, Evidence, Source, Why};
use crate::supervisor::cegui_native::rebase;
use crate::supervisor::Supervisor;

/// `FUN_00def4b0`, the interact firer (unslid VA of the shipped SGW.exe).
pub const INTERACT_FIRER_VA: u32 = 0x00de_f4b0;

#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct InteractRequest {
    /// Client entity id to interact with (from `client_entity_find` or the
    /// server tools). Must be in the client's entity map.
    pub entity_id: u32,
}

/// `call_native` params for the firer: `stdcall`, one word argument (the callee
/// pops it, `ret 4`). The decompiler narrows the parameter to `char`, but the
/// stock caller pushes the full 32-bit id, so the full word is passed here too.
pub fn interact_params(slide: i64, entity_id: u32) -> Value {
    json!({
        "addr": format!("{:#x}", rebase(INTERACT_FIRER_VA, slide)),
        "conv": "stdcall",
        "args": [format!("{entity_id:#x}")],
        "ret": "void",
    })
}

/// The live call: one journaled, main-thread, SEH-guarded `call_native`.
pub(super) async fn live_fire(sup: &Supervisor, slide: i64, entity_id: u32) -> Result<(), String> {
    let v = sup
        .bridge_call("call_native", interact_params(slide, entity_id))
        .await
        .map_err(|e| format!("interact call: {e}"))?;
    if v.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(format!("interact call: {v}"));
    }
    Ok(())
}

/// The checks behind the verdict. `range_m` is the straight-line distance
/// when both poses were readable.
pub(super) fn checks(range_m: Option<f64>) -> Vec<Check> {
    let mut out = Vec::new();
    if let Some(d) = range_m {
        let in_range = d <= INTERACT_RANGE_M;
        let mut c = Check::new("in_range", Source::Client, Some(in_range), !in_range)
            .with_before(json!(INTERACT_RANGE_M))
            .with_after(json!((d * 100.0).round() / 100.0));
        if !in_range {
            c = c.with_why(Why::OutOfRange);
        }
        out.push(c);
    }
    // The server's reaction is not read here; the caller grades it.
    out.push(Check::new("post_read", Source::Client, None, true).with_why(Why::NoPostRead));
    out
}

pub async fn run<W: WorldIo>(io: &mut W, a: InteractRequest) -> Result<Value, WorldError> {
    let mut steps = Steps::new("client_interact", io.now_ms());
    steps.subject(json!({ "entity_id": a.entity_id }));
    if a.entity_id == 0 {
        return Err(steps.fail(io.now_ms(), "args", "entity_id must be non-zero"));
    }
    // 1. The entity must be one the client knows, and not the avatar (the
    // stock interact skips the player's own id).
    let t0 = io.now_ms();
    let snap = io
        .snapshot()
        .await
        .map_err(|e| steps.fail(io.now_ms(), "resolve", e))?;
    if a.entity_id == snap.player_id {
        return Err(steps.fail(
            io.now_ms(),
            "resolve",
            "entity_id is the player's own id; the stock interact skips it",
        ));
    }
    let Some(entity) = snap.entity(a.entity_id) else {
        return Err(steps.fail(
            io.now_ms(),
            "resolve",
            format!("entity {} is not in the client's entity map", a.entity_id),
        ));
    };
    let range_m = match (
        snap.entity(snap.player_id).and_then(|p| p.pose),
        entity.pose,
    ) {
        (Some(p), Some(e)) => Some(distance_m(p.pos, e.pos)),
        _ => None,
    };
    steps.record(
        "resolve",
        t0,
        io.now_ms(),
        json!({ "actor": entity.actor, "rendered": entity.rendered_flag, "range": range_json(range_m) }),
    );
    // Beyond the server's range it ignores the packet, so do not fire at all.
    if range_m.is_some_and(|d| d > INTERACT_RANGE_M) {
        steps.judged(Evidence::judge(checks(range_m)));
        let out = json!({
            "entity_id": a.entity_id,
            "fired": false,
            "range": range_json(range_m),
            "note": "not fired: beyond the server's interact range; stand within 5 m first (client_move_to)",
        });
        return Ok(steps.finish(io.now_ms(), out));
    }
    // 2. Fire the interact.
    let t1 = io.now_ms();
    steps.used(NativeLevel::NativeCall);
    io.fire_interact(a.entity_id)
        .await
        .map_err(|e| steps.fail(io.now_ms(), "fire", e))?;
    steps.record(
        "fire",
        t1,
        io.now_ms(),
        json!({ "fn_va": format!("{INTERACT_FIRER_VA:#x}") }),
    );
    steps.judged(Evidence::judge(checks(range_m)));
    let out = json!({
        "entity_id": a.entity_id,
        "fired": true,
        "range": range_json(range_m),
        "note": "pick skipped; the server's reaction is graded separately",
    });
    Ok(steps.finish(io.now_ms(), out))
}

#[cfg(test)]
#[path = "interact_tests.rs"]
mod tests;
