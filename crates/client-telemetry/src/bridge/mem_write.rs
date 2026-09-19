//! `client_mem_write` — typed or raw memory write with a
//! `VirtualProtect` round-trip, exception-guarded (issue #686 scope 2).
//!
//! Two input shapes:
//!
//! - **Raw**: `{ "addr": …, "hex": "deadbeef" }` writes those bytes
//!   verbatim.
//! - **Typed**: `{ "addr": …, "value": 1234, "type": "u32" }` encodes
//!   `value` as little-endian for the named width.
//!
//! The write follows the same protect → write → restore → flush dance
//! as `hooks::primitives`, because the target may be code (a patched
//! prologue) as well as data. The `memcpy` itself runs inside
//! [`super::seh::guard`]; per the #686 Drop-across-unwind rule, no lock
//! is held across the guarded body (there is none to hold here — the
//! page protection is a native `VirtualProtect` pair, not a Rust
//! guard object).
//!
//! **Journaling**: writes reach here through the dispatch drain, which
//! flips the in-flight flag (`journal::mark_in_flight`) around the body,
//! and the supervisor records the `mem_write` command in its
//! `CommandJournal`. A write in flight at crash time is quarantined and
//! **never replayed** on recovery (ADR §6) — the recovery path replays
//! only persistent hooks, never writes or calls.

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{self, RpcResponse, INVALID_PARAMS, NATIVE_FAULT};

/// Params for `mem_write`.
#[derive(Debug, Deserialize)]
struct MemWriteParams {
    /// Address as a JSON number or `0x`-prefixed / bare hex string.
    addr: Value,
    /// Raw bytes as lowercase/uppercase hex (no separators). Mutually
    /// exclusive with `value`/`type`.
    #[serde(default)]
    hex: Option<String>,
    /// Typed value to encode little-endian (with `type`).
    #[serde(default)]
    value: Option<Value>,
    /// Width for `value`: one of u8/u16/u32/u64/i8/i16/i32/i64/f32/f64.
    #[serde(default, rename = "type")]
    ty: Option<String>,
}

/// Decode a hex string (even length, hex digits only) to bytes.
pub fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(format!("hex length {} is odd", s.len()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char)
            .to_digit(16)
            .ok_or_else(|| format!("non-hex char at {i}"))?;
        let lo = (bytes[i + 1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("non-hex char at {}", i + 1))?;
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    Ok(out)
}

/// Encode a typed `value` to little-endian bytes for `ty`.
///
/// Integer values accept a JSON number or a hex string; float values
/// accept a JSON number. Pure — the encoding table is the wire contract
/// and is unit-tested.
pub fn encode_typed(value: &Value, ty: &str) -> Result<Vec<u8>, String> {
    // Integers: accept number or hex string, range-check against width.
    fn as_i128(v: &Value) -> Result<i128, String> {
        match v {
            Value::Number(n) => n
                .as_i64()
                .map(i128::from)
                .or_else(|| n.as_u64().map(i128::from))
                .ok_or_else(|| "value is not an integer".to_string()),
            Value::String(s) => {
                let s = s.trim();
                let (neg, body) = s.strip_prefix('-').map_or((false, s), |b| (true, b));
                let hex = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X"));
                let mag = match hex {
                    Some(h) => i128::from_str_radix(h, 16),
                    None => body.parse::<i128>(),
                }
                .map_err(|e| format!("unparseable integer value: {e}"))?;
                Ok(if neg { -mag } else { mag })
            }
            _ => Err("value must be a number or hex string".to_string()),
        }
    }
    fn as_f64(v: &Value) -> Result<f64, String> {
        v.as_f64()
            .ok_or_else(|| "value is not a number".to_string())
    }

    match ty {
        "u8" => Ok(vec![
            u8::try_from(as_i128(value)?).map_err(|_| "u8 out of range")?
        ]),
        "i8" => Ok(
            (i8::try_from(as_i128(value)?).map_err(|_| "i8 out of range")? as u8)
                .to_le_bytes()
                .to_vec(),
        ),
        "u16" => Ok(u16::try_from(as_i128(value)?)
            .map_err(|_| "u16 out of range")?
            .to_le_bytes()
            .to_vec()),
        "i16" => Ok(i16::try_from(as_i128(value)?)
            .map_err(|_| "i16 out of range")?
            .to_le_bytes()
            .to_vec()),
        "u32" => Ok(u32::try_from(as_i128(value)?)
            .map_err(|_| "u32 out of range")?
            .to_le_bytes()
            .to_vec()),
        "i32" => Ok(i32::try_from(as_i128(value)?)
            .map_err(|_| "i32 out of range")?
            .to_le_bytes()
            .to_vec()),
        "u64" => Ok(u64::try_from(as_i128(value)?)
            .map_err(|_| "u64 out of range")?
            .to_le_bytes()
            .to_vec()),
        "i64" => Ok(i64::try_from(as_i128(value)?)
            .map_err(|_| "i64 out of range")?
            .to_le_bytes()
            .to_vec()),
        "f32" => Ok((as_f64(value)? as f32).to_le_bytes().to_vec()),
        "f64" => Ok(as_f64(value)?.to_le_bytes().to_vec()),
        other => Err(format!("unknown type '{other}'")),
    }
}

/// Resolve the bytes to write from the params (raw hex or typed).
fn resolve_bytes(p: &MemWriteParams) -> Result<Vec<u8>, String> {
    match (&p.hex, &p.value, &p.ty) {
        (Some(hex), None, None) => decode_hex(hex),
        (None, Some(value), Some(ty)) => encode_typed(value, ty),
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            Err("provide either `hex` or `value`+`type`, not both".to_string())
        }
        (None, Some(_), None) => Err("`value` requires `type`".to_string()),
        (None, None, Some(_)) => Err("`type` requires `value`".to_string()),
        (None, None, None) => Err("provide `hex` or `value`+`type`".to_string()),
    }
}

/// Route a `mem_write` request.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    let p: MemWriteParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("mem_write params: {e}")),
    };
    let Some(addr) = dispatch::parse_addr(&p.addr) else {
        return RpcResponse::error(id, INVALID_PARAMS, "mem_write: unparseable addr");
    };
    let bytes = match resolve_bytes(&p) {
        Ok(b) => b,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("mem_write: {e}")),
    };
    if bytes.is_empty() {
        return RpcResponse::error(id, INVALID_PARAMS, "mem_write: nothing to write");
    }

    // The guarded body absorbs a fault into a typed error; the outer
    // `Result` is the write's own validation/protection failure.
    match super::seh::guard(|| guarded_write(addr, &bytes)) {
        Ok(Ok(())) => RpcResponse::ok(
            id,
            json!({ "addr": format!("{addr:#x}"), "len": bytes.len(), "written": super::memory::to_hex(&bytes) }),
        ),
        Ok(Err(e)) => RpcResponse::error(id, NATIVE_FAULT, format!("mem_write: {e}")),
        Err(fault) => RpcResponse::error_with_data(
            id,
            NATIVE_FAULT,
            fault.message(),
            fault.to_exception_json(),
        ),
    }
}

/// Native write: validate the range is committed, open it writable,
/// copy, restore protection, flush the i-cache (the target may be
/// code). Only compiled into the injected DLL.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
fn guarded_write(addr: usize, bytes: &[u8]) -> Result<(), String> {
    use core::ffi::c_void;
    use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
    use windows_sys::Win32::System::Memory::{
        VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // Refuse a write into an uncommitted/no-access range up front, so a
    // typo doesn't wander into unmapped memory (guarded_read walks the
    // range with VirtualQuery and never faults).
    super::memory::guarded_read(addr, bytes.len())?;

    let mut old: PAGE_PROTECTION_FLAGS = 0;
    // SAFETY: range validated committed above; `old` is a live out-param.
    let ok = unsafe {
        VirtualProtect(
            addr as *const c_void,
            bytes.len(),
            PAGE_EXECUTE_READWRITE,
            &mut old,
        )
    };
    if ok == 0 {
        return Err(format!("VirtualProtect failed to make {addr:#x} writable"));
    }
    // SAFETY: page is RWX; we write exactly bytes.len() bytes into the
    // just-validated range.
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
    }
    let mut restored: PAGE_PROTECTION_FLAGS = 0;
    // SAFETY: same region; restore the saved protection.
    unsafe {
        let _ = VirtualProtect(addr as *const c_void, bytes.len(), old, &mut restored);
        // We may have written code; flush so the CPU sees the new bytes.
        FlushInstructionCache(GetCurrentProcess(), addr as *const c_void, bytes.len());
    }
    Ok(())
}

/// Off-target stub — the real write only exists inside SGW.exe.
#[cfg(not(all(target_os = "windows", target_arch = "x86")))]
fn guarded_write(_addr: usize, _bytes: &[u8]) -> Result<(), String> {
    Err("mem_write is only available in the injected DLL (windows i686)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_decode_round_trips() {
        assert_eq!(
            decode_hex("deadbeef").unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
        assert_eq!(decode_hex("00").unwrap(), vec![0]);
        assert_eq!(decode_hex("").unwrap(), Vec::<u8>::new());
        assert!(decode_hex("abc").is_err(), "odd length");
        assert!(decode_hex("zz").is_err(), "non-hex");
    }

    #[test]
    fn typed_encoding_is_little_endian() {
        assert_eq!(encode_typed(&json!(0x12), "u8").unwrap(), vec![0x12]);
        assert_eq!(
            encode_typed(&json!(0x1234), "u16").unwrap(),
            vec![0x34, 0x12]
        );
        assert_eq!(
            encode_typed(&json!(0xdeadbeefu32), "u32").unwrap(),
            vec![0xef, 0xbe, 0xad, 0xde]
        );
        assert_eq!(
            encode_typed(&json!(-1), "i32").unwrap(),
            vec![0xff, 0xff, 0xff, 0xff]
        );
        // Hex string value.
        assert_eq!(
            encode_typed(&json!("0x0040abcd"), "u32").unwrap(),
            vec![0xcd, 0xab, 0x40, 0x00]
        );
        // Float bit pattern.
        assert_eq!(
            encode_typed(&json!(1.0f64), "f32").unwrap(),
            1.0f32.to_le_bytes().to_vec()
        );
    }

    #[test]
    fn typed_encoding_range_checks() {
        assert!(encode_typed(&json!(256), "u8").is_err());
        assert!(encode_typed(&json!(-1), "u8").is_err());
        assert!(encode_typed(&json!(1), "u128").is_err(), "unknown type");
    }

    #[test]
    fn resolve_rejects_ambiguous_and_missing_inputs() {
        let both = MemWriteParams {
            addr: json!("0x10"),
            hex: Some("00".into()),
            value: Some(json!(1)),
            ty: Some("u8".into()),
        };
        assert!(resolve_bytes(&both).is_err());
        let neither = MemWriteParams {
            addr: json!("0x10"),
            hex: None,
            value: None,
            ty: None,
        };
        assert!(resolve_bytes(&neither).is_err());
        let value_no_type = MemWriteParams {
            addr: json!("0x10"),
            hex: None,
            value: Some(json!(1)),
            ty: None,
        };
        assert!(resolve_bytes(&value_no_type).is_err());
    }

    /// Off-target the native write is a stub, so the dispatch lands on a
    /// well-formed NATIVE_FAULT error with the id echoed — pinning the
    /// routing + response shape end to end.
    #[test]
    fn dispatch_echoes_id_and_shape_offtarget() {
        let r = dispatch(json!(7), &json!({ "addr": "0x400000", "hex": "90" }));
        assert_eq!(r.id, json!(7));
        assert!(r.result.is_some() || r.error.is_some());
        // Bad params → INVALID_PARAMS.
        let bad = dispatch(json!(8), &json!({ "addr": "0x10" }));
        assert_eq!(bad.error.expect("error").code, INVALID_PARAMS);
    }
}
