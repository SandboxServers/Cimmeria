//! JSON-RPC 2.0 request/response types and the main-thread method
//! router.
//!
//! [`dispatch`] runs on the game main thread (the `FEngineLoop::Tick`
//! drain), so it is the only place allowed to touch the Lua VM and
//! process memory. The IO thread parses bytes into an [`RpcRequest`]
//! and queues it; it never calls `dispatch`.
//!
//! Phase-1 methods: `lua_eval`, `module_info`, `mem_read`.
//! Phase-3 methods (#686): `mem_write`, `call_native`, `hook_install`,
//! `hook_remove`, `hook_list`, `events_read`, `console`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{console, dynamic_hooks, events, lua_eval, mem_write, memory, native_call};

/// JSON-RPC parse error (malformed request body).
pub const PARSE_ERROR: i32 = -32700;
/// Method not found.
pub const METHOD_NOT_FOUND: i32 = -32601;
/// Invalid params for the method.
pub const INVALID_PARAMS: i32 = -32602;
/// Server-side error while running the method.
pub const INTERNAL_ERROR: i32 = -32603;
/// Bridge dispatch queue full — retry. Mirrors the Atrea bridge's
/// `-32010`.
pub const QUEUE_FULL: i32 = -32010;
/// A native dispatch body faulted and was absorbed by the inner-tier
/// SEH guard (#686 scope 1). The `data` field carries the `Exception`
/// (STATUS_* code + faulting address); the process survives.
pub const NATIVE_FAULT: i32 = -32020;

/// An inbound JSON-RPC 2.0 request. `id` is echoed verbatim; a
/// missing `id` (notification) round-trips as JSON `null`.
#[derive(Debug, Clone, Deserialize)]
pub struct RpcRequest {
    #[serde(default)]
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// A JSON-RPC 2.0 response. Exactly one of `result`/`error` is set.
#[derive(Debug, Clone, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    /// Optional structured payload. Carries the `Exception` object for a
    /// [`NATIVE_FAULT`] (STATUS_* code + faulting address) so the agent
    /// can correlate a crash-absorbed probe with a Ghidra address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    /// Error response carrying a structured `data` payload — used for a
    /// [`NATIVE_FAULT`] to attach the `Exception` object.
    pub fn error_with_data(id: Value, code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.into(),
                data: Some(data),
            }),
        }
    }

    /// Serialize to a frame body. Infallible in practice (the shape
    /// is fixed); a serialize failure degrades to a minimal error
    /// object so the caller always has something to write back.
    pub fn to_frame_body(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_else(|_| {
            br#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"response serialize failed"}}"#.to_vec()
        })
    }
}

/// Params for `mem_read`.
#[derive(Debug, Deserialize)]
struct MemReadParams {
    /// Address as a JSON number or a `0x`-prefixed / bare hex string.
    addr: Value,
    len: usize,
}

/// Params for `lua_eval`.
#[derive(Debug, Deserialize)]
struct LuaEvalParams {
    chunk: String,
}

/// Route one request to its handler and build the response.
///
/// **Runs on the main thread only.** `lua_eval`, `mem_read`, and
/// `module_info` all reach into the live process here.
pub fn dispatch(req: &RpcRequest) -> RpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "lua_eval" => match serde_json::from_value::<LuaEvalParams>(req.params.clone()) {
            Ok(p) => {
                // SAFETY: dispatch runs on the main thread (the Tick
                // drain), the only context where touching the VM is
                // sound.
                match unsafe { lua_eval::eval_on_main_thread(&p.chunk) } {
                    Ok(res) => {
                        RpcResponse::ok(id, serde_json::to_value(res).unwrap_or(Value::Null))
                    }
                    Err(e) => RpcResponse::error(id, INTERNAL_ERROR, e),
                }
            }
            Err(e) => RpcResponse::error(id, INVALID_PARAMS, format!("lua_eval params: {e}")),
        },
        "heartbeat" => {
            // The Tick-drain counter (this very dispatch runs inside the
            // drain, so the value is current for this frame). The
            // supervisor polls it and compares successive reads: no
            // advance ⇒ hung/crash-dialog ⇒ terminate (ADR §6).
            RpcResponse::ok(
                id,
                serde_json::json!({ "tick_count": super::heartbeat_count() }),
            )
        }
        "module_info" => match memory::module_info() {
            Ok(info) => RpcResponse::ok(
                id,
                serde_json::json!({
                    "image_base": format!("{:#x}", info.image_base),
                    "preferred_base": format!("{:#x}", info.preferred_base),
                    "slide": info.slide,
                }),
            ),
            Err(e) => RpcResponse::error(id, INTERNAL_ERROR, e),
        },
        "mem_read" => match serde_json::from_value::<MemReadParams>(req.params.clone()) {
            Ok(p) => match parse_addr(&p.addr) {
                Some(addr) => match memory::guarded_read(addr, p.len) {
                    Ok(bytes) => RpcResponse::ok(
                        id,
                        serde_json::json!({
                            "addr": format!("{addr:#x}"),
                            "len": bytes.len(),
                            "hex": memory::to_hex(&bytes),
                        }),
                    ),
                    Err(e) => RpcResponse::error(id, INTERNAL_ERROR, e),
                },
                None => RpcResponse::error(id, INVALID_PARAMS, "mem_read: unparseable addr"),
            },
            Err(e) => RpcResponse::error(id, INVALID_PARAMS, format!("mem_read params: {e}")),
        },
        // ── Phase 3 (#686): native probes ───────────────────────────
        "mem_write" => mem_write::dispatch(id, &req.params),
        "call_native" => native_call::dispatch(id, &req.params),
        "hook_install" => dynamic_hooks::dispatch_install(id, &req.params),
        "hook_remove" => dynamic_hooks::dispatch_remove(id, &req.params),
        "hook_list" => dynamic_hooks::dispatch_list(id),
        "events_read" => events::dispatch_read(id, &req.params),
        "console" => console::dispatch(id, &req.params),
        other => RpcResponse::error(id, METHOD_NOT_FOUND, format!("unknown method: {other}")),
    }
}

/// Accept an address as a JSON integer or a hex string (`"0x1234"`
/// or `"1234"`).
pub(crate) fn parse_addr(v: &Value) -> Option<usize> {
    match v {
        Value::Number(n) => n.as_u64().map(|x| x as usize),
        Value::String(s) => {
            let s = s.trim();
            let hex = s
                .strip_prefix("0x")
                .or_else(|| s.strip_prefix("0X"))
                .unwrap_or(s);
            usize::from_str_radix(hex, 16).ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(method: &str, id: i64, params: Value) -> RpcRequest {
        RpcRequest {
            id: Value::from(id),
            method: method.to_string(),
            params,
        }
    }

    #[test]
    fn unknown_method_yields_method_not_found() {
        let r = dispatch(&req("nope", 7, Value::Null));
        let e = r.error.expect("error");
        assert_eq!(e.code, METHOD_NOT_FOUND);
        assert_eq!(r.id, Value::from(7));
    }

    /// On the host (non-i686-Windows) the native primitives are
    /// stubs, so these three land on `INTERNAL_ERROR` with a
    /// well-formed JSON-RPC response and the id echoed. This still
    /// pins routing + response shape end to end off-process.
    #[test]
    fn native_methods_route_and_echo_id() {
        for m in ["module_info"] {
            let r = dispatch(&req(m, 3, Value::Null));
            assert_eq!(r.id, Value::from(3));
            assert!(r.result.is_some() || r.error.is_some());
        }
        let r = dispatch(&req(
            "mem_read",
            4,
            serde_json::json!({ "addr": "0x400000", "len": 8 }),
        ));
        assert_eq!(r.id, Value::from(4));
        let r = dispatch(&req(
            "lua_eval",
            5,
            serde_json::json!({ "chunk": "return 1" }),
        ));
        assert_eq!(r.id, Value::from(5));
    }

    #[test]
    fn bad_params_yield_invalid_params() {
        // mem_read missing `len`.
        let r = dispatch(&req("mem_read", 1, serde_json::json!({ "addr": "0x10" })));
        assert_eq!(r.error.expect("error").code, INVALID_PARAMS);
        // lua_eval missing `chunk`.
        let r = dispatch(&req("lua_eval", 2, serde_json::json!({})));
        assert_eq!(r.error.expect("error").code, INVALID_PARAMS);
    }

    #[test]
    fn addr_parsing() {
        assert_eq!(parse_addr(&Value::from(0x400000)), Some(0x400000));
        assert_eq!(parse_addr(&Value::from("0x416ec0")), Some(0x416ec0));
        assert_eq!(parse_addr(&Value::from("416ec0")), Some(0x416ec0));
        assert_eq!(parse_addr(&Value::from("0Xdeadbeef")), Some(0xdeadbeef));
        assert_eq!(parse_addr(&Value::from("zzz")), None);
        assert_eq!(parse_addr(&Value::Null), None);
    }

    #[test]
    fn response_serializes_without_null_result_and_error() {
        let ok = RpcResponse::ok(Value::from(1), serde_json::json!({"x": 1}));
        let s = String::from_utf8(ok.to_frame_body()).unwrap();
        assert!(s.contains("\"result\""));
        assert!(!s.contains("\"error\""));

        let err = RpcResponse::error(Value::from(1), QUEUE_FULL, "full");
        let s = String::from_utf8(err.to_frame_body()).unwrap();
        assert!(s.contains("\"error\""));
        assert!(!s.contains("\"result\""));
        assert!(s.contains("-32010"));
    }
}
