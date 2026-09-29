//! `client.lua.debug_log`: the UI's `Debug:log`, `Debug:warn` and
//! `Debug:error` lines (tooling backlog C10).
//!
//! `Debug` is not Lua: it is a tolua-bound native usertype,
//! `ScriptedDebug`, registered by the CEGUI script module's tolua open
//! function (`tolua_cclass(L, "Debug", "ScriptedDebug", ...)` at
//! `0x00ad46e5`, then `tolua_function(L, "log" | "warn" | "error", fn)` at
//! `0x00ad4703`, `0x00ad4713` and `0x00ad4723`). Each binding checks
//! `self` is a `ScriptedDebug` and argument 2 is a string, builds a
//! `std::wstring`, and calls **`0x0081c2e0`, which is a bare `ret`**. The
//! shipping client compiled the logger out: every `Debug:log` line the
//! stock UI and the Black Market overlay (`[Cimmeria BM] ...`) write is
//! discarded, and no log file ever held one (Ghidra and the image bytes
//! of the QA `SGW.exe`, 2026-09-29).
//!
//! The hooks sit on the three bindings (`lua_CFunction`s, `cdecl`,
//! `int(lua_State*)`, plain `ret`). Before chaining, the detour reads
//! argument 2 with `lua_type` + `lua_tolstring` only when it is a string
//! (the reader in [`crate::hooks::lua_stack`]), so nothing is converted,
//! allocated or raised; the binding then runs unchanged. All three are in
//! the fingerprint gate.
//!
//! Volume: throttled per (channel, message shape): burst 8, 4 a second,
//! with `suppressed` on the next line through. Levels: `error` and `warn`
//! for those channels; `log` lines are `info` when they carry the
//! overlay's `[Cimmeria` tag (rare and diagnostic) and `debug` otherwise
//! (the stock UI logs every window change, `back| ...`).

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::json;

use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Longest line read, in characters.
pub(crate) const MAX_TEXT_CHARS: usize = 512;

/// The tag every Cimmeria overlay line starts with (`[Cimmeria BM] `).
pub(crate) const CIMMERIA_TAG: &str = "[Cimmeria";

/// Which `Debug` method the script called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Channel {
    /// `Debug:log`.
    Log,
    /// `Debug:warn`.
    Warn,
    /// `Debug:error`.
    Error,
}

impl Channel {
    /// The `channel` field value.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Channel::Log => "log",
            Channel::Warn => "warn",
            Channel::Error => "error",
        }
    }
}

/// `cimmeria_bm` for the Black Market overlay's lines, `cimmeria` for any
/// other Cimmeria-tagged line, `ui` for the stock UI.
pub(crate) fn source_of(text: &str) -> &'static str {
    if text.starts_with("[Cimmeria BM]") {
        "cimmeria_bm"
    } else if text.starts_with(CIMMERIA_TAG) {
        "cimmeria"
    } else {
        "ui"
    }
}

/// The telemetry level for one line.
pub(crate) fn level_of(channel: Channel, text: &str) -> &'static str {
    match channel {
        Channel::Error => "error",
        Channel::Warn => "warn",
        Channel::Log if text.starts_with(CIMMERIA_TAG) => "info",
        Channel::Log => "debug",
    }
}

/// Throttle key: the channel and the message's shape (digits collapsed),
/// so a line that differs only by a number shares a bucket and two
/// different lines never hide each other.
pub(crate) fn throttle_key(channel: Channel, text: &str) -> String {
    format!(
        "{}:{}",
        channel.name(),
        crate::hooks::sinks::text::message_shape(text)
    )
}

/// The fields of one event.
pub(crate) fn line_fields(
    channel: Channel,
    text: &str,
    truncated: bool,
    suppressed: u64,
) -> Vec<(&'static str, serde_json::Value)> {
    let mut f = vec![
        ("channel", json!(channel.name())),
        ("source", json!(source_of(text))),
        ("text", json!(text)),
    ];
    if truncated {
        f.push(("truncated", json!(true)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

static THROTTLE: Mutex<Option<NameThrottle>> = Mutex::new(None);
static EPOCH: OnceLock<Instant> = OnceLock::new();

fn throttle(key: &str) -> Decision {
    let now_ms = EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let mut g = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_with(NameThrottle::new).check(key, now_ms)
}

/// The event for one line, or `None` when throttled.
pub(crate) fn report_with(
    channel: Channel,
    text: &str,
    truncated: bool,
) -> Option<(&'static str, Vec<(&'static str, serde_json::Value)>)> {
    let Decision::Emit { suppressed } = throttle(&throttle_key(channel, text)) else {
        return None;
    };
    Some((
        level_of(channel, text),
        line_fields(channel, text, truncated, suppressed),
    ))
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    use super::*;
    use crate::queue::Producer;
    use std::ffi::c_void;

    /// `ScriptedDebug::log` tolua binding (`Debug:log`).
    pub(in crate::hooks::inline_hooks) const ADDR_DEBUG_LOG: usize = 0x00aa_1620;
    /// `ScriptedDebug::warn` tolua binding (`Debug:warn`).
    pub(in crate::hooks::inline_hooks) const ADDR_DEBUG_WARN: usize = 0x00aa_1710;
    /// `ScriptedDebug::error` tolua binding (`Debug:error`).
    pub(in crate::hooks::inline_hooks) const ADDR_DEBUG_ERROR: usize = 0x00aa_1800;

    static LOG_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
    static WARN_TRAMPOLINE: OnceLock<usize> = OnceLock::new();
    static ERROR_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

    type LuaCFunction = unsafe extern "C-unwind" fn(*mut c_void) -> i32;

    pub(in crate::hooks::inline_hooks) unsafe fn install_all(producer: &Producer) {
        super::super::install_one(
            producer,
            "lua_debug_log",
            ADDR_DEBUG_LOG,
            debug_log_detour as *mut c_void,
            &LOG_TRAMPOLINE,
        );
        super::super::install_one(
            producer,
            "lua_debug_warn",
            ADDR_DEBUG_WARN,
            debug_warn_detour as *mut c_void,
            &WARN_TRAMPOLINE,
        );
        super::super::install_one(
            producer,
            "lua_debug_error",
            ADDR_DEBUG_ERROR,
            debug_error_detour as *mut c_void,
            &ERROR_TRAMPOLINE,
        );
    }

    /// Read argument 2 (the text; argument 1 is `self`) and report it.
    fn capture(l: *mut c_void, channel: Channel) {
        let Some((text, truncated)) = crate::hooks::lua_stack::read_string(l, 2, MAX_TEXT_CHARS)
        else {
            return;
        };
        if let Some((level, fields)) = report_with(channel, &text, truncated) {
            crate::hooks::emit::emit("client.lua.debug_log", level, fields);
        }
    }

    /// Capture, then run the binding. A missing trampoline returns 0 (no
    /// results), which is what the binding returns.
    unsafe fn forward(l: *mut c_void, channel: Channel, trampoline: &OnceLock<usize>) -> i32 {
        let _ = std::panic::catch_unwind(|| capture(l, channel));
        match trampoline.get() {
            Some(t) => {
                let original: LuaCFunction = unsafe { std::mem::transmute(*t) };
                unsafe { original(l) }
            }
            None => 0,
        }
    }

    unsafe extern "C-unwind" fn debug_log_detour(l: *mut c_void) -> i32 {
        unsafe { forward(l, Channel::Log, &LOG_TRAMPOLINE) }
    }

    unsafe extern "C-unwind" fn debug_warn_detour(l: *mut c_void) -> i32 {
        unsafe { forward(l, Channel::Warn, &WARN_TRAMPOLINE) }
    }

    unsafe extern "C-unwind" fn debug_error_detour(l: *mut c_void) -> i32 {
        unsafe { forward(l, Channel::Error, &ERROR_TRAMPOLINE) }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The detour hands `L` to the binding and returns its result, and a
        /// Lua error raised inside the binding (a C++ throw, a panic here)
        /// unwinds through the detour to the enclosing `lua_pcall`.
        #[test]
        fn forwards_the_state_and_lets_errors_through() {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static SEEN: AtomicUsize = AtomicUsize::new(0);
            unsafe extern "C-unwind" fn ok(l: *mut c_void) -> i32 {
                SEEN.store(l as usize, Ordering::SeqCst);
                0
            }
            unsafe extern "C-unwind" fn throws(_l: *mut c_void) -> i32 {
                panic!("tolua error");
            }
            let t_ok = OnceLock::new();
            t_ok.set(ok as *const () as usize).unwrap();
            // lua51.dll is not loaded in the test process, so the reader
            // returns nothing and the binding still runs.
            assert_eq!(
                unsafe { forward(0x4444 as *mut c_void, Channel::Log, &t_ok) },
                0
            );
            assert_eq!(SEEN.load(Ordering::SeqCst), 0x4444);
            let t_throw = OnceLock::new();
            t_throw.set(throws as *const () as usize).unwrap();
            let caught = std::panic::catch_unwind(|| unsafe {
                forward(0x4444 as *mut c_void, Channel::Warn, &t_throw)
            });
            assert!(caught.is_err());
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use native::install_all;
#[cfg(all(test, target_os = "windows", target_arch = "x86"))]
pub(super) use native::{ADDR_DEBUG_ERROR, ADDR_DEBUG_LOG, ADDR_DEBUG_WARN};

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &[(&'static str, serde_json::Value)], k: &str) -> Option<serde_json::Value> {
        f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn overlay_lines_are_tagged_and_raised_to_info() {
        let bm = "[Cimmeria BM] send search refused: offline";
        assert_eq!(source_of(bm), "cimmeria_bm");
        assert_eq!(level_of(Channel::Log, bm), "info");
        assert_eq!(source_of("[Cimmeria] x"), "cimmeria");
        assert_eq!(level_of(Channel::Log, "[Cimmeria] x"), "info");
        assert_eq!(source_of("back| Inventory"), "ui");
        assert_eq!(level_of(Channel::Log, "back| Inventory"), "debug");
        assert_eq!(level_of(Channel::Warn, "back| Inventory"), "warn");
        assert_eq!(level_of(Channel::Error, "anything"), "error");
    }

    #[test]
    fn fields_carry_channel_source_text_and_counts() {
        let f = line_fields(Channel::Warn, "[Cimmeria BM] onError id=99", true, 4);
        assert_eq!(get(&f, "channel"), Some(json!("warn")));
        assert_eq!(get(&f, "source"), Some(json!("cimmeria_bm")));
        assert_eq!(get(&f, "text"), Some(json!("[Cimmeria BM] onError id=99")));
        assert_eq!(get(&f, "truncated"), Some(json!(true)));
        assert_eq!(get(&f, "suppressed"), Some(json!(4)));
        let bare = line_fields(Channel::Log, "back| x", false, 0);
        assert!(!bare
            .iter()
            .any(|(k, _)| matches!(*k, "truncated" | "suppressed")));
    }

    /// Lines differing only by a number share a bucket; different words or
    /// a different channel do not.
    #[test]
    fn the_key_is_channel_and_shape() {
        assert_eq!(
            throttle_key(Channel::Log, "[Cimmeria BM] onError id=99"),
            throttle_key(Channel::Log, "[Cimmeria BM] onError id=12345")
        );
        assert_ne!(
            throttle_key(Channel::Log, "back| A"),
            throttle_key(Channel::Log, "back| B")
        );
        assert_ne!(
            throttle_key(Channel::Log, "same"),
            throttle_key(Channel::Warn, "same")
        );
    }

    /// The stock UI logs every window change; a burst is held to the
    /// bucket, and a rare overlay line still gets through during it.
    #[test]
    fn a_hot_line_is_throttled_without_hiding_a_rare_one() {
        let mut emitted = 0;
        for i in 0..300 {
            if report_with(Channel::Log, &format!("back| window {i}"), false).is_some() {
                emitted += 1;
            }
        }
        assert!((1..=12).contains(&emitted), "{emitted}");
        assert!(report_with(Channel::Log, "[Cimmeria BM] overlay loaded", false).is_some());
    }
}
