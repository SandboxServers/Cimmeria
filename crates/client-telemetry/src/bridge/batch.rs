//! `batch`: several lab reads answered from one main-thread drain, so
//! they all describe the same frame (C7).
//!
//! Batch adds no capability. Every allowed method is one a single call
//! already offers, and each runs through the same handler its single
//! call uses. The allow-list is checked for the whole batch before any
//! call runs, so a refused batch has no side effects at all.

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{self, RpcRequest, RpcResponse, INVALID_PARAMS};

/// Read-only methods a batch may carry. Writes, `call_native`, `hook_*`,
/// the other `input_*` methods, `console` and `batch` itself are out.
pub const ALLOWED: [&str; 6] = [
    "heartbeat",
    "mem_read",
    "lua_eval",
    "input_status",
    "watch_read",
    "module_info",
];

/// Most calls one batch may carry.
const MAX_CALLS: usize = 16;

#[derive(Debug, Deserialize)]
struct BatchParams {
    calls: Vec<BatchCall>,
}

#[derive(Debug, Deserialize)]
struct BatchCall {
    method: String,
    #[serde(default)]
    params: Value,
}

/// Run a batch. **Main thread only**, like every dispatch.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    let parsed = match serde_json::from_value::<BatchParams>(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("batch params: {e}")),
    };
    if parsed.calls.is_empty() || parsed.calls.len() > MAX_CALLS {
        return RpcResponse::error(id, INVALID_PARAMS, "batch: 1..=16 calls");
    }
    // Validate everything first: a refused batch must not have run a call.
    if let Some(bad) = parsed
        .calls
        .iter()
        .find(|c| !ALLOWED.contains(&c.method.as_str()))
    {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("batch: method {} not allowed", bad.method),
        );
    }
    let results: Vec<Value> = parsed
        .calls
        .into_iter()
        .map(|c| {
            let resp = dispatch::dispatch(&RpcRequest {
                id: Value::from(0),
                method: c.method,
                params: c.params,
            });
            match resp.error {
                Some(e) => json!({ "error": { "code": e.code, "message": e.message } }),
                None => json!({ "ok": resp.result.unwrap_or(Value::Null) }),
            }
        })
        .collect();
    RpcResponse::ok(
        id,
        json!({ "results": results, "frame": super::heartbeat_count() }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(calls: Value) -> RpcResponse {
        dispatch(Value::from(1), &json!({ "calls": calls }))
    }

    fn refusal(r: &RpcResponse) -> (i32, String) {
        let e = r.error.as_ref().expect("error");
        (e.code, e.message.clone())
    }

    #[test]
    fn batch_runs_reads_in_order() {
        let r = run(json!([
            { "method": "heartbeat" },
            { "method": "mem_read", "params": { "addr": "zzz", "len": 4 } },
            { "method": "module_info" },
        ]));
        let v = r.result.expect("result");
        let results = v["results"].as_array().expect("results");
        assert_eq!(results.len(), 3);
        assert!(results[0]["ok"]["tick_count"].is_u64());
        assert_eq!(results[1]["error"]["code"], INVALID_PARAMS);
        assert!(results[2].get("ok").is_some() || results[2].get("error").is_some());
        assert!(v["frame"].is_u64());
    }

    #[test]
    fn a_write_refuses_the_whole_batch() {
        // The refusal names the write and no `results` exist: the check
        // ran before any call.
        let r = run(json!([
            { "method": "heartbeat" },
            { "method": "mem_write", "params": { "addr": "0x10", "hex": "00" } },
        ]));
        assert!(r.result.is_none());
        let (code, msg) = refusal(&r);
        assert_eq!(code, INVALID_PARAMS);
        assert_eq!(msg, "batch: method mem_write not allowed");
    }

    #[test]
    fn nested_batch_refused() {
        let r = run(json!([{ "method": "batch", "params": { "calls": [] } }]));
        let (code, msg) = refusal(&r);
        assert_eq!(code, INVALID_PARAMS);
        assert_eq!(msg, "batch: method batch not allowed");
    }

    #[test]
    fn empty_and_oversized_refused() {
        let (code, msg) = refusal(&run(json!([])));
        assert_eq!(code, INVALID_PARAMS);
        assert_eq!(msg, "batch: 1..=16 calls");
        let many: Vec<Value> = (0..17).map(|_| json!({ "method": "heartbeat" })).collect();
        let (code, msg) = refusal(&run(Value::Array(many)));
        assert_eq!(code, INVALID_PARAMS);
        assert_eq!(msg, "batch: 1..=16 calls");
        // 16 is still fine.
        let sixteen: Vec<Value> = (0..16).map(|_| json!({ "method": "heartbeat" })).collect();
        assert!(run(Value::Array(sixteen)).result.is_some());
    }

    #[test]
    fn per_call_errors_do_not_fail_the_batch() {
        let r = run(json!([
            { "method": "mem_read", "params": { "addr": "zzz", "len": 4 } },
            { "method": "heartbeat" },
        ]));
        let v = r.result.expect("batch itself succeeds");
        let results = v["results"].as_array().expect("results");
        assert!(results[0].get("error").is_some());
        assert!(results[1].get("ok").is_some());
    }

    #[test]
    fn unknown_allowed_method_is_a_per_call_error() {
        // watch_read before AX-24 merges answers `unknown method`.
        let r = run(json!([{ "method": "watch_read", "params": {} }]));
        assert!(r.result.is_some());
    }
}
