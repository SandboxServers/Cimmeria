//! `client_batch` in one client frame. When every step is a literal
//! `mem_read` or `lua` (no `$` reference), the lab DLL's `batch` method
//! (C7) runs them all in one main-thread frame and reports that frame, so
//! the reads see one consistent game state. Anything else takes the step
//! path in [`super::batch`]. An old DLL that does not know `batch` answers
//! "method not found"; the supervisor then stops asking and uses the step
//! path for the rest of its life.

use std::sync::atomic::Ordering;
use std::time::Instant;

use serde_json::{json, Map, Value};

use super::batch::{addr_text, decode_read, resolve, BatchOutcome, Op, Step};
use crate::supervisor::flows::ui_state::lua_results_of;
use crate::supervisor::Supervisor;

/// Most steps one DLL batch takes (the step path allows more).
const DLL_MAX_STEPS: usize = 16;

/// Whether the DLL can run `steps` in one frame: 1 to 16 steps, each a
/// `mem_read` or `lua` whose fields hold no reference.
pub fn dll_batchable(steps: &[Step]) -> bool {
    (1..=DLL_MAX_STEPS).contains(&steps.len())
        && steps.iter().all(|s| match &s.op {
            Op::MemRead { addr, .. } => resolve(addr, &Map::new()).is_ok(),
            Op::Lua { chunk } => resolve(&json!(chunk), &Map::new()).is_ok(),
            _ => false,
        })
}

/// The C7 `calls` array for `steps`, with the same `len` rule as the step
/// path. Errors on an address the bridge cannot take.
pub fn dll_calls(steps: &[Step]) -> Result<Value, String> {
    steps
        .iter()
        .map(|s| match &s.op {
            Op::MemRead { addr, len, read_as } => {
                let len = len.unwrap_or(read_as.width()).clamp(1, 65536);
                Ok(json!({
                    "method": "mem_read",
                    "params": { "addr": addr_text(addr)?, "len": len },
                }))
            }
            Op::Lua { chunk } => Ok(json!({
                "method": "lua_eval",
                "params": { "chunk": chunk },
            })),
            _ => Err("only mem_read and lua run in the DLL batch".to_string()),
        })
        .collect::<Result<Vec<_>, String>>()
        .map(Value::Array)
}

/// Decode the DLL's per-step `results` into step values, as the step path
/// does: an `{"ok": v}` entry decodes `v`, an `{"error": {...}}` entry
/// becomes `{"error": message}`, and a null value is left out.
pub fn decode_results(steps: &[Step], results: &[Value]) -> Map<String, Value> {
    let mut values = Map::new();
    for (s, r) in steps.iter().zip(results) {
        match decode_one(&s.op, r) {
            Ok(Value::Null) => {}
            Ok(v) => {
                values.insert(s.id.clone(), v);
            }
            Err(e) => {
                values.insert(s.id.clone(), json!({ "error": e }));
            }
        }
    }
    values
}

/// One step's entry in the DLL's `results`.
fn decode_one(op: &Op, r: &Value) -> Result<Value, String> {
    if let Some(e) = r.get("error") {
        return Err(e["message"]
            .as_str()
            .map_or_else(|| e.to_string(), str::to_string));
    }
    let v = r.get("ok").ok_or_else(|| format!("no result in {r}"))?;
    match op {
        Op::MemRead { read_as, .. } => decode_read(v["hex"].as_str().unwrap_or_default(), *read_as),
        _ => lua_results_of(v).map(lua_value),
    }
}

/// A Lua step's value: `true` for no results, the one result, or all of
/// them as an array.
pub fn lua_value(r: Vec<String>) -> Value {
    match r.len() {
        0 => json!(true),
        1 => json!(r[0]),
        _ => json!(r),
    }
}

impl Supervisor {
    /// Run `steps` in one client frame through the DLL's `batch` method.
    /// `Ok(None)` means the step path should run instead: the DLL is too
    /// old (the flag is set so later batches skip this), or an address
    /// cannot be sent (the step path then reports it).
    pub(super) async fn batch_in_frame(
        &self,
        steps: &[Step],
    ) -> Result<Option<BatchOutcome>, String> {
        let t0 = Instant::now();
        let Ok(calls) = dll_calls(steps) else {
            return Ok(None);
        };
        let r = match self.bridge_call("batch", json!({ "calls": calls })).await {
            Ok(r) => r,
            Err(e) if e.contains("-32601") => {
                tracing::debug!(target: "lab.batch", "batch unsupported by this DLL; using the step path");
                self.dll_batch_unsupported.store(true, Ordering::Relaxed);
                return Ok(None);
            }
            Err(e) => {
                tracing::warn!(target: "lab.batch", error = %e, "DLL batch failed; using the step path");
                return Err(e);
            }
        };
        let results = r["results"].as_array().cloned().unwrap_or_default();
        if results.len() != steps.len() {
            return Err(format!(
                "batch: expected {} results, got {}",
                steps.len(),
                results.len()
            ));
        }
        Ok(Some(BatchOutcome {
            values: decode_results(steps, &results),
            stopped_at: None,
            ms: t0.elapsed().as_millis() as u64,
            frame: r["frame"].as_u64(),
        }))
    }
}

#[cfg(test)]
#[path = "batch_frame_tests.rs"]
mod tests;
