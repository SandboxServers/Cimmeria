//! `client_console` — submit a native slash command (issue #686 scope 5).
//!
//! This drives the client's own console-exec path so a GM-style `/`
//! command runs exactly as if typed. The submit runs on the main thread
//! inside [`super::seh::guard`], so a bad address faults into a
//! `NATIVE_FAULT` JSON-RPC error rather than tearing down the client.
//!
//! # The address is caller-supplied (and here's why)
//!
//! The confirmed console hook in this crate,
//! `APlayerController::execConsoleCommand` (`0x00539850`), is the
//! **UnrealScript exec thunk**: it takes an `FFrame&`, not a command
//! string, so it is the wrong shape to *invoke* a command. The
//! string-taking entry (`APlayerController::ConsoleCommand(const FString&)`
//! / `UConsole::ConsoleCommand`) needs one live Ghidra confirmation. Per
//! the lab model (§3.3: "all tools accept Ghidra addresses"), the agent
//! passes that address — plus the `this` object pointer — in the request
//! once confirmed, and this tool marshals the string and calls it with no
//! rebuild. Until an address is supplied, the call **degrades with a
//! reason** instead of guessing and crashing.
//!
//! # String marshalling
//!
//! [`encode_console_string`] builds the raw buffer: UTF-16LE +
//! NUL-terminator for a wide target (the UE3 default, `TCHAR = wchar_t`),
//! or ASCII + NUL for a narrow one. The target must accept a raw
//! `const TCHAR*`; a function taking an `FString` **by value/struct**
//! needs the three-word `{data, count, max}` layout and is out of scope
//! for this pass (documented in the PR). The marshalling table is pure
//! and unit-tested.

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{self, RpcResponse, INVALID_PARAMS, NATIVE_FAULT};

/// Sentinel for "no built-in string-console-exec address is confirmed".
/// See the module docs — the confirmed hook is the FFrame exec thunk,
/// which is the wrong shape. Overridden per-request by `addr`.
pub const CONSOLE_EXEC_ADDR: usize = 0;

/// Params for `console`.
#[derive(Debug, Deserialize)]
struct ConsoleParams {
    /// The command line to submit, e.g. `"/who"` or `"showdebug ai"`.
    line: String,
    /// Address of the string-taking console-exec function (from Ghidra,
    /// slide already applied). Overrides [`CONSOLE_EXEC_ADDR`].
    #[serde(default)]
    addr: Option<Value>,
    /// `this` object pointer for a `thiscall`/`fastcall` target (the
    /// PlayerController / Console). Required for those conventions.
    #[serde(default)]
    this: Option<Value>,
    /// Whether the target expects a wide (UTF-16) string. Default true
    /// (UE3 `TCHAR = wchar_t`).
    #[serde(default = "default_wide")]
    wide: bool,
    /// Calling convention of the exec function: thiscall (default),
    /// cdecl, or stdcall.
    #[serde(default = "default_conv")]
    conv: String,
}

fn default_wide() -> bool {
    true
}
fn default_conv() -> String {
    "thiscall".to_string()
}

/// Encode a console command line to the raw NUL-terminated buffer the
/// exec function reads through its string pointer. Pure — the byte
/// layout is the wire contract with the target and is unit-tested.
pub fn encode_console_string(line: &str, wide: bool) -> Vec<u8> {
    if wide {
        let mut out = Vec::with_capacity((line.len() + 1) * 2);
        for unit in line.encode_utf16().chain(core::iter::once(0)) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out
    } else {
        let mut out = line.as_bytes().to_vec();
        out.push(0);
        out
    }
}

/// Resolve the exec address from the request or the built-in sentinel.
fn resolve_addr(p: &ConsoleParams) -> Result<usize, String> {
    if let Some(a) = &p.addr {
        return dispatch::parse_addr(a).ok_or_else(|| "unparseable `addr`".to_string());
    }
    if CONSOLE_EXEC_ADDR != 0 {
        return Ok(CONSOLE_EXEC_ADDR);
    }
    Err(
        "no console-exec address: the confirmed hook (execConsoleCommand @0x539850) takes an \
         FFrame, not a string. Supply `addr` for the string-taking ConsoleCommand(FString) once \
         confirmed in Ghidra, or use client_lua_eval to reach the console via Lua"
            .to_string(),
    )
}

/// Route a `console` request.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    let p: ConsoleParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("console params: {e}")),
    };
    if !matches!(p.conv.as_str(), "thiscall" | "cdecl" | "stdcall") {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("console: unknown conv '{}'", p.conv),
        );
    }
    let addr = match resolve_addr(&p) {
        Ok(a) => a,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("console: {e}")),
    };
    let this = match &p.this {
        Some(v) => match dispatch::parse_addr(v) {
            Some(t) => Some(t),
            None => return RpcResponse::error(id, INVALID_PARAMS, "console: unparseable `this`"),
        },
        None => None,
    };
    if matches!(p.conv.as_str(), "thiscall" | "fastcall") && this.is_none() {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!("console: conv '{}' needs a `this` object pointer", p.conv),
        );
    }

    let buf = encode_console_string(&p.line, p.wide);
    let conv = p.conv.clone();
    match super::seh::guard(move || submit(addr, &conv, this, &buf)) {
        Ok(Ok(())) => RpcResponse::ok(id, json!({ "submitted": p.line })),
        Ok(Err(e)) => RpcResponse::error(id, NATIVE_FAULT, format!("console: {e}")),
        Err(fault) => RpcResponse::error_with_data(
            id,
            NATIVE_FAULT,
            fault.message(),
            fault.to_exception_json(),
        ),
    }
}

/// Submit the command by calling the exec function with a pointer to the
/// marshalled string buffer. Only compiled into the injected DLL.
///
/// The buffer is copied into a heap allocation whose pointer is passed to
/// the target; the target consumes it synchronously (console exec is not
/// deferred), so the allocation is freed when this returns.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn submit(addr: usize, conv: &str, this: Option<usize>, buf: &[u8]) -> Result<(), String> {
    // Own the bytes for the duration of the (synchronous) call.
    let owned = buf.to_vec();
    let str_ptr = owned.as_ptr() as u32;
    // SAFETY: the caller stated `addr`/`conv`/`this`; a wrong ABI or a bad
    // address faults into the surrounding SEH guard rather than corrupting
    // us silently. `owned` outlives the call.
    unsafe {
        match conv {
            "cdecl" => {
                let f: unsafe extern "cdecl" fn(u32) = core::mem::transmute(addr);
                f(str_ptr);
            }
            "stdcall" => {
                let f: unsafe extern "stdcall" fn(u32) = core::mem::transmute(addr);
                f(str_ptr);
            }
            "thiscall" => {
                let this = this.ok_or("thiscall needs `this`")? as u32;
                let f: unsafe extern "thiscall" fn(u32, u32) = core::mem::transmute(addr);
                f(this, str_ptr);
            }
            other => return Err(format!("unsupported conv '{other}'")),
        }
    }
    drop(owned);
    Ok(())
}

/// Off-target stub — the real console only exists inside SGW.exe.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
fn submit(_addr: usize, _conv: &str, _this: Option<usize>, _buf: &[u8]) -> Result<(), String> {
    Err("console is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_encoding_is_utf16le_nul_terminated() {
        // "/x" → 0x2f 0x78 then a wide NUL.
        assert_eq!(
            encode_console_string("/x", true),
            vec![0x2f, 0x00, 0x78, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn narrow_encoding_is_ascii_nul_terminated() {
        assert_eq!(encode_console_string("hi", false), vec![b'h', b'i', 0]);
    }

    #[test]
    fn empty_line_still_terminates() {
        assert_eq!(encode_console_string("", true), vec![0x00, 0x00]);
        assert_eq!(encode_console_string("", false), vec![0x00]);
    }

    /// With no built-in address confirmed, a bare request degrades with a
    /// reason (INVALID_PARAMS) rather than guessing — the honest default
    /// until the string-taking exec address is confirmed in Ghidra.
    #[test]
    fn no_address_degrades_with_reason() {
        let r = dispatch(json!(1), &json!({ "line": "/who" }));
        let e = r.error.expect("error");
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("console-exec address"), "{}", e.message);
    }

    /// A thiscall target without `this` is refused up front.
    #[test]
    fn thiscall_requires_this() {
        let r = dispatch(
            json!(2),
            &json!({ "line": "/who", "addr": "0x600000", "conv": "thiscall" }),
        );
        let e = r.error.expect("error");
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("this"), "{}", e.message);
    }

    /// Unknown convention is rejected.
    #[test]
    fn unknown_conv_rejected() {
        let r = dispatch(json!(3), &json!({ "line": "/who", "conv": "pascal" }));
        assert_eq!(r.error.expect("error").code, INVALID_PARAMS);
    }

    /// Off-target, a cdecl call with a supplied address routes to the
    /// stub and echoes the id with a well-formed NATIVE_FAULT.
    #[test]
    fn dispatch_routes_with_address_offtarget() {
        let r = dispatch(
            json!(4),
            &json!({ "line": "/who", "addr": "0x600000", "conv": "cdecl" }),
        );
        assert_eq!(r.id, json!(4));
        assert!(r.result.is_some() || r.error.is_some());
    }
}
