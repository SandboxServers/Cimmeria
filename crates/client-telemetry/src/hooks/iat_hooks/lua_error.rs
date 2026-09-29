//! `client.lua.error`: the error string of a failed `lua_pcall`.
//!
//! The UI scripts run under `lua_pcall`, and a failure there shows the
//! player nothing. When the (IAT-swapped) `lua_pcall` returns a non-zero
//! status, the error value is on top of the stack. The build is the wide
//! Lua 5.1 (`docs/reverse-engineering/findings/black-market-client-io.md`
//! §3): `lua_tolstring` returns a `wchar_t*` and a length in characters.
//!
//! The reader calls only `lua_type(L, -1)` and, if that is a string,
//! `lua_tolstring(L, -1, &len)`; `lua_tolstring` converts a number in place,
//! which is why the type is checked first. Both are resolved by their
//! mangled names from `lua51.dll` (verified against its export table on
//! 2026-09-28: `?lua_type@@YAHPAUlua_State@@H@Z` and
//! `?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z`). If either is missing the
//! event still goes out, without the message. The stack is read after the
//! call has returned and is left as it was.
//!
//! Throttled per message text: an error that repeats every frame is
//! reported once per burst with a `suppressed` count.

use std::sync::Mutex;

use serde_json::json;

use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Longest error message read, in characters.
pub(crate) const MAX_MESSAGE_CHARS: usize = 512;

/// Prefix of the message used as the throttle key.
const KEY_CHARS: usize = 160;

/// `LUA_TSTRING`.
pub(crate) const LUA_TSTRING: i32 = 4;

/// The status name for a non-zero `lua_pcall` result.
pub(crate) fn status_name(status: i32) -> &'static str {
    match status {
        2 => "runtime",
        3 => "syntax",
        4 => "memory",
        5 => "error_handler",
        _ => "unknown",
    }
}

/// The fields of one `client.lua.error`.
pub(crate) fn error_fields(
    status: i32,
    nargs: i32,
    message: Option<&str>,
    truncated: bool,
    suppressed: u64,
) -> Vec<(&'static str, serde_json::Value)> {
    let mut f = vec![
        ("status", json!(status)),
        ("status_name", json!(status_name(status))),
        ("nargs", json!(nargs)),
        ("message", json!(message)),
    ];
    if truncated {
        f.push(("truncated", json!(true)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The throttle key for a message: its first characters, so a message that
/// embeds a changing number or path suffix still groups.
pub(crate) fn throttle_key(message: Option<&str>) -> String {
    message
        .map(|m| m.chars().take(KEY_CHARS).collect())
        .unwrap_or_else(|| "<no message>".to_string())
}

static THROTTLE: Mutex<Option<NameThrottle>> = Mutex::new(None);
static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// Run the shared per-message throttle. Poisoning is ignored.
pub(crate) fn throttle(key: &str) -> Decision {
    let now_ms = EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64;
    let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(NameThrottle::new).check(key, now_ms)
}

/// Report a failed call. `read` fetches the message, which keys the
/// throttle, so it runs on every failure; the throttle bounds the events.
pub(crate) fn report_with(
    status: i32,
    nargs: i32,
    read: impl FnOnce() -> Option<(String, bool)>,
) -> Option<Vec<(&'static str, serde_json::Value)>> {
    let (message, truncated) = match read() {
        Some((m, t)) => (Some(m), t),
        None => (None, false),
    };
    let Decision::Emit { suppressed } = throttle(&throttle_key(message.as_deref())) else {
        return None;
    };
    Some(error_fields(
        status,
        nargs,
        message.as_deref(),
        truncated,
        suppressed,
    ))
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    const TYPE_SYMBOL: &core::ffi::CStr = c"?lua_type@@YAHPAUlua_State@@H@Z";
    const TOLSTRING_SYMBOL: &core::ffi::CStr = c"?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z";

    type LuaTypeFn = unsafe extern "C" fn(*mut c_void, i32) -> i32;
    type ToLStringFn = unsafe extern "C" fn(*mut c_void, i32, *mut u32) -> *const u16;

    /// The two `lua51.dll` exports the reader needs, resolved once.
    struct Api {
        lua_type: LuaTypeFn,
        tolstring: ToLStringFn,
    }

    static API: OnceLock<Option<Api>> = OnceLock::new();

    fn export(symbol: &core::ffi::CStr) -> Option<usize> {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        let wide: Vec<u16> = "lua51.dll".encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated strings that outlive the calls; no
        // reference to the module is kept.
        unsafe {
            let module = GetModuleHandleW(wide.as_ptr());
            if module.is_null() {
                return None;
            }
            GetProcAddress(module, symbol.as_ptr().cast()).map(|f| f as usize)
        }
    }

    fn api() -> Option<&'static Api> {
        API.get_or_init(|| {
            let t = export(TYPE_SYMBOL)?;
            let s = export(TOLSTRING_SYMBOL)?;
            // SAFETY: the exports' signatures, from the mangled names.
            Some(Api {
                lua_type: unsafe { std::mem::transmute::<usize, LuaTypeFn>(t) },
                tolstring: unsafe { std::mem::transmute::<usize, ToLStringFn>(s) },
            })
        })
        .as_ref()
    }

    /// The error string on top of `l`'s stack, if it is a string.
    fn read_message(l: *mut c_void) -> Option<(String, bool)> {
        let api = api()?;
        // SAFETY: `l` is the state the failed `lua_pcall` just returned
        // from, on this thread; index -1 is the error value.
        unsafe {
            if (api.lua_type)(l, -1) != LUA_TSTRING {
                return None;
            }
            let mut len: u32 = 0;
            let ptr = (api.tolstring)(l, -1, &mut len);
            if ptr.is_null() {
                return None;
            }
            let chars = (len as usize).min(MAX_MESSAGE_CHARS);
            let bytes = cimmeria_client_hookgate::os::read_bytes(ptr as usize, chars * 2)?;
            Some((
                crate::msvc_string::decode_bytes(&bytes, crate::msvc_string::Width::Wide),
                len as usize > MAX_MESSAGE_CHARS,
            ))
        }
    }

    /// Called by the `lua_pcall` detour after a non-zero return.
    pub(crate) fn report(l: *mut c_void, status: i32, nargs: i32) {
        if let Some(fields) = report_with(status, nargs, || read_message(l)) {
            crate::hooks::emit::emit("client.lua.error", "warn", fields);
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::report;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_names() {
        assert_eq!(status_name(2), "runtime");
        assert_eq!(status_name(3), "syntax");
        assert_eq!(status_name(4), "memory");
        assert_eq!(status_name(5), "error_handler");
        assert_eq!(status_name(9), "unknown");
    }

    #[test]
    fn fields_carry_message_status_and_counts() {
        let f = error_fields(2, 1, Some("[string \"x\"]:1: boom"), true, 3);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("status"), Some(json!(2)));
        assert_eq!(get("status_name"), Some(json!("runtime")));
        assert_eq!(get("nargs"), Some(json!(1)));
        assert_eq!(get("message"), Some(json!("[string \"x\"]:1: boom")));
        assert_eq!(get("truncated"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(3)));
        let bare = error_fields(2, 0, None, false, 0);
        assert!(bare.iter().any(|(k, v)| *k == "message" && v.is_null()));
        assert!(!bare
            .iter()
            .any(|(k, _)| *k == "truncated" || *k == "suppressed"));
    }

    /// A Lua error that repeats every frame is reported once per burst
    /// refill, with a `suppressed` count, not once per call.
    #[test]
    fn a_repeating_error_is_reported_once_per_burst() {
        let mut emitted = 0;
        let mut reads = 0;
        for _ in 0..200 {
            let r = report_with(2, 0, || {
                reads += 1;
                Some(("Lua.lua:7: attempt to index a nil value".to_string(), false))
            });
            if r.is_some() {
                emitted += 1;
            }
        }
        // The text is needed to key the throttle, so it is read every time;
        // the event count is what the throttle bounds.
        assert_eq!(reads, 200);
        assert!(emitted <= 12, "{emitted}");
        assert!(emitted >= 1);
    }

    /// A different message is a different bucket.
    #[test]
    fn distinct_messages_each_get_through() {
        for i in 0..5 {
            let r = report_with(2, 0, || Some((format!("distinct error {i}"), false)));
            assert!(r.is_some(), "{i}");
        }
    }

    #[test]
    fn the_key_is_a_bounded_prefix() {
        let long = "x".repeat(1000);
        assert_eq!(throttle_key(Some(&long)).chars().count(), KEY_CHARS);
        assert_eq!(throttle_key(None), "<no message>");
    }

    /// The message decodes from the wide characters Lua returns.
    #[test]
    fn wide_messages_decode() {
        let bytes: Vec<u8> = "bad argument #1"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        assert_eq!(
            crate::msvc_string::decode_bytes(&bytes, crate::msvc_string::Width::Wide),
            "bad argument #1"
        );
    }
}
