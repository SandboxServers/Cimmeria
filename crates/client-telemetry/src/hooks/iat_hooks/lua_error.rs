//! `client.lua.error`: the error of a failed `lua_pcall`.
//!
//! The UI scripts run under `lua_pcall`, and a failure there shows the
//! player nothing. When the (IAT-swapped) `lua_pcall` returns a non-zero
//! status, the detour reads the top of the stack. The build is the wide
//! Lua 5.1 (`docs/reverse-engineering/findings/black-market-client-io.md`
//! §3), read through [`crate::hooks::lua_stack`].
//!
//! # Status -1: a foreign exception, which has no message
//!
//! `lua51.dll` is compiled as C++: `luaD_rawrunprotected` (`0x10008600`)
//! runs the call inside `try` with a single `catch (...)` funclet
//! (`Catch_All@1000864f`) that sets the status to **-1** when it is still
//! 0. A Lua `error()` sets its status (2, 3, 4 or 5) before it throws, so
//! -1 means an exception Lua did not raise: a C++ throw out of a C
//! function the script called (a CEGUI or tolua binding), or a structured
//! exception that reached the handler. `luaD_seterrorobj` (`0x100084c0`)
//! writes an error object only for 2, 3, 4 and 5; for -1 it only sets
//! `top = oldtop + 1`, so the top slot still holds the function that was
//! called. That is why 8 of 9 rows in the 2026-09-29 colo session arrived
//! as `status: -1, status_name: unknown, message: null`: the reader looked
//! for a string where Lua had put none (Ghidra on the QA `lua51.dll`,
//! 2026-09-29).
//!
//! So a -1 row is named `foreign_exception`, carries `value_type`
//! (`function`, as expected) and, when it can be read safely, the called
//! function's `function_source` (`short_src`) and `function_line`
//! (`linedefined`): which UI handler threw. The exception's own text, if
//! CEGUI logged it, is the `client.ui.cegui_log` error just before; an SEH
//! fault is `client.os.exception`.
//!
//! Throttled per message text (or, for a row without a message, per status
//! and function): an error that repeats every frame is reported once per
//! burst with a `suppressed` count.

use std::sync::Mutex;

use serde_json::json;

use crate::hooks::lua_stack;
use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Longest error message read, in characters.
pub(crate) const MAX_MESSAGE_CHARS: usize = 512;

/// Prefix of the message used as the throttle key.
const KEY_CHARS: usize = 160;

/// The status `luaD_rawrunprotected`'s `catch (...)` sets for an exception
/// Lua did not raise.
pub(crate) const LUA_FOREIGN_EXCEPTION: i32 = -1;

/// The status name for a non-zero `lua_pcall` result.
pub(crate) fn status_name(status: i32) -> &'static str {
    match status {
        LUA_FOREIGN_EXCEPTION => "foreign_exception",
        1 => "yield",
        2 => "runtime",
        3 => "syntax",
        4 => "memory",
        5 => "error_handler",
        _ => "unknown",
    }
}

/// What the reader found on top of the stack after a failed call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct TopOfStack {
    /// The error message and whether it was cut, if the value is a string.
    pub(crate) message: Option<(String, bool)>,
    /// The value's type tag (`lua_type`), if the API resolved.
    pub(crate) value_type: Option<i32>,
    /// `short_src` and `linedefined` of the function left on top (status
    /// -1 only).
    pub(crate) function: Option<(String, i32)>,
}

/// The fields of one `client.lua.error`.
pub(crate) fn error_fields(
    status: i32,
    nargs: i32,
    top: &TopOfStack,
    suppressed: u64,
) -> Vec<(&'static str, serde_json::Value)> {
    let message = top.message.as_ref().map(|(m, _)| m.as_str());
    let mut f = vec![
        ("status", json!(status)),
        ("status_name", json!(status_name(status))),
        ("nargs", json!(nargs)),
        ("message", json!(message)),
    ];
    if let Some(t) = top.value_type {
        f.push(("value_type", json!(lua_stack::type_name(t))));
    }
    if let Some((source, line)) = &top.function {
        f.push(("function_source", json!(source)));
        f.push(("function_line", json!(line)));
    }
    if top.message.as_ref().is_some_and(|(_, t)| *t) {
        f.push(("truncated", json!(true)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The throttle key: the message's first characters, so a message that
/// embeds a changing number or path suffix still groups; without a
/// message, the status and the function that was called, so two different
/// handlers throwing do not hide each other.
pub(crate) fn throttle_key(status: i32, top: &TopOfStack) -> String {
    match (&top.message, &top.function) {
        (Some((m, _)), _) => m.chars().take(KEY_CHARS).collect(),
        (None, Some((src, line))) => format!("<{}> {src}:{line}", status_name(status)),
        (None, None) => format!("<{}>", status_name(status)),
    }
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

/// Report a failed call. `read` fetches the top of the stack, which keys
/// the throttle, so it runs on every failure; the throttle bounds the
/// events.
pub(crate) fn report_with(
    status: i32,
    nargs: i32,
    read: impl FnOnce() -> TopOfStack,
) -> Option<Vec<(&'static str, serde_json::Value)>> {
    let top = read();
    let Decision::Emit { suppressed } = throttle(&throttle_key(status, &top)) else {
        return None;
    };
    Some(error_fields(status, nargs, &top, suppressed))
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use std::ffi::c_void;

    /// What is on top of `l`'s stack after a failed call: the message if
    /// it is a string, the value's type, and for a foreign exception the
    /// function that was called (the slot `luaD_seterrorobj` leaves there).
    fn read_top(l: *mut c_void, status: i32) -> TopOfStack {
        let value_type = lua_stack::value_type(l, -1);
        let message = lua_stack::read_string(l, -1, MAX_MESSAGE_CHARS);
        let function = if status == LUA_FOREIGN_EXCEPTION {
            lua_stack::top_function_info(l)
        } else {
            None
        };
        TopOfStack {
            message,
            value_type,
            function,
        }
    }

    /// Called by the `lua_pcall` detour after a non-zero return.
    pub(crate) fn report(l: *mut c_void, status: i32, nargs: i32) {
        if let Some(fields) = report_with(status, nargs, || read_top(l, status)) {
            crate::hooks::emit::emit("client.lua.error", "warn", fields);
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::report;

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &[(&'static str, serde_json::Value)], k: &str) -> Option<serde_json::Value> {
        f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
    }

    fn with_message(m: &str) -> TopOfStack {
        TopOfStack {
            message: Some((m.to_string(), false)),
            value_type: Some(lua_stack::LUA_TSTRING),
            function: None,
        }
    }

    /// -1 is the status `luaD_rawrunprotected`'s `catch (...)` sets; it was
    /// reported as `unknown`, which hid what it meant.
    #[test]
    fn status_names() {
        assert_eq!(status_name(-1), "foreign_exception");
        assert_eq!(status_name(1), "yield");
        assert_eq!(status_name(2), "runtime");
        assert_eq!(status_name(3), "syntax");
        assert_eq!(status_name(4), "memory");
        assert_eq!(status_name(5), "error_handler");
        assert_eq!(status_name(9), "unknown");
    }

    #[test]
    fn fields_carry_message_status_and_counts() {
        let top = TopOfStack {
            message: Some(("[string \"x\"]:1: boom".to_string(), true)),
            value_type: Some(lua_stack::LUA_TSTRING),
            function: None,
        };
        let f = error_fields(2, 1, &top, 3);
        assert_eq!(get(&f, "status"), Some(json!(2)));
        assert_eq!(get(&f, "status_name"), Some(json!("runtime")));
        assert_eq!(get(&f, "nargs"), Some(json!(1)));
        assert_eq!(get(&f, "message"), Some(json!("[string \"x\"]:1: boom")));
        assert_eq!(get(&f, "value_type"), Some(json!("string")));
        assert_eq!(get(&f, "truncated"), Some(json!(true)));
        assert_eq!(get(&f, "suppressed"), Some(json!(3)));
        assert_eq!(get(&f, "function_source"), None);
        let bare = error_fields(2, 0, &TopOfStack::default(), 0);
        assert!(get(&bare, "message").unwrap().is_null());
        assert!(!bare
            .iter()
            .any(|(k, _)| matches!(*k, "truncated" | "suppressed" | "value_type")));
    }

    /// The 2026-09-29 row shape: status -1 with the called function on top.
    /// It now names the status, says the top is a function (so a null
    /// message is expected, not a reader fault) and names the handler.
    #[test]
    fn a_foreign_exception_names_the_function_that_threw() {
        let top = TopOfStack {
            message: None,
            value_type: Some(lua_stack::LUA_TFUNCTION),
            function: Some(("[string \"BlackMarket.lua\"]".to_string(), 212)),
        };
        let f = error_fields(-1, 0, &top, 0);
        assert_eq!(get(&f, "status_name"), Some(json!("foreign_exception")));
        assert!(get(&f, "message").unwrap().is_null());
        assert_eq!(get(&f, "value_type"), Some(json!("function")));
        assert_eq!(
            get(&f, "function_source"),
            Some(json!("[string \"BlackMarket.lua\"]"))
        );
        assert_eq!(get(&f, "function_line"), Some(json!(212)));
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
                with_message("Lua.lua:7: attempt to index a nil value")
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
            let r = report_with(2, 0, || with_message(&format!("distinct error {i}")));
            assert!(r.is_some(), "{i}");
        }
    }

    /// Two handlers throwing foreign exceptions do not share a bucket, so a
    /// hot one cannot hide a rare one.
    #[test]
    fn foreign_exceptions_are_keyed_by_function() {
        let a = TopOfStack {
            function: Some(("a.lua".into(), 1)),
            ..Default::default()
        };
        let b = TopOfStack {
            function: Some(("b.lua".into(), 1)),
            ..Default::default()
        };
        assert_ne!(throttle_key(-1, &a), throttle_key(-1, &b));
        assert_eq!(throttle_key(-1, &a), "<foreign_exception> a.lua:1");
        assert_eq!(
            throttle_key(-1, &TopOfStack::default()),
            "<foreign_exception>"
        );
    }

    #[test]
    fn the_key_is_a_bounded_prefix() {
        let long = "x".repeat(1000);
        assert_eq!(
            throttle_key(2, &with_message(&long)).chars().count(),
            KEY_CHARS
        );
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
