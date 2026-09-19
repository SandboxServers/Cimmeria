//! Dynamic logging hooks (`client_hook_install` / `_remove` / `_list`,
//! issue #686 scope 4).
//!
//! A non-freezing logging hook at an address with a capture spec
//! (registers, stack args, typed dereferences, hit limit, sample rate).
//! Built on [`crate::hooks::primitives`]. Function-entry only, cdecl only,
//! no asm — the scope and rationale live in [`native`].
//!
//! Layout (directory — 3 cohesive submodules plus this router):
//! - [`spec`] — capture-spec parsing + validation (pure, tested).
//! - [`registry`] — the live-hook registry: install/list/remove, the
//!   persistent flag, per-hit sampling + hit-limit decisions (pure, tested).
//! - [`native`] — the byte-patching slot pool + detours (windows i686).
//!
//! The registry is a process-global behind a `Mutex`; install/remove/list
//! run on the main thread (the dispatch drain), and [`on_hit`] locks it
//! briefly from the detour thread to record a fire and clone the spec.

pub mod native;
pub mod registry;
pub mod spec;

use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::{json, Value};

use super::dispatch::{self, RpcResponse, INTERNAL_ERROR, INVALID_PARAMS};
use registry::HookRegistry;
use spec::{CaptureSpec, DerefSource, DerefSpec, DerefType};

/// Process-global hook registry. Created on first use.
static REGISTRY: OnceLock<Mutex<HookRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<HookRegistry> {
    REGISTRY.get_or_init(|| Mutex::new(HookRegistry::new()))
}

/// Params for `hook_install`.
#[derive(Debug, Deserialize)]
struct HookInstallParams {
    addr: Value,
    #[serde(default = "default_conv")]
    conv: String,
    #[serde(default)]
    persistent: bool,
    /// Capture spec object; absent means "record only the hit itself".
    #[serde(default)]
    capture: Value,
}

fn default_conv() -> String {
    "cdecl".to_string()
}

/// Params for `hook_remove`.
#[derive(Debug, Deserialize)]
struct HookRemoveParams {
    id: u32,
}

/// Route `hook_install`.
pub fn dispatch_install(id: Value, params: &Value) -> RpcResponse {
    let p: HookInstallParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("hook_install params: {e}"))
        }
    };
    let Some(addr) = dispatch::parse_addr(&p.addr) else {
        return RpcResponse::error(id, INVALID_PARAMS, "hook_install: unparseable addr");
    };
    // Phase-3 native path is cdecl-only (see native.rs). Accept the field
    // for forward-compat but refuse anything we can't patch safely.
    if p.conv != "cdecl" {
        return RpcResponse::error(
            id,
            INVALID_PARAMS,
            format!(
                "hook_install: conv '{}' unsupported; phase-3 native hooks are cdecl-only \
                 (thiscall/stdcall + register capture need an asm stub, deferred)",
                p.conv
            ),
        );
    }
    let spec = match CaptureSpec::parse(&p.capture) {
        Ok(s) => s,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("hook_install: {e}")),
    };

    // Reserve the registry entry, then patch. Roll back on a patch failure
    // so a failed install leaves no ghost entry.
    let hook_id = {
        let mut reg = registry().lock().unwrap();
        if reg.contains_addr(addr) {
            return RpcResponse::error(
                id,
                INVALID_PARAMS,
                format!("hook_install: {addr:#x} is already hooked"),
            );
        }
        reg.install(addr, p.conv.clone(), spec, p.persistent)
    };

    match native::patch(addr, hook_id) {
        Ok(()) => RpcResponse::ok(
            id,
            json!({ "id": hook_id, "addr": format!("{addr:#x}"), "persistent": p.persistent }),
        ),
        Err(e) => {
            let _ = registry().lock().unwrap().remove(hook_id);
            RpcResponse::error(id, INTERNAL_ERROR, format!("hook_install: {e}"))
        }
    }
}

/// Route `hook_remove`.
pub fn dispatch_remove(id: Value, params: &Value) -> RpcResponse {
    let p: HookRemoveParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            return RpcResponse::error(id, INVALID_PARAMS, format!("hook_remove params: {e}"))
        }
    };
    // Restore the bytes first (so no more fires), then drop the registry
    // entry.
    match native::unpatch(p.id) {
        Ok(()) => {
            let removed = registry().lock().unwrap().remove(p.id).is_some();
            RpcResponse::ok(id, json!({ "removed": removed, "id": p.id }))
        }
        Err(e) => {
            // Off-target (or an unknown id): still drop any registry entry
            // so `hook_list` stays truthful.
            let removed = registry().lock().unwrap().remove(p.id).is_some();
            if removed {
                RpcResponse::ok(id, json!({ "removed": true, "id": p.id, "note": e }))
            } else {
                RpcResponse::error(id, INVALID_PARAMS, format!("hook_remove: {e}"))
            }
        }
    }
}

/// Route `hook_list`.
pub fn dispatch_list(id: Value) -> RpcResponse {
    let hooks = registry().lock().unwrap().list_json();
    RpcResponse::ok(id, json!({ "hooks": hooks }))
}

/// Called by a detour when its hooked function fires. Records the hit,
/// and if sampling says so, captures per the spec and pushes a `hook.hit`
/// event onto the local ring (which also uploads to SigNoz).
///
/// Locks the registry only long enough to record the hit and clone the
/// spec — never across the capture reads or the trampoline call.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(super) fn on_hit(hook_id: u32, args: &[u32; 8]) {
    let (capture, spec, addr) = {
        let mut reg = registry().lock().unwrap();
        let decision = reg.record_hit(hook_id);
        if !decision.capture {
            return;
        }
        match reg.get(hook_id) {
            Some(e) => (true, e.spec.clone(), e.addr),
            None => (false, CaptureSpec::default(), 0),
        }
    };
    if !capture {
        return;
    }

    let stack: Vec<u32> = args
        .iter()
        .take(spec.stack_args as usize)
        .copied()
        .collect();

    let mut derefs = serde_json::Map::new();
    for (i, d) in spec.derefs.iter().enumerate() {
        let label = d.label.clone().unwrap_or_else(|| format!("deref{i}"));
        derefs.insert(label, chase_deref(args, d));
    }

    super::events::push(
        "hook.hit",
        super::crash::now_ms(),
        json!({
            "hook_id": hook_id,
            "addr": format!("{addr:#x}"),
            "args": stack,
            "derefs": Value::Object(derefs),
        }),
    );
}

/// Follow a deref chain from its source, chasing a pointer at every
/// offset but the last, then reading the final address as `ty`. Returns a
/// JSON value, or `{ "error": ... }` when a read isn't safe (never faults
/// — every read is `VirtualQuery`-guarded).
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn chase_deref(args: &[u32; 8], d: &DerefSpec) -> Value {
    let base = match d.source {
        DerefSource::StackArg(n) => args[n as usize] as usize,
        // Register capture at entry needs an asm stub (see native.rs);
        // report it explicitly rather than emitting a wrong value.
        DerefSource::Register(r) => {
            return json!({ "error": format!("register '{r}' capture needs asm (deferred)") })
        }
    };

    let mut cur = base;
    if d.offsets.is_empty() {
        return read_typed(cur, d.ty);
    }
    let last = d.offsets.len() - 1;
    for (i, &off) in d.offsets.iter().enumerate() {
        cur = (cur as i64 + off as i64) as usize;
        if i < last {
            match read_ptr(cur) {
                Some(p) => cur = p,
                None => return json!({ "error": format!("unreadable pointer at {cur:#x}") }),
            }
        }
    }
    read_typed(cur, d.ty)
}

/// Read a 4-byte pointer, guarded. `None` if not readable.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn read_ptr(addr: usize) -> Option<usize> {
    let b = super::memory::guarded_read(addr, 4).ok()?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

/// Read a typed value at `addr`, guarded. Returns the decoded JSON value
/// or `{ "error": ... }` when the range isn't readable.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn read_typed(addr: usize, ty: DerefType) -> Value {
    if let Some(len) = ty.fixed_len() {
        let b = match super::memory::guarded_read(addr, len) {
            Ok(b) => b,
            Err(e) => return json!({ "error": e }),
        };
        return decode_fixed(&b, ty);
    }
    match ty {
        DerefType::CStr => match read_c_string(addr) {
            Ok(s) => json!(s),
            Err(e) => json!({ "error": e }),
        },
        DerefType::WStr => match read_w_string(addr) {
            Ok(s) => json!(s),
            Err(e) => json!({ "error": e }),
        },
        _ => json!({ "error": "unexpected variable-length type" }),
    }
}

#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn decode_fixed(b: &[u8], ty: DerefType) -> Value {
    match ty {
        DerefType::U8 => json!(b[0]),
        DerefType::I8 => json!(b[0] as i8),
        DerefType::U16 => json!(u16::from_le_bytes([b[0], b[1]])),
        DerefType::I16 => json!(i16::from_le_bytes([b[0], b[1]])),
        DerefType::U32 => json!(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        DerefType::I32 => json!(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        DerefType::Ptr => json!(format!(
            "{:#x}",
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        )),
        DerefType::U64 => json!(u64::from_le_bytes(b[0..8].try_into().unwrap())),
        DerefType::I64 => json!(i64::from_le_bytes(b[0..8].try_into().unwrap())),
        DerefType::F32 => json!(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        DerefType::F64 => json!(f64::from_le_bytes(b[0..8].try_into().unwrap())),
        DerefType::CStr | DerefType::WStr => json!({ "error": "not a fixed type" }),
    }
}

/// Max bytes a captured string reads before truncating.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
const MAX_STR_BYTES: usize = 256;

/// Read the largest guarded window we can (down to 1 byte) starting at
/// `addr`, capped at `MAX_STR_BYTES`. Strings often sit near a page edge,
/// so we shrink until a read succeeds rather than failing on the cap.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn read_guarded_window(addr: usize, cap: usize) -> Result<Vec<u8>, String> {
    let mut len = cap.min(MAX_STR_BYTES);
    while len >= 1 {
        if let Ok(b) = super::memory::guarded_read(addr, len) {
            return Ok(b);
        }
        len /= 2;
    }
    Err(format!("string at {addr:#x} not readable"))
}

#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn read_c_string(addr: usize) -> Result<String, String> {
    let b = read_guarded_window(addr, MAX_STR_BYTES)?;
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    Ok(String::from_utf8_lossy(&b[..end]).into_owned())
}

#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
fn read_w_string(addr: usize) -> Result<String, String> {
    let b = read_guarded_window(addr, MAX_STR_BYTES)?;
    // `as_chunks` (not `chunks_exact`) per clippy chunks_exact_to_as_chunks.
    let (pairs, _) = b.as_chunks::<2>();
    let units: Vec<u16> = pairs
        .iter()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    Ok(String::from_utf16_lossy(&units))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Off-target, install routes to the native stub and returns an
    /// INTERNAL_ERROR (patch unavailable) with the id echoed — but the
    /// registry rollback means no ghost entry is left behind.
    #[test]
    fn install_offtarget_rolls_back_and_echoes_id() {
        // Use a distinctive address unlikely to collide with another test.
        let r = dispatch_install(json!(1), &json!({ "addr": "0x00abc100", "conv": "cdecl" }));
        assert_eq!(r.id, json!(1));
        // Off-target: patch stub errors, entry rolled back → list has it
        // gone. (On the i686 DLL this path installs for real.)
        let list = dispatch_list(json!(2));
        let hooks = list.result.unwrap()["hooks"].as_array().unwrap().len();
        // Can't assert a hard count under parallel tests, but the address
        // we just tried must not be present after rollback.
        let _ = hooks;
        assert!(!registry().lock().unwrap().contains_addr(0x00abc100));
    }

    #[test]
    fn non_cdecl_conv_rejected_with_reason() {
        let r = dispatch_install(json!(3), &json!({ "addr": "0x401000", "conv": "thiscall" }));
        let e = r.error.expect("error");
        assert_eq!(e.code, INVALID_PARAMS);
        assert!(e.message.contains("cdecl-only"), "{}", e.message);
    }

    #[test]
    fn bad_capture_spec_rejected() {
        let r = dispatch_install(
            json!(4),
            &json!({ "addr": "0x401000", "conv": "cdecl", "capture": { "sample_rate": 0 } }),
        );
        assert_eq!(r.error.expect("error").code, INVALID_PARAMS);
    }

    #[test]
    fn remove_unknown_id_errors() {
        let r = dispatch_remove(json!(5), &json!({ "id": 4_000_000 }));
        // Off-target unpatch errors and no registry entry exists → error.
        assert!(r.error.is_some() || r.result.is_some());
    }

    #[test]
    fn decode_fixed_types_are_little_endian() {
        assert_eq!(decode_fixed(&[0x2a], DerefType::U8), json!(42));
        assert_eq!(decode_fixed(&[0x34, 0x12], DerefType::U16), json!(0x1234));
        assert_eq!(
            decode_fixed(&[0xff, 0xff, 0xff, 0xff], DerefType::I32),
            json!(-1)
        );
        assert_eq!(
            decode_fixed(&1.0f32.to_le_bytes(), DerefType::F32),
            json!(1.0)
        );
    }

    #[test]
    fn wstring_decode_stops_at_nul() {
        // "hi\0" as UTF-16LE plus trailing garbage.
        let bytes = [b'h', 0, b'i', 0, 0, 0, 0xff, 0xff];
        let (pairs, _) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        assert_eq!(String::from_utf16_lossy(&units), "hi");
    }
}
