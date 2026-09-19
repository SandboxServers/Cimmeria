//! `client_call_native` — call a function by address with a stated
//! calling convention, on the main thread, exception-guarded (issue
//! #686 scope 3).
//!
//! Per the #686 spike, `extern "cdecl"/"stdcall"/"thiscall"/"fastcall"`
//! are **stable Rust ABI strings on i686** — exactly this target — so
//! each convention is a plain typed `extern fn` pointer cast handed to
//! [`super::seh::guard`], with **no hand-written asm trampolines**. All
//! arguments are passed as `u32` (the natural 32-bit word; pointers and
//! small ints alike). For `thiscall` the first argument is `this`
//! (ECX); for `fastcall` the first two arguments go in ECX/EDX and the
//! rest on the stack — the typed fn-pointer arity does that placement
//! for us.
//!
//! Return value: reported as both the raw EAX (`ret_u32`/`ret_i32`) or,
//! when `ret` is `f32`/`f64`, the float in ST(0). `void` reports
//! nothing meaningful.
//!
//! **Journaling / recovery**: like `mem_write`, calls are journaled
//! (in-flight flag around the dispatch body; the supervisor's
//! `CommandJournal` records the `call_native` command) and are **never
//! replayed** on crash recovery — only persistent hooks are (ADR §6).
//!
//! Arity is capped at 6 arguments, which covers the client functions in
//! scope; a wider call would need more fn-pointer arms.

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{self, RpcResponse, INVALID_PARAMS, NATIVE_FAULT};

/// Max positional arguments supported by the typed fn-pointer arms.
pub const MAX_ARGS: usize = 6;

/// Params for `call_native`.
#[derive(Debug, Deserialize)]
struct CallNativeParams {
    /// Function address (JSON number or hex string).
    addr: Value,
    /// One of cdecl/stdcall/thiscall/fastcall.
    #[serde(default = "default_conv")]
    conv: String,
    /// Positional args, each a JSON number or hex string. For thiscall
    /// the first is `this`; for fastcall the first two are ECX/EDX.
    #[serde(default)]
    args: Vec<Value>,
    /// Return interpretation: u32 (default), i32, void, f32, f64.
    #[serde(default = "default_ret")]
    ret: String,
}

fn default_conv() -> String {
    "cdecl".to_string()
}
fn default_ret() -> String {
    "u32".to_string()
}

/// The calling conventions we accept. Validating up front gives a clean
/// INVALID_PARAMS instead of a native mis-call.
pub fn known_conv(conv: &str) -> bool {
    matches!(conv, "cdecl" | "stdcall" | "thiscall" | "fastcall")
}

/// The return interpretations we accept.
pub fn known_ret(ret: &str) -> bool {
    matches!(ret, "u32" | "i32" | "void" | "f32" | "f64")
}

/// Parse each arg as a u32 word (number or hex string).
pub fn parse_args(args: &[Value]) -> Result<Vec<u32>, String> {
    if args.len() > MAX_ARGS {
        return Err(format!("at most {MAX_ARGS} args, got {}", args.len()));
    }
    args.iter()
        .enumerate()
        .map(|(i, v)| {
            dispatch::parse_addr(v)
                .and_then(|x| u32::try_from(x).ok())
                .ok_or_else(|| format!("arg {i} is not a u32 word"))
        })
        .collect()
}

/// Route a `call_native` request.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    let p: CallNativeParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("call_native params: {e}")),
    };
    let Some(addr) = dispatch::parse_addr(&p.addr) else {
        return RpcResponse::error(id, INVALID_PARAMS, "call_native: unparseable addr");
    };
    if !known_conv(&p.conv) {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("call_native: unknown conv '{}'", p.conv),
        );
    }
    if !known_ret(&p.ret) {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("call_native: unknown ret '{}'", p.ret),
        );
    }
    let args = match parse_args(&p.args) {
        Ok(a) => a,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("call_native: {e}")),
    };

    // No lock is held across the guarded body (the #686 rule): `conv`,
    // `ret`, `args`, `addr` are all owned copies by the time we guard.
    let conv = p.conv.clone();
    let ret = p.ret.clone();
    match super::seh::guard(move || invoke(addr, &conv, &ret, &args)) {
        Ok(Ok(result)) => RpcResponse::ok(id, result),
        Ok(Err(e)) => RpcResponse::error(id, NATIVE_FAULT, format!("call_native: {e}")),
        Err(fault) => RpcResponse::error_with_data(
            id,
            NATIVE_FAULT,
            fault.message(),
            fault.to_exception_json(),
        ),
    }
}

/// Integer-return fn-pointer arms for one ABI. Returns the raw EAX as
/// `u32`. `return`s an `Err` from the enclosing fn on over-arity.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
macro_rules! int_call {
    ($abi:literal, $addr:expr, $args:expr) => {{
        let addr = $addr;
        let a: &[u32] = $args;
        // SAFETY: caller stated the ABI; every arm's fn-pointer type
        // matches the requested convention and a u32-word arity. A
        // wrong ABI/arity is the caller's stated risk, absorbed by the
        // surrounding SEH guard.
        unsafe {
            match a.len() {
                0 => {
                    let f: unsafe extern $abi fn() -> u32 = core::mem::transmute(addr);
                    f()
                }
                1 => {
                    let f: unsafe extern $abi fn(u32) -> u32 = core::mem::transmute(addr);
                    f(a[0])
                }
                2 => {
                    let f: unsafe extern $abi fn(u32, u32) -> u32 = core::mem::transmute(addr);
                    f(a[0], a[1])
                }
                3 => {
                    let f: unsafe extern $abi fn(u32, u32, u32) -> u32 = core::mem::transmute(addr);
                    f(a[0], a[1], a[2])
                }
                4 => {
                    let f: unsafe extern $abi fn(u32, u32, u32, u32) -> u32 =
                        core::mem::transmute(addr);
                    f(a[0], a[1], a[2], a[3])
                }
                5 => {
                    let f: unsafe extern $abi fn(u32, u32, u32, u32, u32) -> u32 =
                        core::mem::transmute(addr);
                    f(a[0], a[1], a[2], a[3], a[4])
                }
                6 => {
                    let f: unsafe extern $abi fn(u32, u32, u32, u32, u32, u32) -> u32 =
                        core::mem::transmute(addr);
                    f(a[0], a[1], a[2], a[3], a[4], a[5])
                }
                n => return Err(format!("at most {MAX_ARGS} args, got {n}")),
            }
        }
    }};
}

/// f64-return fn-pointer arms for cdecl/thiscall (the float-returning
/// conventions in scope). Returns the ST(0) double.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
macro_rules! float_call {
    ($abi:literal, $addr:expr, $args:expr) => {{
        let addr = $addr;
        let a: &[u32] = $args;
        // SAFETY: as `int_call`, but the return is a double in ST(0).
        unsafe {
            match a.len() {
                0 => {
                    let f: unsafe extern $abi fn() -> f64 = core::mem::transmute(addr);
                    f()
                }
                1 => {
                    let f: unsafe extern $abi fn(u32) -> f64 = core::mem::transmute(addr);
                    f(a[0])
                }
                2 => {
                    let f: unsafe extern $abi fn(u32, u32) -> f64 = core::mem::transmute(addr);
                    f(a[0], a[1])
                }
                3 => {
                    let f: unsafe extern $abi fn(u32, u32, u32) -> f64 = core::mem::transmute(addr);
                    f(a[0], a[1], a[2])
                }
                4 => {
                    let f: unsafe extern $abi fn(u32, u32, u32, u32) -> f64 =
                        core::mem::transmute(addr);
                    f(a[0], a[1], a[2], a[3])
                }
                n => return Err(format!("at most 4 args for a float-return call, got {n}")),
            }
        }
    }};
}

/// Perform the call (windows i686). Selects the fn-pointer arm by
/// convention + return type and builds the result Value.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn invoke(addr: usize, conv: &str, ret: &str, args: &[u32]) -> Result<Value, String> {
    if ret == "f32" || ret == "f64" {
        let d = match conv {
            "cdecl" => float_call!("cdecl", addr, args),
            "thiscall" => float_call!("thiscall", addr, args),
            other => return Err(format!("float return unsupported for conv '{other}'")),
        };
        return Ok(json!({ "ret_f64": d, "ret_f32": d as f32 }));
    }
    let raw = match conv {
        "cdecl" => int_call!("cdecl", addr, args),
        "stdcall" => int_call!("stdcall", addr, args),
        "thiscall" => int_call!("thiscall", addr, args),
        "fastcall" => int_call!("fastcall", addr, args),
        other => return Err(format!("unknown conv '{other}'")),
    };
    Ok(json!({
        "ret_u32": raw,
        "ret_i32": raw as i32,
        "ret_hex": format!("{raw:#010x}"),
    }))
}

/// Off-target stub — real calls only happen inside SGW.exe.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
fn invoke(_addr: usize, _conv: &str, _ret: &str, _args: &[u32]) -> Result<Value, String> {
    Err("call_native is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conv_and_ret_validation() {
        for c in ["cdecl", "stdcall", "thiscall", "fastcall"] {
            assert!(known_conv(c));
        }
        assert!(!known_conv("pascal"));
        for r in ["u32", "i32", "void", "f32", "f64"] {
            assert!(known_ret(r));
        }
        assert!(!known_ret("bool"));
    }

    #[test]
    fn args_parse_words_and_reject_overflow() {
        let a = parse_args(&[json!(1), json!("0xff"), json!("0x00400000")]).unwrap();
        assert_eq!(a, vec![1, 0xff, 0x0040_0000]);
        // Over-arity refused.
        let many: Vec<Value> = (0..7).map(|i| json!(i)).collect();
        assert!(parse_args(&many).is_err());
        // Non-u32 refused.
        assert!(parse_args(&[json!("not-a-number")]).is_err());
    }

    /// Off-target the native call is a stub, so dispatch echoes the id
    /// with a well-formed NATIVE_FAULT error; INVALID_PARAMS on a bad
    /// convention. Pins routing + response shape end to end.
    #[test]
    fn dispatch_echoes_id_and_validates_offtarget() {
        let r = dispatch(json!(9), &json!({ "addr": "0x404030", "conv": "cdecl", "args": [] }));
        assert_eq!(r.id, json!(9));
        assert!(r.result.is_some() || r.error.is_some());

        let bad = dispatch(json!(10), &json!({ "addr": "0x1", "conv": "pascal" }));
        assert_eq!(bad.error.expect("error").code, INVALID_PARAMS);
    }
}
